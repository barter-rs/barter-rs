use barter_instrument::{Side, exchange::ExchangeId};
use chrono::{DateTime, Utc};

use crate::{
    error::DataError,
    event::{MarketEvent, MarketIter},
    subscription::trade::PublicTrade,
};

// Protobuf generated structs module
pub mod proto {
    #![allow(clippy::all)]
    #![allow(warnings)]
    include!("protobuf_gen/_.rs");
}

/// Converts a millisecond Unix epoch timestamp (i64) to `DateTime<Utc>`.
fn ms_epoch_to_datetime_utc(ms: i64) -> Result<DateTime<Utc>, DataError> {
    if ms < 0 {
        return Err(DataError::Socket(format!(
            "Unsupported MexcTrade::Timestamp: invalid unix_epoch_ms (negative): {}",
            ms
        )));
    }
    DateTime::from_timestamp_millis(ms).ok_or_else(|| {
        DataError::Socket(format!(
            "Unsupported MexcTrade::Timestamp: invalid unix_epoch_ms: {}",
            ms
        ))
    })
}

impl<InstrumentKey> From<(ExchangeId, InstrumentKey, proto::PushDataV3ApiWrapper)>
    for MarketIter<InstrumentKey, PublicTrade>
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
        let mut market_events = Vec::new();
        let time_received = Utc::now();

        if let Some(body) = wrapper.body {
            match body {
                proto::push_data_v3_api_wrapper::Body::PublicAggreDeals(deals) => {
                    let events = map_public_aggre_deals_to_market_events(
                        exchange_id,
                        instrument.clone(),
                        &deals,
                        time_received,
                    );
                    market_events.extend(events);
                }
                proto::push_data_v3_api_wrapper::Body::PublicAggreBookTicker(book_ticker) => {
                    let exchange_time = wrapper
                        .send_time
                        .or(wrapper.create_time)
                        .and_then(|ms| ms_epoch_to_datetime_utc(ms).ok())
                        .unwrap_or(time_received);

                    let events = map_public_aggre_book_ticker_to_market_events(
                        exchange_id,
                        instrument.clone(),
                        &book_ticker,
                        exchange_time,
                        time_received,
                    );
                    market_events.extend(events);
                }
                _ => {} // Other message types not handled here
            }
        }
        MarketIter(market_events) // MarketIter expects Vec<Result<_, DataError>>
    }
}

// Helper function to map proto::PublicAggreBookTickerV3Api to two MarketEvent<PublicTrade> instances
fn map_public_aggre_book_ticker_to_market_events<InstrumentKey: Clone>(
    exchange_id: ExchangeId,
    instrument: InstrumentKey,
    ticker: &proto::PublicAggreBookTickerV3Api,
    exchange_time: DateTime<Utc>,
    time_received: DateTime<Utc>,
) -> Vec<Result<MarketEvent<InstrumentKey, PublicTrade>, DataError>> {
    let parse = |price: &str, qty: &str, side: Side| -> Result<_, DataError> {
        let price = price.parse::<f64>().map_err(|e| {
            DataError::Socket(format!(
                "Failed to parse price from MEXC agg book ticker: '{}', error: {}",
                price, e
            ))
        })?;
        let amount = qty.parse::<f64>().map_err(|e| {
            DataError::Socket(format!(
                "Failed to parse quantity from MEXC agg book ticker: '{}', error: {}",
                qty, e
            ))
        })?;
        Ok(MarketEvent {
            time_exchange: exchange_time,
            time_received,
            exchange: exchange_id,
            instrument: instrument.clone(),
            kind: PublicTrade {
                id: exchange_time.timestamp_millis().to_string(),
                price,
                amount,
                side,
            },
        })
    };

    vec![
        parse(&ticker.bid_price, &ticker.bid_quantity, Side::Buy),
        parse(&ticker.ask_price, &ticker.ask_quantity, Side::Sell),
    ]
}

