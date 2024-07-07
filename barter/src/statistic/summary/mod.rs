pub mod data;
pub mod drawdown;
pub mod pnl;
pub mod trading;

use crate::portfolio::position::Position;
use prettytable::{Cell, Row, Table};

pub trait Initialiser {
    type Config: Copy;
    fn init(config: Self::Config) -> Self;
}

pub trait PositionSummariser: Copy {
    fn update(&mut self, position: &Position);
    fn generate_summary(&mut self, positions: &[Position]) {
        for position in positions.iter() {
            self.update(position)
        }
    }
}

pub trait TableBuilder {
    fn titles(&self) -> Row;
    fn row(&self) -> Row;
    fn table(&self, id_cell: &str) -> Table {
        let mut table = Table::new();

        let mut titles = self.titles();
        titles.insert_cell(0, Cell::new(""));
        table.set_titles(titles);

        let mut row = self.row();
        row.insert_cell(0, Cell::new(id_cell));
        table.add_row(row);

        table
    }
    fn table_with<T: TableBuilder>(&self, id_cell: &str, another: (T, &str)) -> Table {
        let mut table = Table::new();

        let mut titles = self.titles();
        titles.insert_cell(0, Cell::new(""));
        table.set_titles(titles);

        let mut first_row = self.row();
        first_row.insert_cell(0, Cell::new(id_cell));
        table.add_row(first_row);

        let mut another_row = another.0.row();
        another_row.insert_cell(0, Cell::new(another.1));
        table.add_row(another_row);

        table
    }
}

pub fn combine<Iter, T>(builders: Iter) -> Table
where
    Iter: IntoIterator<Item = (String, T)>,
    T: TableBuilder,
{
    builders
        .into_iter()
        .enumerate()
        .fold(Table::new(), |mut table, (index, (id, builder))| {
            // Set Table titles using the first builder
            if index == 0 {
                let mut titles = builder.titles();
                titles.insert_cell(0, Cell::new(""));
                table.set_titles(titles);
            }

            // Add rows for each builder
            let mut row = builder.row();
            row.insert_cell(0, Cell::new(&id));
            table.add_row(row);

            table
        })
}
