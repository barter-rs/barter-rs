pub trait ConnectErrorHandler<Err> {
    fn handle(&mut self, error: &ConnectError<Err>) -> ConnectErrorAction;
}

impl<Err, F> ConnectErrorHandler<Err> for F
where
    F: FnMut(&ConnectError<Err>) -> ConnectErrorAction,
{
    #[inline]
    fn handle(&mut self, error: &ConnectError<Err>) -> ConnectErrorAction {
        self(error)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ConnectError<ErrConnect> {
    pub reconnection_attempt: u32,
    pub kind: ConnectErrorKind<ErrConnect>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConnectErrorKind<ErrConnect> {
    Connect(ErrConnect),
    Timeout,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConnectErrorAction {
    Reconnect,
    Terminate,
}
