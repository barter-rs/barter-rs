use crate::{
    Timed,
    statistic::metric::drawdown::{
        Drawdown, DrawdownGenerator,
        max::{MaxDrawdown, MaxDrawdownGenerator},
        mean::{MeanDrawdown, MeanDrawdownGenerator},
    },
};
use barter_execution::balance::{AssetBalance, Balance};
use barter_integration::snapshot::Snapshot;
use serde::{Deserialize, Serialize};

/// TearSheet summarising the trading session changes for an Asset.
#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct TearSheetAsset {
    pub balance_end: Option<Balance>,
    pub drawdown: Option<Drawdown>,
    pub drawdown_mean: Option<MeanDrawdown>,
    pub drawdown_max: Option<MaxDrawdown>,
}

/// Generator for an [`TearSheetAsset`].
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Deserialize, Serialize)]
pub struct TearSheetAssetGenerator {
    pub balance_now: Option<Balance>,
    pub drawdown: DrawdownGenerator,
    pub drawdown_mean: MeanDrawdownGenerator,
    pub drawdown_max: MaxDrawdownGenerator,
}

impl TearSheetAssetGenerator {
    /// Initialise a [`TearSheetAssetGenerator`] from an initial `AssetState`.
    pub fn init(initial: &Timed<Balance>) -> Self {
        Self {
            balance_now: Some(initial.value),
            drawdown: DrawdownGenerator::init(Timed::new(initial.value.total, initial.time)),
            drawdown_mean: MeanDrawdownGenerator::default(),
            drawdown_max: MaxDrawdownGenerator::default(),
        }
    }

    /// Update the [`TearSheetAssetGenerator`] from the next [`Snapshot`] [`AssetBalance`].
    pub fn update_from_balance<AssetKey>(&mut self, balance: Snapshot<&AssetBalance<AssetKey>>) {
        self.balance_now = Some(balance.value().balance);

        if let Some(next_drawdown) = self.drawdown.update(Timed::new(
            balance.value().balance.total,
            balance.value().time_exchange,
        )) {
            self.drawdown_mean.update(&next_drawdown);
            self.drawdown_max.update(&next_drawdown);
        }
    }

    /// Generate the latest [`TearSheetAsset`].
    pub fn generate(&mut self) -> TearSheetAsset {
        let current_drawdown = self.drawdown.generate();
        if let Some(drawdown) = &current_drawdown {
            self.drawdown_mean.update(drawdown);
            self.drawdown_max.update(drawdown);
        }

        TearSheetAsset {
            balance_end: self.balance_now,
            drawdown: current_drawdown,
            drawdown_mean: self.drawdown_mean.generate(),
            drawdown_max: self.drawdown_max.generate(),
        }
    }

