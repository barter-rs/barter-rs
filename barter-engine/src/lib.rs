use futures::Stream;

pub mod replica;
pub mod run;
pub mod sequence;

pub trait Processor<Event> {
    type Audit;
    fn process(&mut self, event: Event) -> Self::Audit;
}

pub trait Auditable<Event>
where
    Self: Processor<Event>,
{
    type Snapshot: Clone;
    type Context: Clone;

    fn name(&self) -> &str;
    fn snapshot(&self) -> &Self::Snapshot;
    fn context(&self) -> &Self::Context;
    fn running(&self) -> bool;
}

pub struct Tick<Kind, Meta> {
    pub kind: Kind,
    pub meta: Meta,
}
