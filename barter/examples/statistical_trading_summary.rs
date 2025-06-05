use barter::{
    engine::state::{
        EngineState, global::DefaultGlobalData, instrument::data::DefaultInstrumentMarketData,
        position::PositionExited, trading::TradingState,
    },
    statistic::{summary::TradingSummaryGenerator, time::Annual365},
};
use barter_execution::{
    balance::{AssetBalance, Balance},
    trade::{AssetFees, TradeId},
};
use barter_instrument::{
    Side, Underlying,
    asset::{AssetIndex, QuoteAsset},
    exchange::ExchangeId,
    index::IndexedInstruments,
    instrument::{Instrument, InstrumentIndex},
};
use barter_integration::snapshot::Snapshot;
use chrono::{DateTime, Days, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;

// Risk-free rate of 5% (configure as needed)
const RISK_FREE_RETURN: Decimal = dec!(0.05);
const EXCHANGE: ExchangeId = ExchangeId::BinanceSpot;
const STARTING_BALANCE_USDT: Balance = Balance {
    total: dec!(10_000.0),
    free: dec!(10_000.0),
};
const STARTING_BALANCE_BTC: Balance = Balance {
    total: dec!(0.1),
    free: dec!(0.1),
};
const STARTING_BALANCE_ETH: Balance = Balance {
    total: dec!(1.0),
    free: dec!(1.0),
};

pub enum ContrivedEvents {
    Balance(Snapshot<AssetBalance<AssetIndex>>),
    Position(PositionExited<QuoteAsset, InstrumentIndex>),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate IndexedInstruments
    let instruments = indexed_instruments();

    // Set initial timestamp to seed Instrument TearSheets
    let time_now = Utc::now();

    // Construct EngineState from IndexedInstruments and hard-coded exchange asset Balances
    let state = EngineState::builder(&instruments, DefaultGlobalData::default(), |_| {
        DefaultInstrumentMarketData::default()
    })
    .time_engine_start(time_now)
    // Note: you may want to start to engine with TradingState::Disabled and turn on later
    .trading_state(TradingState::Enabled)
    .balances([
        (EXCHANGE, "usdt", STARTING_BALANCE_USDT),
        (EXCHANGE, "btc", STARTING_BALANCE_BTC),
        (EXCHANGE, "eth", STARTING_BALANCE_ETH),
    ])
    // Note: can add other initial data via this builder (eg/ exchange asset balances)
    .build();

    // Initialise TradingSummaryGenerator for all indexed instruments & assets
    // Note: EngineState already contains Instrument & Asset TearSheets
    //  --> this is just an example of using a TradingSummaryGenerator directly
    let mut summary_generator = TradingSummaryGenerator::init(
        RISK_FREE_RETURN,
        time_now, // time_engine_start
        // Note: time_engine_now will be updated by the synthetic updates
        time_now,
        &state.instruments,
        &state.assets,
    );

    // Update TradingSummaryGenerator with some synthetic Balance & PositionExited events
    for update in generate_synthetic_updates(time_now) {
        match update {
            ContrivedEvents::Balance(balance) => {
                summary_generator.update_from_balance(balance.as_ref());
            }
            ContrivedEvents::Position(position) => {
                summary_generator.update_from_position(&position);
            }
        }
    }

    // Generate crypto-centric (24/7 trading) annualised TradingSummary
    let summary = summary_generator.generate(Annual365);

    summary.print_summary();

    Ok(())
}

fn indexed_instruments() -> IndexedInstruments {
    IndexedInstruments::builder()
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_btc_usdt",
            "BTCUSDT",
            Underlying::new("btc", "usdt"),
            None,
        ))
        .add_instrument(Instrument::spot(
            ExchangeId::BinanceSpot,
            "binance_spot_eth_usdt",
            "ETHUSDT",
            Underlying::new("eth", "usdt"),
            None,
        ))
        .build()
}

