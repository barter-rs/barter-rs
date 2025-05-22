use jackbot_data::exchange::{
    bybit::spot::BybitSpot,
    coinbase::Coinbase,
    kraken::Kraken,
    kucoin::Kucoin,
    okx::Okx,
    hyperliquid::Hyperliquid,
    Connector,
    DEFAULT_HEARTBEAT_INTERVAL,
};

#[test]
fn test_exchange_heartbeat_intervals() {
    assert_eq!(BybitSpot::heartbeat_interval(), Some(DEFAULT_HEARTBEAT_INTERVAL));
    assert_eq!(Coinbase::heartbeat_interval(), Some(DEFAULT_HEARTBEAT_INTERVAL));
    assert_eq!(Kraken::heartbeat_interval(), Some(DEFAULT_HEARTBEAT_INTERVAL));
    assert_eq!(Kucoin::heartbeat_interval(), Some(DEFAULT_HEARTBEAT_INTERVAL));
    assert_eq!(Okx::heartbeat_interval(), Some(DEFAULT_HEARTBEAT_INTERVAL));
    assert_eq!(Hyperliquid::heartbeat_interval(), Some(DEFAULT_HEARTBEAT_INTERVAL));
}
