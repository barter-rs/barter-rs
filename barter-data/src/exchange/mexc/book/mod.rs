use crate::{
    books::Level,
    error::DataError,
    event::{MarketEvent, MarketIter},
    subscription::book::OrderBookL1,
};
use barter_instrument::exchange::ExchangeId;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use super::trade::proto;

fn ms_epoch_to_datetime_utc(ms: i64) -> Result<DateTime<Utc>, DataError> {
    if ms < 0 {
        return Err(DataError::Socket(format!(
            "Unsupported MexcBookTicker::Timestamp: invalid unix_epoch_ms (negative): {}",
            ms
        )));
    }
    DateTime::from_timestamp_millis(ms).ok_or_else(|| {
        DataError::Socket(format!(
            "Unsupported MexcBookTicker::Timestamp: invalid unix_epoch_ms: {}",
            ms
        ))
    })
}

fn parse_level(price: &str, qty: &str) -> Result<Level, DataError> {
    let price = price.parse::<Decimal>().map_err(|e| {
        DataError::Socket(format!(
            "Failed to parse price from MEXC agg book ticker: '{}', error: {}",
            price, e
        ))
    })?;
    let amount = qty.parse::<Decimal>().map_err(|e| {
        DataError::Socket(format!(
            "Failed to parse quantity from MEXC agg book ticker: '{}', error: {}",
            qty, e
        ))
    })?;
    Ok(Level::new(price, amount))
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, proto::PushDataV3ApiWrapper)>
    for MarketIter<InstrumentKey, OrderBookL1>
where
    InstrumentKey: Clone,
{
    fn from(
        (exchange_id, instrument, wrapper): (
            ExchangeId,
            InstrumentKey,
            proto::PushDataV3ApiWrapper,
        ),
    ) -> Self {
        let time_received = Utc::now();
        if let Some(proto::push_data_v3_api_wrapper::Body::PublicAggreBookTicker(ticker)) =
            wrapper.body
        {
            let exchange_time = wrapper
                .send_time
                .or(wrapper.create_time)
                .and_then(|ms| ms_epoch_to_datetime_utc(ms).ok())
                .unwrap_or(time_received);

            let best_bid = match parse_level(&ticker.bid_price, &ticker.bid_quantity) {
                Ok(lvl) => Some(lvl),
                Err(err) => return Self(vec![Err(err)]),
            };
            let best_ask = match parse_level(&ticker.ask_price, &ticker.ask_quantity) {
                Ok(lvl) => Some(lvl),
                Err(err) => return Self(vec![Err(err)]),
            };

            return Self(vec![Ok(MarketEvent {
                time_exchange: exchange_time,
                time_received,
                exchange: exchange_id,
                instrument,
                kind: OrderBookL1 {
                    last_update_time: exchange_time,
                    best_bid,
                    best_ask,
                },
            })]);
        }
        Self(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Identifier;
    use barter_integration::de::datetime_utc_from_epoch_duration;
    use rust_decimal_macros::dec;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    struct TestInstrument {
        base: String,
        quote: String,
    }

    impl Identifier<String> for TestInstrument {
        fn id(&self) -> String {
            format!("{}{}", self.base, self.quote)
        }
    }

    #[test]
    fn test_public_aggre_book_ticker_into_order_book_l1() {
        let instrument = TestInstrument {
            base: "BTC".into(),
            quote: "USDT".into(),
        };

        let ticker = proto::PublicAggreBookTickerV3Api {
            bid_price: "50000.5".to_string(),
            bid_quantity: "0.1".to_string(),
            ask_price: "50001".to_string(),
            ask_quantity: "0.2".to_string(),
        };

        let wrapper = proto::PushDataV3ApiWrapper {
            channel: "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT".to_string(),
            symbol: Some("BTCUSDT".to_string()),
            symbol_id: Some("BTCUSDT_ID".to_string()),
            create_time: Some(1609459200000),
            send_time: Some(1609459200500),
            body: Some(proto::push_data_v3_api_wrapper::Body::PublicAggreBookTicker(ticker)),
        };

        let market_iter = MarketIter::<TestInstrument, OrderBookL1>::from((
            ExchangeId::Mexc,
            instrument.clone(),
            wrapper,
        ));

        let events: Vec<_> = market_iter
            .0
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.exchange, ExchangeId::Mexc);
        assert_eq!(event.instrument, instrument);

        let expected_time = datetime_utc_from_epoch_duration(Duration::from_millis(1609459200500));
        assert_eq!(event.time_exchange, expected_time);
        assert_eq!(event.kind.last_update_time, expected_time);
        assert_eq!(
            event.kind.best_bid,
            Some(Level::new(dec!(50000.5), dec!(0.1)))
        );
        assert_eq!(
            event.kind.best_ask,
            Some(Level::new(dec!(50001), dec!(0.2)))
        );
    }

    #[test]
    fn test_public_aggre_book_ticker_invalid_price() {
        let instrument = TestInstrument {
            base: "BTC".into(),
            quote: "USDT".into(),
        };

        let ticker = proto::PublicAggreBookTickerV3Api {
            bid_price: "not_a_decimal".to_string(),
            bid_quantity: "0.1".to_string(),
            ask_price: "50001".to_string(),
            ask_quantity: "0.2".to_string(),
        };

        let wrapper = proto::PushDataV3ApiWrapper {
            channel: "spot@public.aggre.bookTicker.v3.api.pb@100ms@BTCUSDT".to_string(),
            symbol: Some("BTCUSDT".to_string()),
            symbol_id: Some("BTCUSDT_ID".to_string()),
            create_time: Some(1609459200000),
            send_time: Some(1609459200500),
            body: Some(proto::push_data_v3_api_wrapper::Body::PublicAggreBookTicker(ticker)),
        };

        let events = MarketIter::<TestInstrument, OrderBookL1>::from((
            ExchangeId::Mexc,
            instrument,
            wrapper,
        ))
        .0;

        assert_eq!(events.len(), 1);
        match &events[0] {
            Err(DataError::Socket(s)) => assert!(s.contains("Failed to parse price")),
            other => panic!("Unexpected event: {other:?}"),
        }
    }

    #[test]
    fn test_public_aggre_book_ticker_invalid_quantity() {
        let instrument = TestInstrument {
            base: "ETH".into(),
            quote: "USDT".into(),
        };

        let ticker = proto::PublicAggreBookTickerV3Api {
            bid_price: "50000".to_string(),
            bid_quantity: "0.1".to_string(),
            ask_price: "50001".to_string(),
            ask_quantity: "bad_qty".to_string(),
        };

        let wrapper = proto::PushDataV3ApiWrapper {
            channel: "spot@public.aggre.bookTicker.v3.api.pb@100ms@ETHUSDT".to_string(),
            symbol: Some("ETHUSDT".to_string()),
            symbol_id: Some("ETHUSDT_ID".to_string()),
            create_time: Some(1609459200000),
            send_time: Some(1609459200500),
            body: Some(proto::push_data_v3_api_wrapper::Body::PublicAggreBookTicker(ticker)),
        };

        let events = MarketIter::<TestInstrument, OrderBookL1>::from((
            ExchangeId::Mexc,
            instrument,
            wrapper,
        ))
        .0;

        assert_eq!(events.len(), 1);
        match &events[0] {
            Err(DataError::Socket(s)) => assert!(s.contains("Failed to parse quantity")),
            other => panic!("Unexpected event: {other:?}"),
        }
    }
}
