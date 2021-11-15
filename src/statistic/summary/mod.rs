pub mod data;
pub mod drawdown;
pub mod pnl;
pub mod trading;

use crate::portfolio::position::Position;

pub trait PositionSummariser {
    fn update(&mut self, position: &Position);
    fn generate_summary(&mut self, positions: &Vec<Position>) {
        for position in positions.iter() {
            self.update(position)
        }
    }
}

pub trait TablePrinter {
    fn print(&self);
}
