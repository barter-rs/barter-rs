use std::marker::PhantomData;

use super::http::{
    parser::BybitParser,
    requests::{
        CancelOrderBody, CancelOrderRequest, GetOpenAndClosedOrders, GetOpenAndClosedOrdersParams,
        GetOrderTradesParams, GetOrderTradesRequest, GetWalletBalanceParams,
        GetWalletBalanceRequest, GetWalletBalanceResponseInner,
    },
    signer::{BybitRequestSigner, BybitSigner},
};
use super::servers::BybitServer;
use super::types::AccountType;
use super::websocket::{extract_event, subscribe, BybitPayload};
use barter_instrument::{
    asset::{name::AssetNameExchange, QuoteAsset},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use barter_integration::{
    channel::{mpsc_unbounded, Tx},
    protocol::{
        http::{private::encoder::HexEncoder, rest::client::RestClient},
        websocket::{connect, WebSocketParser},
        StreamParser,
    },
};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use hmac::{Hmac, Mac};
use itertools::Itertools;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error, warn};

use crate::{
    balance::{AssetBalance, Balance},
    client::ExecutionClient,
    error::{ConnectivityError, UnindexedClientError, UnindexedOrderError},
    order::{
        id::StrategyId,
        state::{Cancelled, Open},
        Order, RequestCancel, RequestOpen,
    },
    trade::{AssetFees, Trade},
    InstrumentAccountSnapshot, UnindexedAccountEvent, UnindexedAccountSnapshot,
};

use super::BybitConfig;

const WEBSOCKET_BASE_URL_BYBIT: &str = "wss://stream.bybit.com/v5/private";
const HTTP_BASE_URL_BYBIT: &str = "https://api.bybit.com";

/// Only UTA 2.0 account type is supported by this client
/// https://bybit-exchange.github.io/docs/v5/acct-mode#uta-20
#[derive(Debug, Clone)]
pub struct BybitClient<Server> {
    config: BybitConfig,
    rest_client: RestClient<'static, BybitRequestSigner, BybitParser>,
    server: PhantomData<Server>,
}

