use crate::v2::engine::state::order_manager::Orders;
use crate::v2::execution::InstrumentAccountSnapshot;
use crate::v2::instrument::Instrument;
use crate::v2::order::{Open, Order};
use crate::v2::position::Position;
use crate::v2::Snapshot;
use crate::v2::trade::Trade;

#[derive(Debug)]
pub struct InstrumentState<AssetKey, InstrumentKey, Market> {
    pub instrument: Instrument<AssetKey>,
    pub position: Position<InstrumentKey>,
    pub orders: Orders<InstrumentKey>,
    pub market: Market,
}

impl<AssetKey, InstrumentKey, Market> InstrumentState<AssetKey, InstrumentKey, Market> 
where
    InstrumentKey: Clone,
{
    pub fn update_from_account_snapshot(
        &mut self, 
        snapshot: &InstrumentAccountSnapshot<InstrumentKey>
    ) {
        self.update_from_position_snapshot(Snapshot(&snapshot.position));
        self.update_from_opens_snapshot(Snapshot(&snapshot.orders))
    }
    
    pub fn update_from_position_snapshot(&mut self, position: Snapshot<&Position<InstrumentKey>>) {
        let _ = std::mem::replace(&mut self.position, position.0.clone());
    }

    pub fn update_from_trade(&mut self, _trade: &Trade<AssetKey, InstrumentKey>) {
        todo!()
    }
    
    pub fn update_from_opens_snapshot(&mut self, orders: Snapshot<&Vec<Order<InstrumentKey, Open>>>) {
        let _ = std::mem::replace(
            &mut self.orders.0,
            orders
                .0
                .iter()
                .map(|order| (order.cid, Order::from(order.clone())))
                .collect()
        );
    }
}

