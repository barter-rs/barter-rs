use crate::{
    engine::state::asset::AssetState,
    statistic::metric::drawdown::{
        max::{MaxDrawdown, MaxDrawdownGenerator},
        mean::{MeanDrawdown, MeanDrawdownGenerator},
        Drawdown, DrawdownGenerator,
    },
    Timed,
};
use barter_execution::balance::{AssetBalance, Balance};
use barter_integration::snapshot::Snapshot;
use serde::{Deserialize, Serialize};

/// TearSheet summarising the trading session changes for an Asset.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct TearSheetAsset {
    pub balance_end: Balance,
    pub drawdown: Option<Drawdown>,
    pub drawdown_mean: Option<MeanDrawdown>,
    pub drawdown_max: Option<MaxDrawdown>,
}

/// Generator for an [`TearSheetAsset`].
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct TearSheetAssetGenerator {
    pub balance: Balance,
    pub drawdown: DrawdownGenerator,
    pub drawdown_mean: MeanDrawdownGenerator,
    pub drawdown_max: MaxDrawdownGenerator,
}

impl TearSheetAssetGenerator {
    /// Initialise a [`TearSheetAssetGenerator`] from an initial [`AssetState`].
    pub fn init(state: &AssetState) -> Self {
        Self {
            balance: state.balance,
            drawdown: DrawdownGenerator::init(Timed::new(state.balance.total, state.time_exchange)),
            drawdown_mean: MeanDrawdownGenerator::default(),
            drawdown_max: MaxDrawdownGenerator::default(),
        }
    }

    /// Update the [`TearSheetAssetGenerator`] from the next [`Snapshot`] [`AssetBalance`].
    pub fn update_from_balance<AssetKey>(&mut self, balance: Snapshot<&AssetBalance<AssetKey>>) {
        if let Some(next_drawdown) = self
            .drawdown
            .update(Timed::new(balance.0.balance.total, balance.0.time_exchange))
        {
            self.drawdown_mean.update(&next_drawdown);
            self.drawdown_max.update(&next_drawdown);
        }

        self.balance = balance.0.balance;
    }

