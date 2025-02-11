use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};
use databento::dbn::{ErrorMsg, MboMsg, SymbolMappingMsg};
use databento::live::Client;
use databento::LiveClient;
use futures::{pin_mut, Stream, TryFuture};
use thiserror::Error;
use pin_project::pin_project;

#[derive(Debug)]
#[pin_project]
pub struct DatabentoStream {
    client: Client
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

impl Stream for DatabentoStream {
    type Item = Result<DatabentoMessage, DatabentoError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().project();
        let client = this.client;
        let future = client.next_record();
        pin_mut!(future);

        match future.try_poll(cx) {
            Poll::Ready(Ok(Some(record))) => {
                let output = if let Some(mb0) = record.get::<MboMsg>() {
                    dbg!("test");
                    Ok(DatabentoMessage::Mbo(mb0.clone()))
                } else if let Some(sym) = record.get::<SymbolMappingMsg>() {
                    Ok(DatabentoMessage::SymbolMapping(sym.clone()))
                } else if let Some(_error) = record.get::<ErrorMsg>() {
                    Err(DatabentoError::InvalidRecord)
                } else {
                    Err(DatabentoError::InvalidRecord)
                };
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