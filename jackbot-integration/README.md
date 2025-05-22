# Jackbot-Integration

High-performance, low-level framework for composing flexible web integrations. 

Utilised by other [`Jackbot`] trading ecosystem crates to build robust financial exchange integrations,
primarily for public data collection & trade execution. It is:
* **Low-Level**: Translates raw data streams communicated over the web into any desired data model using arbitrary data transformations.
* **Flexible**: Compatible with any protocol (WebSocket, FIX, Http, etc.), any input/output model, and any user defined transformations.

Core abstractions include:
- **RestClient** providing configurable signed Http communication between client & server.
- **ExchangeStream** providing configurable communication over any asynchronous stream protocols (WebSocket, FIX, etc.).

Both core abstractions provide the robust glue you need to conveniently translate between server & client data models.

## RestClient
**(sync private & public Http communication)**

At a high level, a `RestClient` is has a few major components that allow it to execute `RestRequests`:
* `RequestSigner` with configurable signing logic on the target API.
* `HttpParser` that translates API specific responses into the desired output types.

## ExchangeStream
**(async communication using streaming protocols such as WebSocket and FIX)**

At a high level, an `ExchangeStream` is made up of a few major components:
* Inner Stream/Sink socket (eg/ WebSocket, FIX, etc).
* StreamParser that is capable of parsing input protocol messages (eg/ WebSocket, FIX, etc.) as exchange
  specific messages.
* Transformer that transforms from exchange specific message into an iterator of the desired outputs type.

## Roadmap
* Add new default StreamParser implementations to enable integration with redis and s3 (parquet + iceberg)
