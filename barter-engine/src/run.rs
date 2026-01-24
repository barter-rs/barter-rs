use crate::{
    Auditable, Processor, Tick,
    sequence::{Sequence, Sequenced},
};
use derive_more::Constructor;
use futures::Stream;
use tracing::info;

enum Termination<Meta> {
    EventsEnded,
    ProcessorEnded(Meta),
}

enum ProcessingState<Audit> {
    Processed(Audit),
    Terminated(Audit),
}

// Todo: not even sure this is worth it, maybe just have let terminated = bool;
#[derive(Constructor)]
pub struct SyncDriver<'a, Events, Process> {
    events: &'a mut Events,
    processor: &'a mut Process,
    sequence: &'a mut Sequence,
    terminated: bool,
}

impl<'a, Events, Process> SyncDriver<'a, Events, Process> {
    pub fn events(&self) -> &Events {
        self.events
    }

    pub fn processor(&self) -> &Process {
        self.processor
    }

    pub fn sequence(&self) -> &Sequence {
        self.sequence
    }

    pub fn terminated(&self) -> bool {
        self.terminated
    }
}

impl<'a, Events, Process> Iterator for SyncDriver<'a, Events, Process>
where
    Events: Iterator,
    Process: Processor<Events::Item> + Auditable<Events::Item>,
{
    type Item = Tick<Process::Audit, Sequenced<Process::Context>>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.processor.running() {
            return None;
        }

        let event = self.events.next()?;

        let audit = self.processor.process(event);

        let tick = Tick {
            kind,
            meta: Sequenced {
                value: audit,
                sequence: self.sequence.fetch_add(),
            },
        };

        Some(tick)
    }
}

// Todo: This is perhaps perfect and fits with current Barter engine
pub fn sync_run<Events, Process>(
    events: &mut Events,
    processor: &mut Process,
    sequence: &mut Sequence,
    fn_audit: impl FnMut(Tick<Process::Audit, Sequenced<Process::Context>>),
) where
    Events: Iterator,
    Process: Processor<Events::Item> + Auditable<Events::Item>,
{
    info!(
        feed_mode = "sync",
        processor = processor.name(),
        context = processor.context(),
        %sequence,
        "Processor running"
    );

    SyncDriver::new(events, processor, sequence, false).for_each(fn_audit);

    info!(
        feed_mode = "sync",
        processor = processor.name(),
        context = processor.context(),
        %sequence,
        "Processor stopped running"
    );
}

// pub async fn async_run<Events, Process>(
//     events: &mut Events,
//     processor: &mut Process,
//     auditor: &mut Auditor,
//     fn_audit: impl AsyncFnMut(AuditTick<Process::Audit, Process::Context>),
// )
// where
//     Events: Stream,
//     Process: Processor<Events::Item> + Auditable<Events::Item>,
// {
//     info!(
//         feed_mode = "async",
//         processor = processor.name(),
//         context = processor.context(),
//         "Processor running"
//     );
//
//     loop {
//
//         let Some(event) = events.next().await else {
//             break Termination::EventsEnded
//         };
//
//         let audit = processor
//             .process(event);
//
//         let audit_tick = auditor
//             .tick(audit, processor.context());
//     }
//
//     info!(
//         feed_mode = "async",
//         processor = processor.name(),
//         context = processor.context(),
//         "Processor stopped running"
//     );
//
//
// }
