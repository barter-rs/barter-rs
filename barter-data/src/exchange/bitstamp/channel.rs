use crate::{
    Identifier,
    exchange::bitstamp::BitstampSpot,
    subscription::{Subscription, book::OrderBooksL2},
};

#[derive(Debug, Clone, Copy)]
pub struct BitstampChannel(pub &'static str);

impl BitstampChannel {
    pub const ORDER_BOOK_L2: Self = Self("diff_order_book_");
}

impl<Instrument> Identifier<BitstampChannel>
    for Subscription<BitstampSpot, Instrument, OrderBooksL2>
{
    fn id(&self) -> BitstampChannel {
        BitstampChannel::ORDER_BOOK_L2
    }
}

impl AsRef<str> for BitstampChannel {
    fn as_ref(&self) -> &str {
        self.0
    }
}
