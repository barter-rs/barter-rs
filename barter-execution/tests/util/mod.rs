use barter_execution::{
    model::{
        balance::Balance,
        order::{Cancelled, Open, Order, OrderId, OrderKind, RequestCancel, RequestOpen},
        AccountEvent, ClientOrderId,
    },
    simulated::{
        exchange::{
            account::{balance::ClientBalances, ClientAccount},
            SimulatedExchange,
        },
        SimulatedEvent,
    },
    ExecutionId,
};
use barter_integration::model::{
    instrument::{kind::InstrumentKind, symbol::Symbol, Instrument},
    Exchange, Side,
};
use std::{collections::HashMap, time::Duration};
use tokio::sync::mpsc;

pub(super) async fn run_default_exchange(
    event_account_tx: mpsc::UnboundedSender<AccountEvent>,
    event_simulated_rx: mpsc::UnboundedReceiver<SimulatedEvent>,
) {
    // Define SimulatedExchange available Instruments
    let instruments = instruments();

    // Create initial ClientAccount balances (Symbols must all be included in the Instruments)
    let balances = initial_balances();

    // Build SimulatedExchange & run on it's own Tokio task
    SimulatedExchange::builder()
        .event_simulated_rx(event_simulated_rx)
        .account(
            ClientAccount::builder()
                .latency(latency_50ms())
                .fees_percent(fees_50_percent())
                .event_account_tx(event_account_tx)
                .instruments(instruments)
                .balances(balances)
                .build()
                .expect("failed to build ClientAccount"),
        )
        .build()
        .expect("failed to build SimulatedExchange")
        .run()
        .await
}

pub(super) fn latency_50ms() -> Duration {
    Duration::from_millis(50)
}

pub(super) fn fees_50_percent() -> f64 {
    0.5
}

// Instruments that the SimulatedExchange supports
pub(super) fn instruments() -> Vec<Instrument> {
    vec![Instrument::from(("btc", "usdt", InstrumentKind::Perpetual))]
}

// Initial SimulatedExchange ClientAccount balances for each Symbol
pub(super) fn initial_balances() -> ClientBalances {
    ClientBalances(HashMap::from([
        (Symbol::from("btc"), Balance::new(10.0, 10.0)),
        (Symbol::from("usdt"), Balance::new(10_000.0, 10_000.0)),
    ]))
}

// Utility for creating an Open Order request
pub(super) fn order_request_limit<I>(
    instrument: I,
    cid: ClientOrderId,
    side: Side,
    price: f64,
    quantity: f64,
) -> Order<RequestOpen>
where
    I: Into<Instrument>,
{
    Order {
        exchange: Exchange::from(ExecutionId::Simulated),
        instrument: instrument.into(),
        cid,
        side,
        state: RequestOpen {
            kind: OrderKind::Limit,
            price,
            quantity,
        },
    }
}

// Utility for creating an Open Order
pub(super) fn open_order<I>(
    instrument: I,
    cid: ClientOrderId,
    id: OrderId,
    side: Side,
    price: f64,
    quantity: f64,
    filled: f64,
) -> Order<Open>
where
    I: Into<Instrument>,
{
    Order {
        exchange: Exchange::from(ExecutionId::Simulated),
        instrument: instrument.into(),
        cid,
        side,
        state: Open {
            id,
            price,
            quantity,
            filled_quantity: filled,
        },
    }
}

// Utility for creating an Order RequestCancel
pub(super) fn order_cancel_request<I, Id>(
    instrument: I,
    cid: ClientOrderId,
    side: Side,
    id: Id,
) -> Order<RequestCancel>
where
    I: Into<Instrument>,
    Id: Into<OrderId>,
{
    Order {
        exchange: Exchange::from(ExecutionId::Simulated),
        instrument: instrument.into(),
        cid,
        side,
        state: RequestCancel::from(id),
    }
}

// Utility for creating an Order<Cancelled>
pub(super) fn order_cancelled<I, Id>(
    instrument: I,
    cid: ClientOrderId,
    side: Side,
    id: Id,
) -> Order<Cancelled>
where
    I: Into<Instrument>,
    Id: Into<OrderId>,
{
    Order {
        exchange: Exchange::from(ExecutionId::Simulated),
        instrument: instrument.into(),
        cid,
        side,
        state: Cancelled::from(id),
    }
}
