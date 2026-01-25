use crate::{
    Auditable, Processor, Tick,
    run::SyncDriver,
    sequence::{Sequence, Sequenced},
};
use futures::{Stream, StreamExt};
use tracing::info;

// Todo: consider constraining "Processor" to produce (Audit, SideEffects)
//   --> enables type driven decoupling from State to enable ReadReplica more easily
//


pub struct Replica<Process, Context> {
    processor: Process,
    context: Sequenced<Context>,
}

pub struct SequenceError<Context> {
    current: Sequenced<Context>,
    next: Sequenced<Context>,
}

impl<Event, Process> Processor<Tick<Event, Sequenced<Process::Context>>>
    for Replica<Process, Process::Context>
where
    Process: Processor<Event> + Auditable<Event>,
{
    type Audit = Result<
        Tick<Process::Snapshot, Sequenced<Process::Context>>,
        SequenceError<Process::Context>,
    >;

    fn process(&mut self, event: Tick<Event, Sequenced<Process::Context>>) -> Self::Audit {
        self.validate_sequence(&event.meta.sequence)?;
        self.context = event.meta;

        let _audit = self.processor.process(event.kind);

        Tick {
            kind: self.processor.snapshot().clone(),
            meta: self.context.clone(),
        }
    }
}

impl<Process, Context> Replica<Process, Context> {
    fn validate_sequence<Event>(
        &mut self,
        next: &Sequenced<Process::Context>,
    ) -> Result<(), SequenceError<Context>>
    where
        Process: Auditable<Event>,
    {
        if self.context.sequence.value() == next.value() - 1 {
            Ok(())
        } else {
            Err(SequenceError {
                current: self.context.clone(),
                next: next.clone(),
            })
        }
    }
}

// Todo: General query:
//  - How would this lack of "output" paradigm work with Barter StateReplicaManager?
//    '--> eg/ output orders etc, do we need a different pipeline for that?
//    '--> current Barter one just has "State" to update, and it's all hard-coded

// Todo: Idea:
//  - What if the Engine had zero side-affects, and instead returns:
//  ->> (Audit + SideEffects)
//  ->>> Then some other components puts the Engines SideEffects into action
//  ->>>> That way Input + Engine is all we need to replicate state, with no side effects
//  ->>>>> may simplify Engine audit flow too!

// Todo:
//  - this is too much link regular async_run -> that's good it's just a different Procesor
//   -> Can do the ready_chunk(capacity) before the impl Stream is passed in here
pub async fn async_run_with_ready_chunks<Events, Process>(
    events: &mut Events,
    processor: &mut Process,
    chunk_capacity: usize,
    fn_snapshot: impl AsyncFnMut(Tick<Process::Snapshot, Sequenced<Process::Context>>),
) where
    Events: Stream,
    Process: Processor<Events::Item> + Auditable<Events::Item>,
{
    info!(
        feed_mode = "async_ready_chunks",
        processor = processor.name(),
        context = processor.context(),
        chunk_capacity,
        "Processor running"
    );

    // Todo: don't really want to Send snapshots, watch channel better:
    //  clients can have Tick<Snapshot>, and they know if they've missed event due to Tick
    //  --> how do I not have to clone current snapshot to put in the channel without lock

    let mut events = events.ready_chunks(chunk_capacity);

    // Todo: how to note allocate the Vec every .next()?
    while let Some(chunk) = events.next().await {
        chunk.into_iter().map(|event| {
            let x = processor.process(event);
        });
    }

    info!(
        feed_mode = "async_ready_chunks",
        processor = processor.name(),
        context = processor.context(),
        chunk_capacity,
        "Replica<Processor> running"
    );
}
