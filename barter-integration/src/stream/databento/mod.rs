use std::fmt::Debug;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use databento::dbn::{ErrorMsg, MboMsg, RecordRef, SymbolMappingMsg, SystemMsg};
use databento::live::Client;
use databento::LiveClient;
use databento::error::Error as DBError;
use futures::{pin_mut, Stream, TryFuture};
use thiserror::Error;
use pin_project::pin_project;
#[derive(Debug)]
#[pin_project]
pub struct DatabentoStream {
    client: Client,
}

#[derive(Debug)]
pub enum DatabentoMessage {
    SymbolMapping(SymbolMappingMsg),
    Mbo(MboMsg),
}

#[derive(Debug, Error)]
pub enum DatabentoError {
    #[error("Unmatched record type")]
    InvalidRecord,
}

#[derive(Debug, Default)]
pub struct DatabentoTransformer<'a> {
    misc: PhantomData<&'a ()>,
}

pub trait DBTransformer {
    type Error;
    type Input;
    type Output;
    type OutputIter: IntoIterator<Item = Result<Self::Output, Self::Error>>;
    fn transform(&mut self, input: Self::Input) -> Self::OutputIter;
}


impl <'a> DatabentoTransformer<'_> {

    fn transform(&mut self, input: RecordRef) -> Result<DatabentoMessage, DatabentoError> {
        // Most frequent message type
        if let Some(mb0) = input.get::<MboMsg>() {
            return Ok(DatabentoMessage::Mbo(mb0.clone()))
        } else if let Some(sym) = input.get::<SymbolMappingMsg>() {
            return Ok(DatabentoMessage::SymbolMapping(sym.clone()))
        } else if let Some(_error) = input.get::<ErrorMsg>() {
            return Err(DatabentoError::InvalidRecord)
        } else if let Some(system) = input.get::<SystemMsg>() {
            eprintln!("{}", system.msg().expect("System message is empty"));
        }

        Err(DatabentoError::InvalidRecord)
    }
}

impl Stream for DatabentoStream {
    type Item = Result<DatabentoMessage, DatabentoError>;

    fn poll_next(self: Pin<&mut Self>,cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let mut client = this.client;
        let future = client.next_record();
        pin_mut!(future);

        match future.as_mut().try_poll(cx) {
            Poll::Ready(Ok(Some(record))) => {
                let mut transformer = DatabentoTransformer::default();
                let output = transformer.transform(record);
                Poll::Ready(Some(output))
            },
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(_)) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}


impl DatabentoStream {
    pub fn new(
        client: LiveClient
    ) -> Self {

        Self {
            client
        }
    }
}