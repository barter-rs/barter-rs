use crate::engine::Message;
use crate::Market;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Commander;

impl Commander {

    pub fn terminate_traders(&self, message: Message) {

    }

    pub fn exit_position(&self, market: Market) {
        // Determine Trader to send it to using Market, since EngineId is latently known from
        // creation of Engine - Trader channel relationships -> only this Engine has access to this
        // these Traders so they are linked by EngineId

        // Require position Id
        // Require market
    }

    pub fn exit_all_positions(&self) {

    }



}