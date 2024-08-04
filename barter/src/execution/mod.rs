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

pub type AccountStreamEvent<
    ExchangeKey = ExchangeIndex,
    AssetKey = AssetIndex,
    InstrumentKey = InstrumentIndex,
> = reconnect::Event<ExchangeId, AccountEvent<ExchangeKey, AssetKey, InstrumentKey>>;
