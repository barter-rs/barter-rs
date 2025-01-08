use crate::{
    balance::AssetBalance,
    client::ExecutionClient,
    error::UnindexedClientError,
    order::{Cancelled, Open, Order, RequestCancel, RequestOpen},
    trade::Trade,
    ApiCredentials, InstrumentAccountSnapshot, UnindexedAccountEvent, UnindexedAccountSnapshot,
};
use barter_instrument::{
    asset::{name::AssetNameExchange, QuoteAsset},
    exchange::ExchangeId,
    instrument::name::InstrumentNameExchange,
};
use barter_integration::protocol::http::{
    private::{encoder::HexEncoder, RequestSigner},
    rest::client::RestClient,
};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use http::{
    parser::BinanceSpotHttpParser,
    signer::{BinanceSigner, BinanceSpotSigner},
};
use itertools::Itertools;
use tracing::warn;

mod http;
mod model;
mod websocket;

const HTTP_BASE_URL_BINANCE_SPOT: &str = "https://api.binance.com";

#[derive(Debug, Clone)]
pub struct BinanceSpotConfig {
    credentials: ApiCredentials,
}

#[derive(Debug, Clone)]
pub struct BinanceSpot {
    rest_client: RestClient<'static, BinanceSpotSigner, BinanceSpotHttpParser>,
}

impl ExecutionClient for BinanceSpot {
    const EXCHANGE: ExchangeId = ExchangeId::BinanceSpot;
    type Config = BinanceSpotConfig;
    type AccountStream = futures::stream::Empty<UnindexedAccountEvent>;

    fn new(config: Self::Config) -> Self {
        let hmac = Hmac::new_from_slice(config.credentials.secret.as_bytes())
            .expect("ApiCredentials secret invalid length");

        Self {
            rest_client: RestClient::new(
                HTTP_BASE_URL_BINANCE_SPOT,
                RequestSigner::new(BinanceSigner::new(config.credentials.key), hmac, HexEncoder),
                BinanceSpotHttpParser,
            ),
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
                        "BinanceSpot | AccountSnapshot | received open_orders for untracked instrument - filtering"
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
        todo!()
    }

    async fn cancel_order(
        &self,
        request: Order<ExchangeId, &InstrumentNameExchange, RequestCancel>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Cancelled, UnindexedClientError>> {
        todo!()
    }

    async fn open_order(
        &self,
        request: Order<ExchangeId, &InstrumentNameExchange, RequestOpen>,
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedClientError>> {
        todo!()
    }

    async fn fetch_balances(
        &self,
    ) -> Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError> {
        todo!()
    }

    async fn fetch_open_orders(
        &self,
    ) -> Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError> {
        todo!()
    }

    async fn fetch_trades(
        &self,
        time_since: DateTime<Utc>,
    ) -> Result<Vec<Trade<QuoteAsset, InstrumentNameExchange>>, UnindexedClientError> {
        todo!()
    }
}
