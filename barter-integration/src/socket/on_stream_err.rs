pub trait StreamErrorHandler<Err> {
    fn handle(&mut self, error: &Err) -> StreamErrorAction;
}

impl<Err, F> StreamErrorHandler<Err> for F
where
    F: FnMut(&Err) -> StreamErrorAction,
{
    #[inline]
    fn handle(&mut self, error: &Err) -> StreamErrorAction {
        self(error)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StreamErrorAction {
    Continue,
    Reconnect,
}