impl<Server> ExecutionClient for BybitClient<Server>
where
    Server: Clone + BybitServer + Sync,
{
    const EXCHANGE: ExchangeId = Server::EXCHANGE;
    type Config = BybitConfig;
    type AccountStream = UnboundedReceiverStream<UnindexedAccountEvent>;

    fn new(config: Self::Config) -> Self {
        let hmac = Hmac::new_from_slice(config.credentials.secret.as_bytes())
            .expect("ApiCredentials secret invalid length");

        Self {
            rest_client: RestClient::new(
                HTTP_BASE_URL_BYBIT,
                BybitRequestSigner::new(
                    BybitSigner::new(config.credentials.key.clone()),
                    hmac,
                    HexEncoder,
                ),
                BybitParser,
            ),
            config,
            server: PhantomData,
        }
    }

    async fn account_snapshot(
        &self,
        _: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> Result<UnindexedAccountSnapshot, UnindexedClientError> {
        let balances = self.fetch_balances().await?;

        let orders_by_instrument = self
            .fetch_open_orders()
            .await?
            .into_iter()
            .sorted_by(|a, b| a.instrument.cmp(&b.instrument))
            .chunk_by(|order| order.instrument.clone());

        let instruments = orders_by_instrument
            .into_iter()
            .fold(Vec::with_capacity(instruments.len()), |mut snapshots, (instrument, orders)| {
                if !instruments.contains(&instrument) {
                    warn!(
                        exchange = %Self::EXCHANGE,
                        %instrument,
                        "AccountSnapshot | received open_orders for untracked instrument - filtering"
                    );

                    return snapshots
                }

                snapshots.push(InstrumentAccountSnapshot {
                    instrument,
                    orders: orders
                        .into_iter()
                        .map(Order::from)
                        .collect(),
                });

                snapshots
            });

        Ok(UnindexedAccountSnapshot {
            exchange: Self::EXCHANGE,
            balances,
            instruments,
        })
    }

    async fn account_stream(
        &self,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange],
    ) -> Result<Self::AccountStream, UnindexedClientError> {
        let mut stream = connect(WEBSOCKET_BASE_URL_BYBIT)
            .await
            .map_err(|err| ConnectivityError::Socket(err.to_string()))?;

        // Authenticate connection and subscribe to the required topics.
        subscribe(&self.config.credentials, &mut stream)
            .await
            .map_err(|err| ConnectivityError::Socket(err.to_string()))?;

        // Channel used to push account updates
        let (events_tx, events_rx) = mpsc_unbounded::<UnindexedAccountEvent>();

        // Handle events from the client
        tokio::spawn({
            let assets = assets.to_vec();
            let instruments = instruments.to_vec();

            async move {
                while let Some(message) = stream.next().await {
                    // TODO: Log error that we received

                    let Some(parsed) = WebSocketParser::parse::<BybitPayload>(message) else {
                        continue;
                    };

                    let message = match parsed {
                        Ok(message) => message,
                        Err(err) => {
                            error!(
                                ?err,
                                "received error from the ByBit execution client stream"
                            );
                            // TODO: Should we cancel the stream?
                            return;
                        }
                    };

                    match extract_event(message, &assets, &instruments).await {
                        // The event should be published
                        Ok(Some(event)) => {
                            events_tx.send(event).unwrap();
                        }
                        // Event was filtered out
                        Ok(None) => {
                            todo!()
                        }
                        // Error
                        Err(err) => {
                            // TODO: Check if the error should cancel the stream
                            error!(
                                ?err,
                                "error occurred while handling Bybit execution client message"
                            );
                            return;
                        }
                    }
                }
            }
        });

        Ok(events_rx.into_stream())
    }

    async fn cancel_order(
        &self,
        cancel_request: Order<ExchangeId, &InstrumentNameExchange, RequestCancel>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Cancelled, UnindexedOrderError>> {
        let request = CancelOrderRequest::new(CancelOrderBody {
            category: Server::CATEGORY,
            instrument: cancel_request.instrument.clone(),
            exchange_order_id: None,
            client_order_id: Some(cancel_request.cid.clone()),
        });

        let state = self
            .rest_client
            .execute(request)
            .await
            .map(|(response, _metric)| Cancelled {
                id: response.result.exchange_order_id,
                time_exchange: response.time,
            })
            .map_err(Into::into);

        Order {
            exchange: Self::EXCHANGE,
            instrument: cancel_request.instrument.clone(),
            strategy: cancel_request.strategy,
            cid: cancel_request.cid,
            side: cancel_request.side,
            state,
        }
    }

    async fn open_order(
        &self,
        open_request: Order<ExchangeId, &InstrumentNameExchange, RequestOpen>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedOrderError>> {
        // let time_in_force = match open_request.state.time_in_force.try_into() {
        //     Ok(time_in_force) => time_in_force,
        //     Err(err) => {
        //         return Order {
        //             exchange: Self::EXCHANGE,
        //             instrument: open_request.instrument.clone(),
        //             strategy: open_request.strategy,
        //             cid: open_request.cid,
        //             side: open_request.side,
        //             state: Err(err),
        //         }
        //     }
        // };

        // let request = PlaceOrderRequest::new(PlaceOrderBody {
        //     category: Server::CATEGORY,
        //     instrument: open_request.instrument.clone(),
        //     client_order_id: Some(open_request.cid.clone()),
        //     side: open_request.side,
        //     kind: open_request.state.kind,
        //     time_in_force,
        //     quantity: open_request.state.quantity,
        //     price: Some(open_request.state.price),
        //     position_side: None,
        //     reduce_only: None,
        // });

        // let state = self
        //     .rest_client
        //     .execute(request)
        //     .await
        //     .map(|(response, _metric)| Open {
        //         id: response.result.exchange_order_id,
        //         time_exchange: response.time,
        //         price: open_request.state.price,
        //         quantity: open_request.state.quantity,
        //         filled_quantity: Decimal::ZERO,
        //     })
        //     .map_err(Into::into);

        // Order {
        //     exchange: Self::EXCHANGE,
        //     instrument: open_request.instrument.clone(),
        //     strategy: open_request.strategy,
        //     cid: open_request.cid,
        //     side: open_request.side,
        //     state,
        // }
        todo!()
    }

    async fn fetch_balances(
        &self,
    ) -> Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError> {
        let request = GetWalletBalanceRequest::new(GetWalletBalanceParams {
            account_type: AccountType::Unified,
            coin: None,
        });

        // TODO: Implement pagination
        let (response, _) = self.rest_client.execute(request).await?;
        let balances: Vec<GetWalletBalanceResponseInner> = response.result.list;

        // We only support Unified account
        let Some(balances) = balances
            .into_iter()
            .find(|b| b.account_type == AccountType::Unified)
        else {
            return Ok(vec![]);
        };

        let balances = balances
            .coin
            .into_iter()
            .map(|balance| AssetBalance {
                asset: balance.asset,
                balance: Balance {
                    total: balance.total_balance,
                    free: balance.total_balance - balance.locked_balance,
                },
                time_exchange: response.time,
            })
            .collect::<Vec<_>>();

        Ok(balances)
    }

    async fn fetch_open_orders(
        &self,
    ) -> Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError> {
        let request = GetOpenAndClosedOrders::new(GetOpenAndClosedOrdersParams {
            category: Server::CATEGORY,
        });
        let (response, _) = self.rest_client.execute(request).await?;

        let orders = response
            .result
            .list
            .into_iter()
            .filter_map(|o| {
                let Some(cid) = o.client_order_id else {
                    debug!("fetch_open_orders: filtered out an order without a client id");
                    return None;
                };

                Some(Order {
                    exchange: Self::EXCHANGE,
                    instrument: o.instrument,
                    strategy: StrategyId::unknown(),
                    cid,
                    side: o.side,
                    state: Open {
                        id: o.exchange_order_id,
                        time_exchange: response.time,
                        price: o.price,
                        quantity: o.quantity,
                        filled_quantity: o.filled_quantity,
                    },
                })
            })
            .collect::<Vec<_>>();

        Ok(orders)
    }

    async fn fetch_trades(
        &self,
        time_since: DateTime<Utc>,
    ) -> Result<Vec<Trade<QuoteAsset, InstrumentNameExchange>>, UnindexedClientError> {
        // TODO: Use time_since
        let request = GetOrderTradesRequest::new(GetOrderTradesParams {
            category: Server::CATEGORY,
            client_order_id: None,
            limit: Some(100), // Max limit available
            cursor: None,
        });

        // TODO: Implement pagination
        let (response, _) = self.rest_client.execute(request).await?;

        let trades = response
            .result
            .list
            .into_iter()
            .map(|t| Trade {
                id: t.trade_id,
                order_id: t.exchange_order_id,
                instrument: t.instrument,
                strategy: StrategyId::unknown(),
                time_exchange: t.executed_at,
                side: t.side,
                price: t.exec_price,
                quantity: t.exec_qty,
                // TODO: Set the correct fees
                fees: AssetFees::default(),
            })
            .collect::<Vec<_>>();

        Ok(trades)
    }
}