    /// Generate the latest [`TearSheetAsset`].
    pub fn generate(&self) -> TearSheetAsset {
        TearSheetAsset {
            balance_end: self.balance,
            drawdown: self.drawdown.generate(),
            drawdown_mean: self.drawdown_mean.generate(),
            drawdown_max: self.drawdown_max.generate(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::time_plus_days;
    use barter_instrument::asset::{
        name::{AssetNameExchange, AssetNameInternal},
        Asset, AssetIndex,
    };
    use chrono::{DateTime, Utc};

    fn balance(balance: Balance, time: DateTime<Utc>) -> AssetBalance<AssetIndex> {
        AssetBalance {
            asset: AssetIndex(0),
            balance,
            time_exchange: time,
        }
    }

    fn duration_ms(start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
        end.signed_duration_since(start).num_milliseconds()
    }

    #[test]
    fn test_tear_sheet_asset_generator() {
        struct TestCase {
            input: AssetBalance<AssetIndex>,
            expected: TearSheetAsset,
        }

        let base_time = DateTime::<Utc>::MIN_UTC;

        let mut generator = TearSheetAssetGenerator::init(&AssetState {
            asset: Asset {
                name_internal: AssetNameInternal::new("btc"),
                name_exchange: AssetNameExchange::new("xbt"),
            },
            balance: Balance::new(1.0, 1.0),
            time_exchange: base_time,
        });

        let cases = vec![
            // TC0: Balance increased from 1.0 peak, so no expected drawdowns
            TestCase {
                input: balance(Balance::new(2.0, 2.0), time_plus_days(base_time, 1)),
                expected: TearSheetAsset {
                    balance_end: Balance::new(2.0, 2.0),
                    drawdown: None,
                    drawdown_mean: None,
                    drawdown_max: None,
                },
            },
            // TC1: Balance decreased, so expect a current drawdown only
            TestCase {
                input: balance(Balance::new(1.5, 1.5), time_plus_days(base_time, 2)),
                expected: TearSheetAsset {
                    balance_end: Balance::new(1.5, 1.5),
                    drawdown: Some(Drawdown {
                        value: 0.25, // (2.0 - 1.5) / 2.0
                        time_start: time_plus_days(base_time, 1),
                        time_end: time_plus_days(base_time, 2),
                    }),
                    drawdown_mean: None,
                    drawdown_max: None,
                },
            },
            // TC2: Further decrease - larger drawdown
            TestCase {
                input: balance(Balance::new(1.0, 1.0), time_plus_days(base_time, 3)),
                expected: TearSheetAsset {
                    balance_end: Balance::new(1.0, 1.0),
                    drawdown: Some(Drawdown {
                        value: 0.5, // (2.0 - 1.0) / 2.0
                        time_start: time_plus_days(base_time, 1),
                        time_end: time_plus_days(base_time, 3),
                    }),
                    drawdown_mean: None,
                    drawdown_max: None,
                },
            },
            // TC3: Recovery above previous peak - should complete drawdown period
            TestCase {
                input: balance(Balance::new(2.5, 2.5), time_plus_days(base_time, 4)),
                expected: TearSheetAsset {
                    balance_end: Balance::new(2.5, 2.5),
                    drawdown: None,
                    drawdown_mean: Some(MeanDrawdown {
                        mean_drawdown: 0.5, // Only one drawdown period completed
                        mean_drawdown_ms: duration_ms(
                            time_plus_days(base_time, 1),
                            time_plus_days(base_time, 4),
                        ),
                    }),
                    drawdown_max: Some(MaxDrawdown(Drawdown {
                        value: 0.5,
                        time_start: time_plus_days(base_time, 1),
                        time_end: time_plus_days(base_time, 4),
                    })),
                },
            },
            // TC4: Small drawdown after new peak (25/10 -> 24/10)
            TestCase {
                input: balance(Balance::new(2.4, 2.4), time_plus_days(base_time, 5)),
                expected: TearSheetAsset {
                    balance_end: Balance::new(2.4, 2.4),
                    drawdown: Some(Drawdown {
                        value: 0.040000000000000036,
                        time_start: time_plus_days(base_time, 4),
                        time_end: time_plus_days(base_time, 5),
                    }),
                    drawdown_mean: Some(MeanDrawdown {
                        mean_drawdown: 0.5,
                        mean_drawdown_ms: duration_ms(
                            time_plus_days(base_time, 1),
                            time_plus_days(base_time, 4),
                        ),
                    }),
                    drawdown_max: Some(MaxDrawdown(Drawdown {
                        value: 0.5,
                        time_start: time_plus_days(base_time, 1),
                        time_end: time_plus_days(base_time, 4),
                    })),
                },
            },
            // TC5: Equal to previous value - drawdown continues
            TestCase {
                input: balance(Balance::new(2.4, 2.4), time_plus_days(base_time, 6)),
                expected: TearSheetAsset {
                    balance_end: Balance::new(2.4, 2.4),
                    drawdown: Some(Drawdown {
                        value: 0.040000000000000036,
                        time_start: time_plus_days(base_time, 4),
                        time_end: time_plus_days(base_time, 6),
                    }),
                    drawdown_mean: Some(MeanDrawdown {
                        mean_drawdown: 0.5,
                        mean_drawdown_ms: duration_ms(
                            time_plus_days(base_time, 1),
                            time_plus_days(base_time, 4),
                        ),
                    }),
                    drawdown_max: Some(MaxDrawdown(Drawdown {
                        value: 0.5,
                        time_start: time_plus_days(base_time, 1),
                        time_end: time_plus_days(base_time, 4),
                    })),
                },
            },
            // TC6: Tiny change, but still in drawdown - retain max drawdown from current period
            TestCase {
                input: balance(Balance::new(2.41, 2.41), time_plus_days(base_time, 7)),
                expected: TearSheetAsset {
                    balance_end: Balance::new(2.41, 2.41),
                    drawdown: Some(Drawdown {
                        value: 0.040000000000000036, // max drawdown from current period
                        time_start: time_plus_days(base_time, 4),
                        time_end: time_plus_days(base_time, 7),
                    }),
                    drawdown_mean: Some(MeanDrawdown {
                        mean_drawdown: 0.5,
                        mean_drawdown_ms: duration_ms(
                            time_plus_days(base_time, 1),
                            time_plus_days(base_time, 4),
                        ),
                    }),
                    drawdown_max: Some(MaxDrawdown(Drawdown {
                        value: 0.5,
                        time_start: time_plus_days(base_time, 1),
                        time_end: time_plus_days(base_time, 4),
                    })),
                },
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            generator.update_from_balance(Snapshot(&test.input));
            assert_eq!(generator.generate(), test.expected, "TC{index} failed");
        }
    }
}
