pub mod transform;

use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};
use async_trait::async_trait;
use barter_instrument::Side;
use databento::{dbn::Side as DbSide, live::Client};
use databento::dbn::PitSymbolMap;
use futures::{pin_mut, Stream, TryFuture};
use pin_project::pin_project;
use tracing::error;
use barter_instrument::instrument::market_data::MarketDataInstrument;
use crate::error::DataError;
use crate::event::DataKind;
use crate::provider::databento::transform::transform;
use crate::provider::Provider;
use crate::streams::consumer::MarketStreamResult;
use crate::streams::reconnect::Event;

#[derive(Debug)]
#[pin_project]
pub struct DatabentoProvider {
    #[pin]
    client: Client,
    initialised: bool,
    buffer: VecDeque<MarketStreamResult<MarketDataInstrument, DataKind>>,
    symbol_map: PitSymbolMap
}

impl DatabentoProvider {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            initialised: false,
            buffer: VecDeque::new(),
            symbol_map: PitSymbolMap::new()
        }
    }
}

impl Stream for DatabentoProvider {
    type Item = MarketStreamResult<MarketDataInstrument, DataKind>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            if let Some(output) = this.buffer.pop_front() {
                return Poll::Ready(Some(MarketStreamResult::from(output)));
            }

            let future = this.client.next_record();
            pin_mut!(future);

            let input = match future.try_poll(cx) {
                Poll::Ready(Ok(Some(record_ref))) => {
                    let _ = this.symbol_map.on_record(record_ref);
                    transform(record_ref, &this.symbol_map)
                },
                Poll::Pending => return Poll::Pending,
                _ => return Poll::Ready(None)
            };

            if let Err(e) = input {
                error!("Error processing record: {:?}", e);
                continue;
            }

            match input.unwrap() {
                Some(event) => {
                    this.buffer.push_back(Event::Item(Ok(event)));
                },
                None => {
                    continue;
                }
            }
        }
    }
}

#[async_trait]
impl Provider for DatabentoProvider {
    async fn init(&mut self) -> Result<(), DataError> {
        match self.client.start().await {
            Ok(_) => {
                self.initialised = true;
                Ok(())
            }
            Err(e) => Err(DataError::from(e)),
        }
    }
}

#[derive(Debug)]
pub enum DatabentoSide {
    Buy,
    Sell,
    None
}


impl From<DbSide> for DatabentoSide {
    fn from(value: DbSide) -> Self {
        match value {
            DbSide::Bid => DatabentoSide::Buy,
            DbSide::Ask => DatabentoSide::Sell,
            _ => DatabentoSide::None
        }
    }
}

impl From<DatabentoSide> for Side {
    fn from(value: DatabentoSide) -> Self {
        match value {
            DatabentoSide::Buy => Side::Buy,
            DatabentoSide::Sell => Side::Sell,
            DatabentoSide::None => {
                panic!("Cannot convert DatabentoSide::None to Side")
            }
        }
    }
}