// Helper to map proto::PublicAggreDealsV3Api to MarketEvent<PublicTrade> instances
fn map_public_aggre_deals_to_market_events<InstrumentKey: Clone>(
    exchange_id: ExchangeId,
    instrument: InstrumentKey,
    deals: &proto::PublicAggreDealsV3Api,
    time_received: DateTime<Utc>,
) -> Vec<Result<MarketEvent<InstrumentKey, PublicTrade>, DataError>> {
    deals
        .deals
        .iter()
        .map(|deal| {
            let price = deal.price.parse::<f64>().map_err(|e| {
                DataError::Socket(format!(
                    "Failed to parse price from MEXC agg deal: '{}', error: {}",
                    deal.price, e
                ))
            })?;
            let amount = deal.quantity.parse::<f64>().map_err(|e| {
                DataError::Socket(format!(
                    "Failed to parse quantity from MEXC agg deal: '{}', error: {}",
                    deal.quantity, e
                ))
            })?;
            let side = match deal.trade_type {
                1 => Side::Buy,
                2 => Side::Sell,
                s => {
                    return Err(DataError::Socket(format!(
                        "Unsupported trade_type for MEXC agg deal: {}",
                        s
                    )));
                }
            };
            let exchange_time = ms_epoch_to_datetime_utc(deal.time)?;

            Ok(MarketEvent {
                time_exchange: exchange_time,
                time_received,
                exchange: exchange_id,
                instrument: instrument.clone(),
                kind: PublicTrade {
                    id: exchange_time.timestamp_millis().to_string(),
                    price,
                    amount,
                    side,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Identifier;
    use barter_integration::de::datetime_utc_from_epoch_duration;
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
    fn test_ms_epoch_to_datetime_utc_conversion() {
        let timestamp_ms_valid = 1609459200000i64;
        let expected_datetime =
            datetime_utc_from_epoch_duration(Duration::from_millis(timestamp_ms_valid as u64));
        assert_eq!(
            ms_epoch_to_datetime_utc(timestamp_ms_valid),
            Ok(expected_datetime)
        );

        let timestamp_ms_invalid = -1i64;
        match ms_epoch_to_datetime_utc(timestamp_ms_invalid) {
            Err(DataError::Socket(s)) => {
                assert!(s.contains("Unsupported MexcTrade::Timestamp"));
                assert!(s.contains("invalid unix_epoch_ms (negative): -1"));
            }
            other => panic!(
                "Expected DataError::Socket(String) for negative timestamp, got {:?}",
                other
            ),
        }

        // Test with a large value that might fail parsing if not handled by from_timestamp_millis
        // but should be caught by our negative check if it were negative.
        // This specific test case for from_timestamp_millis failing is harder to hit without
        // knowing its exact internal limits for i64 if they are less than i64::MAX.
        // For now, the negative check is the primary concern from the failed test.
    }

    #[test]
    fn test_public_aggre_book_ticker_to_market_event() {
        let ticker = proto::PublicAggreBookTickerV3Api {
            bid_price: "50000.50".to_string(),
            bid_quantity: "0.001".to_string(),
            ask_price: "50001.0".to_string(),
            ask_quantity: "0.002".to_string(),
        };

        let instrument = TestInstrument {
            base: "BTC".into(),
            quote: "USDT".into(),
        };
        let time_received = Utc::now();

        let events = map_public_aggre_book_ticker_to_market_events(
            ExchangeId::Mexc,
            instrument.clone(),
            &ticker,
            time_received,
            time_received,
        );

        assert_eq!(events.len(), 2);
        let event0 = events[0].as_ref().unwrap();
        assert_eq!(event0.exchange, ExchangeId::Mexc);
        assert_eq!(event0.instrument.id(), "BTCUSDT".to_string());
        assert_eq!(event0.kind.price, 50000.50);

        // Test parsing failure for price
        let ticker_bad_price = proto::PublicAggreBookTickerV3Api {
            bid_price: "not_a_float".to_string(),
            bid_quantity: "0.001".to_string(),
            ask_price: "50001.0".to_string(),
            ask_quantity: "0.002".to_string(),
        };
        let result_bad_price = map_public_aggre_book_ticker_to_market_events(
            ExchangeId::Mexc,
            instrument.clone(),
            &ticker_bad_price,
            time_received,
            time_received,
        );
        assert!(matches!(result_bad_price[0], Err(DataError::Socket(_))));
        if let Err(DataError::Socket(s)) = &result_bad_price[0] {
            assert!(s.contains("Failed to parse price"));
        }

        // Test parsing failure for quantity
        let ticker_bad_quantity = proto::PublicAggreBookTickerV3Api {
            bid_price: "50000.50".to_string(),
            bid_quantity: "not_a_float".to_string(),
            ask_price: "50001.0".to_string(),
            ask_quantity: "0.002".to_string(),
        };
        let result_bad_quantity = map_public_aggre_book_ticker_to_market_events(
            ExchangeId::Mexc,
            instrument.clone(),
            &ticker_bad_quantity,
            time_received,
            time_received,
        );
        assert!(matches!(result_bad_quantity[0], Err(DataError::Socket(_))));
        if let Err(DataError::Socket(s)) = &result_bad_quantity[0] {
            assert!(s.contains("Failed to parse quantity"));
        }
    }

    #[test]
    fn test_transform_push_data_v3_api_wrapper_public_aggre_book_ticker() {
        let instrument = TestInstrument {
            base: "ETH".into(),
            quote: "USDT".into(),
        };
        let ticker = proto::PublicAggreBookTickerV3Api {
            bid_price: "3000.1".to_string(),
            bid_quantity: "0.1".to_string(),
            ask_price: "3000.2".to_string(),
            ask_quantity: "0.05".to_string(),
        };

        let wrapper = proto::PushDataV3ApiWrapper {
            channel: "spot@public.aggre.bookTicker.v3.api.pb@100ms@ETHUSDT".to_string(),
            symbol: Some("ETHUSDT".to_string()),
            symbol_id: Some("ETHUSDT_ID".to_string()),
            create_time: Some(1609459300200),
            send_time: Some(1609459300250),
            body: Some(
                proto::push_data_v3_api_wrapper::Body::PublicAggreBookTicker(ticker.clone()),
            ),
        };

        let market_iter = MarketIter::<TestInstrument, PublicTrade>::from((
            ExchangeId::Mexc,
            instrument.clone(),
            wrapper,
        ));
        let events: Vec<_> = market_iter
            .0
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind.side, Side::Buy);
        assert_eq!(events[1].kind.side, Side::Sell);
    }

    #[test]
    fn test_transform_push_data_v3_api_wrapper_no_body() {
        let instrument = TestInstrument {
            base: "BTC".into(),
            quote: "USDT".into(),
        };
        let wrapper = proto::PushDataV3ApiWrapper {
            channel: "some_channel".to_string(),
            symbol: Some("BTCUSDT".to_string()),
            symbol_id: Some("BTCUSDT_ID".to_string()),
            create_time: Some(1609459200000),
            send_time: Some(1609459200000),
            body: None,
        };
        let market_iter = MarketIter::<TestInstrument, PublicTrade>::from((
            ExchangeId::Mexc,
            instrument,
            wrapper,
        ));
        let events: Vec<_> = market_iter
            .0
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_transform_push_data_v3_api_wrapper_other_body_type() {
        let instrument = TestInstrument {
            base: "BTC".into(),
            quote: "USDT".into(),
        };
        let kline_item = proto::PublicSpotKlineV3Api {
            interval: "Min1".to_string(),
            window_start: 1609459200,
            opening_price: "50000".to_string(),
            closing_price: "50010".to_string(),
            highest_price: "50015".to_string(),
            lowest_price: "49990".to_string(),
            volume: "10".to_string(),
            amount: "500000".to_string(),
            window_end: 1609459260,
        };
        let wrapper = proto::PushDataV3ApiWrapper {
            channel: "spot@public.kline.v3.api@Min1@BTCUSDT".to_string(),
            symbol: Some("BTCUSDT".to_string()),
            symbol_id: Some("BTCUSDT_ID".to_string()),
            create_time: Some(1609459260000),
            send_time: Some(1609459260000),
            body: Some(proto::push_data_v3_api_wrapper::Body::PublicSpotKline(
                kline_item,
            )),
        };

        let market_iter = MarketIter::<TestInstrument, PublicTrade>::from((
            ExchangeId::Mexc,
            instrument,
            wrapper,
        ));
        let events: Vec<_> = market_iter
            .0
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(events.is_empty());
    }
}
