use crate::statistic::{
    summary::{asset::TearSheetAsset, instrument::TearSheet, TradingSummary},
    time::TimeInterval,
};
use prettytable::{Cell, Row, Table};

impl<Interval> TradingSummary<Interval>
where
    Interval: TimeInterval,
{
    pub fn print_summary(&self) {
        println!();
        self.title_table().printstd();
        self.instrument_table().printstd();
        self.asset_table().printstd();
    }
    fn title_table(&self) -> Table {
        let mut title_table = Table::new();
        title_table.set_format(*prettytable::format::consts::FORMAT_CLEAN);

        // ASCII art for "TRADING SUMMARY"
        let large_text = vec![
            "████████ ██████   █████  ██████  ██ ███    ██  ██████      ███████ ██    ██ ███    ███ ███    ███  █████  ██████  ██    ██",
            "   ██    ██   ██ ██   ██ ██   ██ ██ ████   ██ ██           ██      ██    ██ ████  ████ ████  ████ ██   ██ ██   ██  ██  ██ ",
            "   ██    ██████  ███████ ██   ██ ██ ██ ██  ██ ██   ███     ███████ ██    ██ ██ ████ ██ ██ ████ ██ ███████ ██████    ████  ",
            "   ██    ██   ██ ██   ██ ██   ██ ██ ██  ██ ██ ██    ██          ██ ██    ██ ██  ██  ██ ██  ██  ██ ██   ██ ██   ██    ██   ",
            "   ██    ██   ██ ██   ██ ██████  ██ ██   ████  ██████      ███████  ██████  ██      ██ ██      ██ ██   ██ ██   ██    ██   "
        ];

        // Add the large text
        for line in large_text {
            let mut cell = Cell::new(line).style_spec("bB"); // Bold and blue
            cell.set_hspan(1);
            title_table.add_row(Row::new(vec![cell]));
        }

        // Add duration on a new line
        let mut duration_cell = Cell::new(&format!(
            "(Trading Duration: {} Minutes)",
            self.trading_duration().num_minutes()
        ))
        .style_spec("bcB");

        duration_cell.set_hspan(1);
        title_table.add_row(Row::new(vec![duration_cell]));

        title_table
    }

    pub fn instrument_table(&self) -> Table {
        let mut table = Table::new();

        // Styling
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

        // Title row spanning all columns
        let num_columns = self.instruments.len() + 1;
        let mut title_row = Row::new(vec![]);
        let mut title_cell = Cell::new("Instrument TearSheets").style_spec("bcB");
        title_cell.set_hspan(num_columns);
        title_row.add_cell(title_cell);
        table.add_row(title_row);

        // Extract TimeInterval name (eg/ Annual365, Daily, etc)
        let interval = match self.instruments.first() {
            Some((_, sheet)) => sheet.sharpe_ratio.interval.name(),
            None => return table,
        };

        // Header row (eg/ Metric | bybit_btc_usdt | okx_eth_usdt | ... )
        let mut header_row = Row::new(vec![Cell::new("").style_spec("bcB")]);
        for instrument in self.instruments.keys() {
            header_row.add_cell(Cell::new(instrument.name().as_str()).style_spec("bcB"));
        }
        table.add_row(header_row);

        // Add metric rows
        self.add_instrument_metric_row(&mut table, "PnL", |ts| format!("{:.2}", ts.pnl));
        self.add_instrument_metric_row(&mut table, &format!("Return {}", interval), |ts| {
            format!("{:.2}%", ts.pnl_return.value * 100.0)
        });
        self.add_instrument_metric_row(&mut table, &format!("Sharpe {}", interval), |ts| {
            format!("{:.3}", ts.sharpe_ratio.value)
        });
        self.add_instrument_metric_row(&mut table, &format!("Sortino {}", interval), |ts| {
            format!("{:.3}", ts.sortino_ratio.value)
        });
        self.add_instrument_metric_row(&mut table, &format!("Calmar {}", interval), |ts| {
            format!("{:.3}", ts.calmar_ratio.value)
        });
        self.add_instrument_metric_row(&mut table, "PnL Drawdown", |ts| {
            if let Some(drawdown) = &ts.pnl_drawdown {
                format!("{:.2}%", drawdown.value * 100.0)
            } else {
                "N/A".to_string()
            }
        });
        self.add_instrument_metric_row(&mut table, "PnL Drawdown Avg", |ts| {
            if let Some(mean_drawdown) = &ts.pnl_drawdown_mean {
                format!("{:.2}%", mean_drawdown.mean_drawdown * 100.0)
            } else {
                "N/A".to_string()
            }
        });
        self.add_instrument_metric_row(&mut table, "PnL Drawdown Max", |ts| {
            if let Some(max_drawdown) = &ts.pnl_drawdown_max {
                format!("{:.2}%", max_drawdown.0.value * 100.0)
            } else {
                "N/A".to_string()
            }
        });
        self.add_instrument_metric_row(&mut table, "Win Rate", |ts| {
            format!("{:.1}%", ts.win_rate.value * 100.0)
        });
        self.add_instrument_metric_row(&mut table, "Profit Factor", |ts| {
            format!("{:.2}", ts.profit_factor.value)
        });

        table
    }

    fn add_instrument_metric_row<F>(&self, table: &mut Table, label: &str, format_value: F)
    where
        F: Fn(&TearSheet<Interval>) -> String,
    {
        let mut row = Row::new(vec![Cell::new(label).style_spec("bcB")]);
        for tear_sheet in self.instruments.values() {
            row.add_cell(Cell::new(&format_value(tear_sheet)));
        }
        table.add_row(row);
    }

    pub fn asset_table(&self) -> Table {
        let mut table = Table::new();

        // Styling
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

        // Title row spanning all columns
        let num_columns = self.assets.len() + 1;
        let mut title_row = Row::new(vec![]);

        let mut title_cell = Cell::new("Asset TearSheets").style_spec("bcB");
        title_cell.set_hspan(num_columns);
        title_row.add_cell(title_cell);
        table.add_row(title_row);

        // Header row (eg/ Metric | btc | eth | ...)
        let mut header_row = Row::new(vec![Cell::new("").style_spec("bcB")]);
        for asset in self.assets.keys() {
            header_row.add_cell(
                Cell::new(&format!("{}_{}", asset.exchange.as_str(), asset.asset))
                    .style_spec("bcB"),
            );
        }
        table.add_row(header_row);

        // Add metric rows
        self.add_asset_metric_row(&mut table, "Balance", |ts| {
            format!("{:.8}", ts.balance_end.total)
        });
        self.add_asset_metric_row(&mut table, "Drawdown", |ts| {
            if let Some(drawdown) = &ts.drawdown {
                format!("{:.2}%", drawdown.value * 100.0)
            } else {
                "N/A".to_string()
            }
        });
        self.add_asset_metric_row(&mut table, "Drawdown Avg", |ts| {
            if let Some(mean_drawdown) = &ts.drawdown_mean {
                format!("{:.2}%", mean_drawdown.mean_drawdown * 100.0)
            } else {
                "N/A".to_string()
            }
        });
        self.add_asset_metric_row(&mut table, "Drawdown Max", |ts| {
            if let Some(max_drawdown) = &ts.drawdown_max {
                format!("{:.2}%", max_drawdown.0.value * 100.0)
            } else {
                "N/A".to_string()
            }
        });

        table
    }

    fn add_asset_metric_row<F>(&self, table: &mut Table, label: &str, format_value: F)
    where
        F: Fn(&TearSheetAsset) -> String,
    {
        let mut row = Row::new(vec![Cell::new(label).style_spec("bcB")]);
        for tear_sheet in self.assets.values() {
            row.add_cell(Cell::new(&format_value(tear_sheet)));
        }
        table.add_row(row);
    }
}
