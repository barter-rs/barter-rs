// Todo: Open Questions:
//  - How can each SingleBookManager "own" the input Stream, so it can restart it etc.
//  - Or Perhaps one BookManager per Exchange/Connection?
//     OR can have a SnapshotTransformer that fetches an initial snapshot from the exchange...
//          '--> leaning towards this

// use std::future::Future;
// use std::pin::Pin;
// use std::sync::Arc;
// use fnv::FnvHashMap;
// use parking_lot::RwLock;
// use barter_integration::error::SocketError;
// use barter_integration::model::Exchange;
// use barter_integration::Validator;
// use crate::books::OrderBook;
// use crate::error::DataError;
// use crate::exchange::StreamSelector;
// use crate::Identifier;
// use crate::instrument::InstrumentData;
// use crate::streams::consumer::{init_market_stream, STREAM_RECONNECTION_POLICY};
// use crate::subscription::book::{OrderBooksL2};
// use crate::subscription::Subscription;
//
// type SubscribeL2Future<St> = Pin<Box<dyn Future<Output = Result<St, DataError>>>>;
//
// pub struct OrderBookBuilder<InstrumentKey> {
//     pub futures: Vec<SubscribeL2Future<InstrumentKey>>,
// }
//
// impl<St> OrderBookBuilder<St> {
//     pub fn subscribe<SubIter, Sub, Exchange, Instrument>(
//         mut self,
//         subscriptions: SubIter,
//     ) -> Self
//     where
//         SubIter: IntoIterator<Item = Sub>,
//         Sub: Into<Subscription<Exchange, Instrument, OrderBooksL2>>,
//         Exchange: StreamSelector<Instrument, OrderBooksL2>,
//         Instrument: InstrumentData,
//         Subscription<Exchange, Instrument, OrderBooksL2>: Identifier<Exchange::Channel> + Identifier<Exchange::Market>,
//     {
//         self.futures.push(Box::pin(async move {
//             // Validate & dedup Subscriptions
//             let mut subscriptions = validate(subscriptions)?;
//
//             init_market_stream(
//                 STREAM_RECONNECTION_POLICY,
//                 subscriptions
//             ).await
//         }));
//
//         self
//     }
//
//
// }
//
// fn validate<SubIter, Sub, Exchange, Instrument, Kind>(
//     subscriptions: SubIter
// ) -> Result<Vec<Subscription<Exchange, Instrument, Kind>>, DataError>
// where
//     SubIter: IntoIterator<Item = Sub>,
//     Sub: Into<Subscription<Exchange, Instrument, OrderBooksL2>>,
//     Instrument: InstrumentData,
// {
//     // Validate Subscription is supported
//     let mut subscriptions = subscriptions
//         .into_iter()
//         .map(Sub::into)
//         .map(Subscription::validate)
//         .collect::<Result<Vec<_>, SocketError>>()?;
//
//     if subscriptions.is_empty() {
//         return Err(DataError::SubscriptionsEmpty)
//     }
//
//     // Remove duplicate Subscriptions
//     subscriptions.sort();
//     subscriptions.dedup();
//
//     Ok(subscriptions)
// }
//
// pub struct OrderBookManager<InstrumentKey> {
//     pub books: FnvHashMap<InstrumentKey, SingleBookManager>,
// }
//
// // impl<InstrumentKey> OrderBookManager<InstrumentKey> {
// //     pub fn run<St>(self, mut stream: St)
// //     where
// //         St: Stream<Item = MarketEvent<InstrumentKey, OrderBookEvent>>
// //     {
// //
// //     }
// // }
//
// pub struct SingleBookManager {
//     pub exchange: Exchange,
//     pub book: Arc<RwLock<OrderBook>>,
// }
//
//
// pub async fn manage_order_books<BookMap, St, InstrumentKey>(books: BookMap, mut stream: St)
// where
//     BookMap: OrderBookMap<Key = InstrumentKey>,
//     St: Stream<Item = MarketEvent<InstrumentKey, OrderBookEvent>> + Unpin,
//     InstrumentKey: Debug,
// {
//     while let Some(event) = stream.next().await {
//         let Some(book) = books.find(&event.instrument) else {
//             warn!(
//                 instrument = ?event.instrument,
//                 "consumed MarketEvent<OrderBookEvent> for non-configured instrument"
//             );
//             continue;
//         };
//
//         // Todo: consider OrderBook<Kind> for Snapshot & Update { meta }
//
//         match event.kind {
//             OrderBookEvent::Snapshot(snapshot) => {
//                 *book.write() = snapshot;
//             }
//             OrderBookEvent::Update(update) => {
//                 let OrderBook {
//                     sequence,
//                     time_engine,
//                     bids,
//                     asks,
//                 } = update;
//             }
//         }
//     }
// }
