pub mod data;
pub mod drawdown;
pub mod pnl;
pub mod trading;

use std::mem;
use crate::portfolio::position::Position;
use prettytable::{AsTableSlice, Cell, Row, Table};
use prettytable::format::TableFormat;

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

/// A table with public titles.
///
/// This struct is used to keep track of a table's format, titles, and rows.
/// It is used to create a new table with a transposed format.
#[derive(Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TableWithPubTitles {
    /// The format of the table.
    pub format: Box<TableFormat>,
    /// The titles of the table.
    pub titles: Box<Option<Row>>,
    /// The rows of the table.
    pub rows: Vec<Row>,
}

/// Transmutes a `Table` into a `TableWithPubTitles`.
///
/// # Safety
/// This function is unsafe because it transmutes a `Table` into a `TableWithPubTitles`.
/// It should only be used when the `Table` is known to have the correct format.
unsafe fn transmute_table(table: Table) -> TableWithPubTitles {
    mem::transmute(table)
}

/// Transposes a table.
///
/// # Arguments
///
/// * `in_table` - The table to be transposed.
///
/// # Returns
///
/// A new table with transposed data.
pub fn transpose(in_table: Table) -> Table {
    let mut modified_table = Table::new();
    let table_source: Table = in_table.clone();
    let table_clone: Table = in_table.clone();
    let table_with_pub_titles: TableWithPubTitles = unsafe { transmute_table(table_source) };

    let table = table_clone.as_slice();
    let titles = table_with_pub_titles.titles;

    for title in titles.iter() {
        for (row_index, (index, title_cell)) in title.iter().enumerate().enumerate() {
            let mut new_row = Row::new(vec![]);
            if row_index == 0 {
                new_row.add_cell(Cell::new("#"));
            } else {
                new_row.add_cell(Cell::new(&row_index.to_string()));
            }
            if index == 0 {
                new_row.add_cell(Cell::new("Metric"));
            } else {
                new_row.add_cell(Cell::new(&title_cell.get_content()));
            }
            for col in table.column_iter(row_index) {
                new_row.add_cell(Cell::new(&col.get_content()));
            }
            modified_table.add_row(new_row);
        }
    }
    modified_table
}