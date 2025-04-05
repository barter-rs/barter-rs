use crate::{
    engine::state::{asset::AssetStates, instrument::InstrumentStates, position::PositionExited},
    statistic::{
        summary::{
            asset::{TearSheetAsset, TearSheetAssetGenerator},
            instrument::{TearSheet, TearSheetGenerator},
        },
        time::TimeInterval,
    },
};
use barter_execution::balance::AssetBalance;
use barter_instrument::{
    asset::{AssetIndex, ExchangeAsset, name::AssetNameInternal},
    instrument::{InstrumentIndex, name::InstrumentNameInternal},
};
use barter_integration::{collection::FnvIndexMap, snapshot::Snapshot};
use chrono::{DateTime, TimeDelta, Utc};
use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub mod asset;
pub mod dataset;
pub mod display;
pub mod instrument;
pub mod pnl;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct TradingSummary<Interval> {
    /// Trading session start time defined by the [`Engine`](crate::engine::Engine) clock.
    pub time_engine_start: DateTime<Utc>,

    /// Trading session end time defined by the [`Engine`](crate::engine::Engine) clock.
    pub time_engine_end: DateTime<Utc>,

    /// Instrument [`TearSheet`]s.
    ///
    /// Note that an Instrument is unique to an exchange, so, for example, Binance btc_usdt_spot
    /// and Okx btc_usdt_spot will be summarised by distinct [`TearSheet`]s.
    pub instruments: FnvIndexMap<InstrumentNameInternal, TearSheet<Interval>>,

    /// [`ExchangeAsset`] [`TearSheet`]s.
    pub assets: FnvIndexMap<ExchangeAsset<AssetNameInternal>, TearSheetAsset>,
}

impl<Interval> TradingSummary<Interval> {
    /// Duration of trading that the `TradingSummary` covers.
    pub fn trading_duration(&self) -> TimeDelta {
        self.time_engine_end
            .signed_duration_since(self.time_engine_start)
    }
}

/// Generator for a [`TradingSummary`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Constructor)]
pub struct TradingSummaryGenerator {
    /// Theoretical rate of return of an investment with zero risk.
    ///
    /// See docs: <https://www.investopedia.com/terms/r/risk-freerate.asp>
    pub risk_free_return: Decimal,

    /// Trading session summary start time defined by the [`Engine`](crate::engine::Engine) clock.
    pub time_engine_start: DateTime<Utc>,

    /// Trading session summary most recent update time defined by the
    /// [`Engine`](crate::engine::Engine) clock.
    pub time_engine_now: DateTime<Utc>,

    /// Instrument [`TearSheetGenerator`]s.
    ///
    /// Note that an Instrument is unique to an exchange, so, for example, Binance btc_usdt_spot
    /// and Okx btc_usdt_spot will be summarised by distinct [`TearSheet`]s.
    pub instruments: FnvIndexMap<InstrumentNameInternal, TearSheetGenerator>,

    /// [`ExchangeAsset`] [`TearSheetAssetGenerator`]s.
    pub assets: FnvIndexMap<ExchangeAsset<AssetNameInternal>, TearSheetAssetGenerator>,
}

impl TradingSummaryGenerator {
    /// Initialise a [`TradingSummaryGenerator`] from a `risk_free_return` value, and initial
    /// indexed state.
    pub fn init<InstrumentData>(
        risk_free_return: Decimal,
        time_engine_start: DateTime<Utc>,
        time_engine_now: DateTime<Utc>,
        instruments: &InstrumentStates<InstrumentData>,
        assets: &AssetStates,
    ) -> Self {
        Self {
            risk_free_return,
            time_engine_start,
            time_engine_now,
            instruments: instruments
                .0
                .values()
                .map(|state| {
                    (
                        state.instrument.name_internal.clone(),
                        state.tear_sheet.clone(),
                    )
                })
                .collect(),
            assets: assets
                .0
                .iter()
                .map(|(asset, state)| (asset.clone(), state.statistics.clone()))
                .collect(),
        }
    }

    /// Update the [`TradingSummaryGenerator`] `time_now`.
    pub fn update_time_now(&mut self, time_now: DateTime<Utc>) {
        self.time_engine_now = time_now;
    }

