use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use barter_instrument::asset::name::AssetNameExchange;
use barter_instrument::asset::QuoteAsset;
use barter_instrument::exchange::ExchangeId;
use barter_instrument::instrument::name::InstrumentNameExchange;
use barter_integration::protocol::http::private::encoder::HexEncoder;
use barter_integration::protocol::http::private::RequestSigner;
use barter_integration::protocol::http::rest::client::RestClient;
use crate::balance::AssetBalance;
use crate::client::binance::http_parser::BinanceSpotHttpParser;
use crate::client::binance::signer::{BinanceSigner, BinanceSpotSigner};
use crate::client::ExecutionClient;
use crate::error::UnindexedClientError;
use crate::order::{Cancelled, Open, Order, RequestCancel, RequestOpen};
use crate::trade::Trade;
use crate::{ApiCredentials, UnindexedAccountEvent, UnindexedAccountSnapshot};

pub mod model;
pub mod signer;
pub mod http_parser;

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
                RequestSigner::new(
                    BinanceSigner::new(config.credentials.key),
                    hmac,
                    HexEncoder
                ),
                BinanceSpotHttpParser
            )
        }
    }

    async fn account_snapshot(
        &self,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange]
    ) -> Result<UnindexedAccountSnapshot, UnindexedClientError>
    {
        todo!()
    }

    async fn account_stream(
        &self,
        assets: &[AssetNameExchange],
        instruments: &[InstrumentNameExchange]
    ) -> Result<Self::AccountStream, UnindexedClientError> {
        todo!()
    }

    async fn cancel_order(
        &self,
        request: Order<ExchangeId, &InstrumentNameExchange, RequestCancel>
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Cancelled, UnindexedClientError>>
    {
        todo!()
    }

    async fn open_order(
        &self,
        request: Order<ExchangeId, &InstrumentNameExchange, RequestOpen>
    ) -> Order<ExchangeId, InstrumentNameExchange, Result<Open, UnindexedClientError>>
    {
        todo!()
    }

    async fn fetch_balances(
        &self
    ) -> Result<Vec<AssetBalance<AssetNameExchange>>, UnindexedClientError> {
        todo!()
    }

    async fn fetch_open_orders(
        &self
    ) -> Result<Vec<Order<ExchangeId, InstrumentNameExchange, Open>>, UnindexedClientError> {
        todo!()
    }

    async fn fetch_trades(
        &self,
        time_since: DateTime<Utc>
    ) -> Result<Vec<Trade<QuoteAsset, InstrumentNameExchange>>, UnindexedClientError> {
        todo!()
    }
}