fn generate_synthetic_updates(base_time: DateTime<Utc>) -> Vec<ContrivedEvents> {
    vec![
        // Update 1: minus 1000 usdt (ie/ executed a Side::Buy MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(9000.0), dec!(9000.0)),
            time_exchange: base_time.checked_add_days(Days::new(1)).unwrap(),
        })),
        // Update 2: plus 3000 usdt (ie/ executed a Side::Sell MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(12_000.0), dec!(12_000.0)),
            time_exchange: base_time.checked_add_days(Days::new(2)).unwrap(),
        })),
        // Update 3: PositionExited
        ContrivedEvents::Position(PositionExited {
            instrument: InstrumentIndex(0), // BinanceSpot btc_usdt
            side: Side::Buy,
            price_entry_average: dec!(1.0),
            quantity_abs_max: dec!(1000.0),
            pnl_realised: dec!(2000.0), // 2000 usdt profit
            fees_enter: AssetFees {
                asset: QuoteAsset,
                fees: dec!(0.0),
            },
            fees_exit: AssetFees {
                asset: QuoteAsset,
                fees: dec!(0.0),
            },
            time_enter: base_time.checked_add_days(Days::new(1)).unwrap(),
            time_exit: base_time.checked_add_days(Days::new(2)).unwrap(),
            trades: vec![TradeId(SmolStr::new("1")), TradeId(SmolStr::new("2"))],
        }),
        // Update 4: minus 2000 usdt (ie/ executed a Side::Buy MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(10_000.0), dec!(10_000.0)),
            time_exchange: base_time.checked_add_days(Days::new(2)).unwrap(),
        })),
        // Update 5: plus 3000 usdt (ie/ executed a Side::Sell MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(13_000.0), dec!(13_000.0)),
            time_exchange: base_time.checked_add_days(Days::new(3)).unwrap(),
        })),
        // Update 6: PositionExited
        ContrivedEvents::Position(PositionExited {
            instrument: InstrumentIndex(0), // BinanceSpot btc_usdt
            side: Side::Buy,
            price_entry_average: dec!(1.0),
            quantity_abs_max: dec!(2000.0),
            pnl_realised: dec!(1000.0), // 1000 usdt profit
            fees_enter: AssetFees::default(),
            fees_exit: AssetFees::default(),
            time_enter: base_time.checked_add_days(Days::new(2)).unwrap(),
            time_exit: base_time.checked_add_days(Days::new(3)).unwrap(),
            trades: vec![TradeId(SmolStr::new("3")), TradeId(SmolStr::new("4"))],
        }),
        // Update 7: minus 5000 usdt (ie/ executed a Side::Buy MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(8000.0), dec!(8000.0)),
            time_exchange: base_time.checked_add_days(Days::new(4)).unwrap(),
        })),
        // Update 8: plus 3000 usdt (ie/ executed a Side::Sell MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(11_000.0), dec!(11_000.0)),
            time_exchange: base_time.checked_add_days(Days::new(5)).unwrap(),
        })),
        // Update 9: PositionExited
        ContrivedEvents::Position(PositionExited {
            instrument: InstrumentIndex(0), // BinanceSpot btc_usdt
            side: Side::Buy,
            price_entry_average: dec!(1.0),
            quantity_abs_max: dec!(2000.0),
            pnl_realised: dec!(-2000.0), // 2000 usdt loss
            fees_enter: AssetFees::default(),
            fees_exit: AssetFees::default(),
            time_enter: base_time.checked_add_days(Days::new(4)).unwrap(),
            time_exit: base_time.checked_add_days(Days::new(5)).unwrap(),
            trades: vec![TradeId(SmolStr::new("5")), TradeId(SmolStr::new("6"))],
        }),
        // Update 10: minus 5000 usdt (ie/ executed a Side::Buy MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(6000.0), dec!(6000.0)),
            time_exchange: base_time.checked_add_days(Days::new(6)).unwrap(),
        })),
        // Update 11: minus 1000 usdt (ie/ executed a Side::Buy MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(5000.0), dec!(5000.0)),
            time_exchange: base_time.checked_add_days(Days::new(7)).unwrap(),
        })),
        // Update 12: plus 5000 usdt (ie/ executed a Side::Sell MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(10_000.0), dec!(10_000.0)),
            time_exchange: base_time.checked_add_days(Days::new(8)).unwrap(),
        })),
        // Update 13: PositionExited
        ContrivedEvents::Position(PositionExited {
            instrument: InstrumentIndex(1), // BinanceSpot eth_usdt
            side: Side::Buy,
            price_entry_average: dec!(1.0),
            quantity_abs_max: dec!(6000.0),
            pnl_realised: dec!(-1000.0), // 1000 usdt loss
            fees_enter: AssetFees::default(),
            fees_exit: AssetFees::default(),
            time_enter: base_time.checked_add_days(Days::new(6)).unwrap(),
            time_exit: base_time.checked_add_days(Days::new(8)).unwrap(),
            trades: vec![
                TradeId(SmolStr::new("7")),
                TradeId(SmolStr::new("8")),
                TradeId(SmolStr::new("9")),
            ],
        }),
        // Update 14: minus 3000 usdt (ie/ executed a Side::Buy MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(7000.0), dec!(7000.0)),
            time_exchange: base_time.checked_add_days(Days::new(10)).unwrap(),
        })),
        // Update 15: plus 3500 usdt (ie/ executed a Side::Sell MARKET order with no fees)
        ContrivedEvents::Balance(Snapshot::new(AssetBalance {
            asset: AssetIndex(2), // usdt
            balance: Balance::new(dec!(10_500.0), dec!(10_500.0)),
            time_exchange: base_time.checked_add_days(Days::new(11)).unwrap(),
        })),
        // Update 16: PositionExited
        ContrivedEvents::Position(PositionExited {
            instrument: InstrumentIndex(1), // BinanceSpot eth_usdt
            side: Side::Buy,
            price_entry_average: dec!(1.0),
            quantity_abs_max: dec!(6000.0),
            pnl_realised: dec!(500.0), // 500 usdt profit
            fees_enter: AssetFees::default(),
            fees_exit: AssetFees::default(),
            time_enter: base_time.checked_add_days(Days::new(10)).unwrap(),
            time_exit: base_time.checked_add_days(Days::new(11)).unwrap(),
            trades: vec![TradeId(SmolStr::new("10")), TradeId(SmolStr::new("11"))],
        }),
    ]
}