    /// Reset the internal state, using a new starting `Timed<Balance>` as seed.
    pub fn reset(&mut self, balance_start: &Timed<Balance>) {
        *self = Self::init(balance_start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::time_plus_days;
    use barter_instrument::asset::AssetIndex;
    use chrono::{DateTime, Utc};
    use rust_decimal_macros::dec;

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
            expected: TearSheetAssetGenerator,
        }

        let base_time = DateTime::<Utc>::MIN_UTC;

        let mut generator = TearSheetAssetGenerator::init(&Timed::new(
            Balance::new(dec!(1.0), dec!(1.0)),
            base_time,
        ));

        let cases = vec![
            // TC0: Balance increased from 1.0 peak, so no expected drawdowns
            TestCase {
                input: balance(
                    Balance::new(dec!(2.0), dec!(2.0)),
                    time_plus_days(base_time, 1),
                ),
                expected: TearSheetAssetGenerator {
                    balance_now: Some(Balance::new(dec!(2.0), dec!(2.0))),
                    drawdown: DrawdownGenerator::init(Timed::new(
                        dec!(2.0),
                        time_plus_days(base_time, 1),
                    )),
                    drawdown_mean: MeanDrawdownGenerator::default(),
                    drawdown_max: MaxDrawdownGenerator::default(),
                },
            },
            // TC1: Balance decreased, so expect a current drawdown only
            TestCase {
                input: balance(
                    Balance::new(dec!(1.5), dec!(1.5)),
                    time_plus_days(base_time, 2),
                ),
                expected: TearSheetAssetGenerator {
                    balance_now: Some(Balance::new(dec!(1.5), dec!(1.5))),
                    drawdown: DrawdownGenerator {
                        peak: Some(dec!(2.0)),
                        drawdown_max: dec!(0.25), // (2.0 - 1.5) / 2.0,
                        time_peak: Some(time_plus_days(base_time, 1)),
                        time_now: time_plus_days(base_time, 2),
                    },
                    drawdown_mean: MeanDrawdownGenerator::default(),
                    drawdown_max: MaxDrawdownGenerator::default(),
                },
            },
            // TC2: Further decrease - larger drawdown
            TestCase {
                input: balance(
                    Balance::new(dec!(1.0), dec!(1.0)),
                    time_plus_days(base_time, 3),
                ),
                expected: TearSheetAssetGenerator {
                    balance_now: Some(Balance::new(dec!(1.0), dec!(1.0))),
                    drawdown: DrawdownGenerator {
                        peak: Some(dec!(2.0)),
                        drawdown_max: dec!(0.5), // (2.0 - 1.0) / 2.0
                        time_peak: Some(time_plus_days(base_time, 1)),
                        time_now: time_plus_days(base_time, 3),
                    },
                    drawdown_mean: MeanDrawdownGenerator::default(),
                    drawdown_max: MaxDrawdownGenerator::default(),
                },
            },
            // TC3: Recovery above previous peak - should complete drawdown period
            TestCase {
                input: balance(
                    Balance::new(dec!(2.5), dec!(2.5)),
                    time_plus_days(base_time, 4),
                ),
                expected: TearSheetAssetGenerator {
                    balance_now: Some(Balance::new(dec!(2.5), dec!(2.5))),
                    drawdown: DrawdownGenerator::init(Timed::new(
                        dec!(2.5),
                        time_plus_days(base_time, 4),
                    )),
                    drawdown_mean: MeanDrawdownGenerator {
                        count: 1,
                        mean_drawdown: Some(MeanDrawdown {
                            mean_drawdown: dec!(0.5), // Only one drawdown period completed
                            mean_drawdown_ms: duration_ms(
                                time_plus_days(base_time, 1),
                                time_plus_days(base_time, 4),
                            ),
                        }),
                    },
                    drawdown_max: MaxDrawdownGenerator {
                        max: Some(MaxDrawdown(Drawdown {
                            value: dec!(0.5),
                            time_start: time_plus_days(base_time, 1),
                            time_end: time_plus_days(base_time, 4),
                        })),
                    },
                },
            },
            // TC4: Small drawdown after new peak (2.5 -> 2.4)
            TestCase {
                input: balance(
                    Balance::new(dec!(2.4), dec!(2.4)),
                    time_plus_days(base_time, 5),
                ),
                expected: TearSheetAssetGenerator {
                    balance_now: Some(Balance::new(dec!(2.4), dec!(2.4))),
                    drawdown: DrawdownGenerator {
                        peak: Some(dec!(2.5)),
                        drawdown_max: dec!(0.04), // (2.5 - 2.4) / 2.5
                        time_peak: Some(time_plus_days(base_time, 4)),
                        time_now: time_plus_days(base_time, 5),
                    },
                    drawdown_mean: MeanDrawdownGenerator {
                        count: 1,
                        mean_drawdown: Some(MeanDrawdown {
                            mean_drawdown: dec!(0.5), // Only one drawdown period completed
                            mean_drawdown_ms: duration_ms(
                                time_plus_days(base_time, 1),
                                time_plus_days(base_time, 4),
                            ),
                        }),
                    },
                    drawdown_max: MaxDrawdownGenerator {
                        max: Some(MaxDrawdown(Drawdown {
                            value: dec!(0.5),
                            time_start: time_plus_days(base_time, 1),
                            time_end: time_plus_days(base_time, 4),
                        })),
                    },
                },
            },
            // TC5: Equal to previous value - drawdown continues
            TestCase {
                input: balance(
                    Balance::new(dec!(2.4), dec!(2.4)),
                    time_plus_days(base_time, 6),
                ),
                expected: TearSheetAssetGenerator {
                    balance_now: Some(Balance::new(dec!(2.4), dec!(2.4))),
                    drawdown: DrawdownGenerator {
                        peak: Some(dec!(2.5)),
                        drawdown_max: dec!(0.04), // (2.5 - 2.4) / 2.5
                        time_peak: Some(time_plus_days(base_time, 4)),
                        time_now: time_plus_days(base_time, 6),
                    },
                    drawdown_mean: MeanDrawdownGenerator {
                        count: 1,
                        mean_drawdown: Some(MeanDrawdown {
                            mean_drawdown: dec!(0.5), // Only one drawdown period completed
                            mean_drawdown_ms: duration_ms(
                                time_plus_days(base_time, 1),
                                time_plus_days(base_time, 4),
                            ),
                        }),
                    },
                    drawdown_max: MaxDrawdownGenerator {
                        max: Some(MaxDrawdown(Drawdown {
                            value: dec!(0.5),
                            time_start: time_plus_days(base_time, 1),
                            time_end: time_plus_days(base_time, 4),
                        })),
                    },
                },
            },
            // TC6: Tiny change, but still in drawdown - retain max drawdown from current period
            TestCase {
                input: balance(
                    Balance::new(dec!(2.41), dec!(2.41)),
                    time_plus_days(base_time, 7),
                ),
                expected: TearSheetAssetGenerator {
                    balance_now: Some(Balance::new(dec!(2.41), dec!(2.41))),
                    drawdown: DrawdownGenerator {
                        peak: Some(dec!(2.5)),
                        drawdown_max: dec!(0.04), // (2.5 - 2.4) / 2.5
                        time_peak: Some(time_plus_days(base_time, 4)),
                        time_now: time_plus_days(base_time, 7),
                    },
                    drawdown_mean: MeanDrawdownGenerator {
                        count: 1,
                        mean_drawdown: Some(MeanDrawdown {
                            mean_drawdown: dec!(0.5), // Only one drawdown period completed
                            mean_drawdown_ms: duration_ms(
                                time_plus_days(base_time, 1),
                                time_plus_days(base_time, 4),
                            ),
                        }),
                    },
                    drawdown_max: MaxDrawdownGenerator {
                        max: Some(MaxDrawdown(Drawdown {
                            value: dec!(0.5),
                            time_start: time_plus_days(base_time, 1),
                            time_end: time_plus_days(base_time, 4),
                        })),
                    },
                },
            },
            // TC7: recovery above previous peak - should complete drawdown period
            TestCase {
                input: balance(
                    Balance::new(dec!(3.0), dec!(3.0)),
                    time_plus_days(base_time, 8),
                ),
                expected: TearSheetAssetGenerator {
                    balance_now: Some(Balance::new(dec!(3.0), dec!(3.0))),
                    drawdown: DrawdownGenerator::init(Timed::new(
                        dec!(3.0),
                        time_plus_days(base_time, 8),
                    )),
                    drawdown_mean: MeanDrawdownGenerator {
                        count: 2,
                        mean_drawdown: Some(MeanDrawdown {
                            mean_drawdown: dec!(0.27), // (0.5 + 0.04) / 2
                            mean_drawdown_ms: (duration_ms(
                                time_plus_days(base_time, 1),
                                time_plus_days(base_time, 4),
                            ) + duration_ms(
                                time_plus_days(base_time, 4),
                                time_plus_days(base_time, 8),
                            )) / 2,
                        }),
                    },
                    drawdown_max: MaxDrawdownGenerator {
                        max: Some(MaxDrawdown(Drawdown {
                            value: dec!(0.5),
                            time_start: time_plus_days(base_time, 1),
                            time_end: time_plus_days(base_time, 4),
                        })),
                    },
                },
            },
        ];

        for (index, test) in cases.into_iter().enumerate() {
            generator.update_from_balance(Snapshot(&test.input));
            assert_eq!(generator, test.expected, "TC{index} failed");
        }
    }
}
