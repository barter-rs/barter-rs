use std::collections::HashMap;
use crate::portfolio::repository::redis::{PositionHandler, ValueHandler, CashHandler};
use uuid::Uuid;


pub struct DefaultStatistic<'a, T> where T: PositionHandler + ValueHandler + CashHandler {
    repository: &'a mut T,
}

// long count, short count, long average %, short average %,
// average profit %, cum long %, cum short %, cum profit %, total profit <denomination>, avg duration trade

// impl<T> DefaultStatistic<'_, T> where T: PositionHandler + ValueHandler + CashHandler {
//     fn count_long_positions(&mut self) {
//         self.repository.get(&Uuid::new_v4());
//     }
// }

// Todo: Impl repository getClosedPositions() & getOpenPositions() to aid backtest/trade performance
//   review at the end
