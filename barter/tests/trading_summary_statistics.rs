use barter::{
    engine::state::{
        asset::{AssetState, AssetStates},
        position::PositionExited,
    },
    statistic::summary::{asset::TearSheetAsset, instrument::TearSheet, TradingSummaryGenerator},
};
use barter_execution::balance::{AssetBalance, Balance};
use barter_instrument::{
    asset::{AssetIndex, ExchangeAsset},
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{kind::InstrumentKind, Instrument, InstrumentIndex},
    test_utils::instrument_spec,
    Underlying,
};
use barter_integration::snapshot::Snapshot;
use chrono::{DateTime, TimeDelta, Utc};

const TIME_START: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;
const RISK_FREE_RETURN: f64 = 0.05;
const BINANCE_BTC_BALANCE: f64 = 0.1;
const BINANCE_USDT_BALANCE: f64 = 10_000.0;

struct TestCase {
    input: Event,
}

enum Event {
    Balance(Snapshot<AssetBalance<AssetIndex>>),
    Position(PositionExited<AssetIndex, InstrumentIndex>),
}

#[test]
fn test_trading_summary_generator_one_instrument() {
    let mut generator = initial_state();

    struct TestCase {
        input: Event,
        expected_instrument: TearSheet<TimeDelta>,
        expected_asset: TearSheetAsset,
    }
}

fn initial_state() -> TradingSummaryGenerator {
    let instruments = instruments();

    let asset_states = AssetStates(
        instruments
            .assets()
            .iter()
            .map(|keyed_asset| {
                (
                    ExchangeAsset::new(
                        keyed_asset.value.exchange,
                        keyed_asset.value.asset.name_internal.clone(),
                    ),
                    AssetState::new(
                        keyed_asset.value.asset.clone(),
                        if keyed_asset.value.exchange == ExchangeId::BinanceSpot
                            && keyed_asset.value.asset.name_internal.as_ref() == "btc"
                        {
                            Balance::new(BINANCE_BTC_BALANCE, BINANCE_BTC_BALANCE)
                        } else if keyed_asset.value.exchange == ExchangeId::BinanceSpot
                            && keyed_asset.value.asset.name_internal.as_ref() == "usdt"
                        {
                            Balance::new(BINANCE_USDT_BALANCE, BINANCE_BTC_BALANCE)
                        } else {
                            unimplemented!()
                        },
                        TIME_START,
                    ),
                )
            })
            .collect(),
    );

    TradingSummaryGenerator::init(&instruments, TIME_START, &asset_states, RISK_FREE_RETURN)
}

fn instruments() -> IndexedInstruments {
    IndexedInstruments::new([Instrument::new(
        ExchangeId::BinanceSpot,
        "binance_spot_btc_usdt",
        "BTCUSDT",
        Underlying::new("btc", "usdt"),
        InstrumentKind::Spot,
        instrument_spec(),
    )])
}
