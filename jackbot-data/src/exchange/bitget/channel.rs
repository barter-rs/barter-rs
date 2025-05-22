//! Channel definitions for Bitget exchange.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BitgetChannel(pub &'static str);

impl BitgetChannel {
    /// Bitget real-time OrderBook Level1 (top of book) channel name.
    /// See: https://www.bitget.com/api-doc/spot/websocket/public/depth1-channel
    pub const ORDER_BOOK_L1: Self = Self("depth1");
    /// Bitget real-time OrderBook Level2 (full book) channel name.
    /// See: https://www.bitget.com/api-doc/spot/websocket/public/depth-channel
    pub const ORDER_BOOK_L2: Self = Self("books");
    /// Bitget real-time trades channel name.
    /// See: https://www.bitget.com/api-doc/spot/websocket/public/trade-channel
    pub const TRADES: Self = Self("trade");
    /// Bitget real-time ticker channel name.
    /// See: https://www.bitget.com/api-doc/spot/websocket/public/ticker-channel
    pub const TICKER: Self = Self("ticker");
    /// Bitget real-time candlestick channel name.
    /// See: https://www.bitget.com/api-doc/spot/websocket/public/candle-channel
    pub const CANDLE: Self = Self("candle");

    pub fn as_str(&self) -> &str {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_channel_consts() {
        assert_eq!(BitgetChannel::ORDER_BOOK_L1.as_str(), "depth1");
        assert_eq!(BitgetChannel::ORDER_BOOK_L2.as_str(), "books");
        assert_eq!(BitgetChannel::TRADES.as_str(), "trade");
        assert_eq!(BitgetChannel::TICKER.as_str(), "ticker");
        assert_eq!(BitgetChannel::CANDLE.as_str(), "candle");
    }
}
