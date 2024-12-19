use crate::{
    engine::state::{
        asset::{manager::AssetStateManager, AssetStates},
        position::PositionExited,
    },
    statistic::{
        summary::{
            asset::{TearSheetAsset, TearSheetAssetGenerator},
            instrument::{TearSheet, TearSheetGenerator},
        },
        time::TimeInterval,
    },
};
use barter_execution::{balance::AssetBalance, FnvIndexMap};
use barter_instrument::{
    asset::{name::AssetNameInternal, AssetIndex, ExchangeAsset},
    index::IndexedInstruments,
    instrument::{name::InstrumentNameInternal, InstrumentIndex},
};
use barter_integration::snapshot::Snapshot;
use chrono::{DateTime, TimeDelta, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

pub mod asset;
pub mod dataset;
mod display;
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
    pub risk_free_return: f64,

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
    /// Initialise a [`TradingSummaryGenerator`] from [`IndexedInstruments`], an initial state
    /// snapshot, and a configurable `risk_free_return` value.
    pub fn init(
        instruments: &IndexedInstruments,
        time_engine_start: DateTime<Utc>,
        asset_states: &AssetStates,
        risk_free_return: f64,
    ) -> Self {
        Self {
            risk_free_return,
            time_engine_start,
            time_engine_now: time_engine_start,
            instruments: instruments
                .instruments()
                .iter()
                .map(|instrument| {
                    (
                        instrument.value.name_internal.clone(),
                        TearSheetGenerator::init(time_engine_start),
                    )
                })
                .collect(),
            assets: instruments
                .assets()
                .iter()
                .map(|asset| {
                    (
                        ExchangeAsset {
                            exchange: asset.value.exchange,
                            asset: asset.value.asset.name_internal.clone(),
                        },
                        TearSheetAssetGenerator::init(asset_states.asset(&asset.key)),
                    )
                })
                .collect(),
        }
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
        Self: AssetStateManager<AssetKey, State = TearSheetAssetGenerator>,
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
    pub fn generate<Interval>(&self, interval: Interval) -> TradingSummary<Interval>
    where
        Interval: TimeInterval,
    {
        let instruments = self
            .instruments
            .iter()
            .map(|(instrument, tear_sheet)| {
                (
                    instrument.clone(),
                    tear_sheet.generate(self.risk_free_return, interval),
                )
            })
            .collect();

        let assets = self
            .assets
            .iter()
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

impl AssetStateManager<AssetIndex> for TradingSummaryGenerator {
    type State = TearSheetAssetGenerator;

    fn asset(&self, key: &AssetIndex) -> &Self::State {
        self.assets
            .get_index(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key}"))
    }

    fn asset_mut(&mut self, key: &AssetIndex) -> &mut Self::State {
        self.assets
            .get_index_mut(key.index())
            .map(|(_key, state)| state)
            .unwrap_or_else(|| panic!("TradingSummaryGenerator does not contain: {key}"))
    }
}

#[cfg(test)]
mod tests {

    // #[test]
    // fn test_trading_summary_generate
}