    /// Update the [`TradingSummaryGenerator`] from the next [`PositionExited`].
    pub fn update_from_position<AssetKey, InstrumentKey>(
        &mut self,
        position: &PositionExited<AssetKey, InstrumentKey>,
    ) where
        Self: InstrumentTearSheetManager<InstrumentKey>,
    {
        if self.time_engine_now < position.time_exit {
            self.time_engine_now = position.time_exit;
        }

        self.instrument_mut(&position.instrument)
            .update_from_position(position)
    }

    /// Update the [`TradingSummaryGenerator`] from the next [`Snapshot`] [`AssetBalance`].
    pub fn update_from_balance<AssetKey>(&mut self, balance: Snapshot<&AssetBalance<AssetKey>>)
    where
        Self: AssetTearSheetManager<AssetKey>,
    {
        if self.time_engine_now < balance.0.time_exchange {
            self.time_engine_now = balance.0.time_exchange;
        }

        self.asset_mut(&balance.0.asset)
            .update_from_balance(balance)
    }

    /// Generate the latest [`TradingSummary`] at the specific [`TimeInterval`].
    ///
    /// For example, pass [`Annual365`](super::time::Annual365) to generate a crypto-centric
    /// (24/7 trading) annualised [`TradingSummary`].
    pub fn generate<Interval>(&mut self, interval: Interval) -> TradingSummary<Interval>
    where
        Interval: TimeInterval,
    {
        let instruments = self
            .instruments
            .iter_mut()
            .map(|(instrument, tear_sheet)| {
                (
                    instrument.clone(),
                    tear_sheet.generate(self.risk_free_return, interval),
                )
            })
            .collect();

        let assets = self
            .assets
            .iter_mut()
            .map(|(asset, tear_sheet)| (asset.clone(), tear_sheet.generate()))
            .collect();

        TradingSummary {
            time_engine_start: self.time_engine_start,
            time_engine_end: self.time_engine_now,
            instruments,
            assets,
        }
    }
}

pub trait InstrumentTearSheetManager<InstrumentKey> {
    fn instrument(&self, key: &InstrumentKey) -> &TearSheetGenerator;
    fn instrument_mut(&mut self, key: &InstrumentKey) -> &mut TearSheetGenerator;
}

impl InstrumentTearSheetManager<InstrumentNameInternal> for TradingSummaryGenerator {
    fn instrument(&self, key: &InstrumentNameInternal) -> &TearSheetGenerator {
        self.instruments
            .get(key)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key}"))
    }

    fn instrument_mut(&mut self, key: &InstrumentNameInternal) -> &mut TearSheetGenerator {
        self.instruments
            .get_mut(key)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key}"))
    }
}

impl InstrumentTearSheetManager<InstrumentIndex> for TradingSummaryGenerator {
    fn instrument(&self, key: &InstrumentIndex) -> &TearSheetGenerator {
        self.instruments
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key}"))
    }

    fn instrument_mut(&mut self, key: &InstrumentIndex) -> &mut TearSheetGenerator {
        self.instruments
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key}"))
    }
}

pub trait AssetTearSheetManager<AssetKey> {
    fn asset(&self, key: &AssetKey) -> &TearSheetAssetGenerator;
    fn asset_mut(&mut self, key: &AssetKey) -> &mut TearSheetAssetGenerator;
}

impl AssetTearSheetManager<AssetIndex> for TradingSummaryGenerator {
    fn asset(&self, key: &AssetIndex) -> &TearSheetAssetGenerator {
        self.assets
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key}"))
    }

    fn asset_mut(&mut self, key: &AssetIndex) -> &mut TearSheetAssetGenerator {
        self.assets
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key}"))
    }
}

impl AssetTearSheetManager<ExchangeAsset<AssetNameInternal>> for TradingSummaryGenerator {
    fn asset(&self, key: &ExchangeAsset<AssetNameInternal>) -> &TearSheetAssetGenerator {
        self.assets
            .get(key)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key:?}"))
    }

    fn asset_mut(
        &mut self,
        key: &ExchangeAsset<AssetNameInternal>,
    ) -> &mut TearSheetAssetGenerator {
        self.assets
            .get_mut(key)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key:?}"))
    }
}
