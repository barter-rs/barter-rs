use crate::protocol::websocket::WsSink;

pub trait AsyncTransformer {
    type Input;
    type Output;
    fn transform(&mut self, input: Self::Input) -> impl Future<Output = Self::Output>;
}

pub struct WebSocketManager {
    sink: WsSink, // Pretend we have a "unified Sink" for now
}
pub struct Subscription;

pub enum WebSocketManagerInput {
    Tick, // For now, just merge input Stream with IntervalStream<1 second> for easy wakeup
    Command(SubscriptionCommand),
    Pong(u64),
    Response(Result<Subscription, Subscription>),
}

pub enum SubscriptionCommand {
    Subscribe(Subscription),
    Unsubscribe(Subscription),
}

pub struct WebSocketManagerOutput;

impl AsyncTransformer for WebSocketManager {
    type Input = WebSocketManagerInput;
    type Output = WebSocketManagerOutput;

    async fn transform(&mut self, input: Self::Input) -> Self::Output {
        match input {
            WebSocketManagerInput::Tick => {
                // Check for request-response timeouts (eg/ SubRequest, or PingPong, etc).
            }
            WebSocketManagerInput::Command(SubscriptionCommand::Subscribe(_sub)) => {}
            WebSocketManagerInput::Command(SubscriptionCommand::Unsubscribe(_sub)) => {}
            WebSocketManagerInput::Pong(_) => {}
            WebSocketManagerInput::Response(_) => {}
        }

        todo!()
    }
}

// fn run<St, Transformer>(
//     stream: St,
//     transformer: Transformer,
// ) -> impl Stream<Item = Transformer::Output>
// where
//     St: Stream,
//     Transformer: AsyncTransformer
// {
//     stream
//         .scan(transformer, |transformer, input| Some(transformer.transform(input)))
// }
