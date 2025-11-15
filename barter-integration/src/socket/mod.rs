

pub mod reconnecting;

// Todo: Layer Transformations
//  1. Establish connection, Stream<Item = ProtocolMessage>
//  2. Transform ProtocolMessage -> enum { Protocol, Application }
//  3. Transform Application -> enum { Heartbeat, Response, Event }