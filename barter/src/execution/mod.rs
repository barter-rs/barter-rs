use barter_data::streams::reconnect;
use barter_execution::AccountEvent;
use barter_instrument::{
    asset::AssetIndex,
    exchange::{ExchangeId, ExchangeIndex},
    instrument::InstrumentIndex,
};

pub mod builder;
pub mod error;
pub mod manager;
pub mod request;

pub type IndexedAccountStreamEvent = AccountStreamEvent<ExchangeIndex, AssetIndex, InstrumentIndex>;

pub type AccountStreamEvent<ExchangeKey, AssetKey, InstrumentKey> =
    reconnect::Event<ExchangeId, AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>;
