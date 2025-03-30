use barter_instrument::asset::name::AssetNameExchange;
use barter_instrument::asset::QuoteAsset;
use barter_instrument::exchange::ExchangeId;
use barter_instrument::instrument::name::InstrumentNameExchange;
use barter_instrument::Side;
use barter_integration::de::de_u64_epoch_ms_as_datetime_utc;
use barter_integration::error::SocketError;
use barter_integration::protocol::http::private::encoder::{Encoder, HexEncoder};
use barter_integration::protocol::websocket::{WebSocket, WsMessage};
use chrono::{DateTime, Duration, Utc};
use futures::SinkExt;
use hmac::{Hmac, Mac};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_with::{serde_as, DefaultOnError, DisplayFromStr};
use sha2::Sha256;
use tracing::error;

use crate::order::id::{OrderId, StrategyId};
use crate::order::Order;
use crate::trade::{AssetFees, Trade, TradeId};
use crate::{AccountEvent, AccountEventKind, ApiCredentials, UnindexedAccountEvent};

use super::types::{BybitOrderStatus, BybitPositionSide, InstrumentCategory};

#[derive(Debug, Deserialize)]
pub struct BybitPayload {
    #[serde(alias = "topic")]
    pub topic: String,

    #[serde(
        alias = "creationTime",
        deserialize_with = "de_u64_epoch_ms_as_datetime_utc"
    )]
    pub timestamp: DateTime<Utc>,

    #[serde(rename = "data")]
    pub data: Box<RawValue>,
}

#[serde_as]
#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct OrderUpdateData {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,

    #[serde(rename = "symbol")]
    pub symbol: InstrumentNameExchange,

    #[serde(rename = "orderId")]
    pub exchange_order_id: OrderId,

    #[serde_as(as = "DefaultOnError<Option<DisplayFromStr>>")]
    #[serde(rename = "orderLinkId")]
    pub client_order_id: Option<String>,

    #[serde(rename = "side")]
    pub side: Side,

    #[serde(rename = "positionIdx")]
    pub position_side: Option<BybitPositionSide>,

    #[serde(rename = "orderStatus")]
    pub status: BybitOrderStatus,

    #[serde(rename = "rejectReason")]
    pub rejection_reason: String,

    #[serde(rename = "cancelType")]
    pub cancel_type: String,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "qty")]
    pub original_quantity: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "price")]
    pub original_price: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "cumExecQty")]
    pub cumulative_executed_quantity: f64,

    #[serde_as(as = "DefaultOnError<Option<DisplayFromStr>>")]
    #[serde(rename = "avgPrice")]
    pub average_price: Option<f64>,
}

#[serde_as]
#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct OrderExecutionData {
    #[serde(rename = "category")]
    pub category: InstrumentCategory,

    #[serde(rename = "symbol")]
    pub symbol: InstrumentNameExchange,

    #[serde(rename = "orderId")]
    pub exchange_order_id: OrderId,

    #[serde_as(as = "DefaultOnError<Option<DisplayFromStr>>")]
    #[serde(rename = "orderLinkId")]
    pub client_order_id: Option<String>,

    #[serde(rename = "execId")]
    pub trade_id: TradeId,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "execPrice")]
    pub exec_price: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "execQty")]
    pub exec_qty: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "orderQty")]
    pub order_qty: f64,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "leavesQty")]
    pub remaining_qty: f64,
}

/// Authenticate the connection and subscribe to required topics.
pub async fn subscribe(
    credentials: &ApiCredentials,
    stream: &mut WebSocket,
) -> Result<(), SocketError> {
    let auth_message = generate_auth_message(&credentials.key, &credentials.secret);
    let sub_message = WsMessage::text(
        serde_json::json!({
                "op": "subscribe",
                "args": ["order", "execution"] // TODO: Add account balance changes
        })
        .to_string(),
    );

    stream.send(auth_message).await?;
    stream.send(sub_message).await?;

    // TODO: Validate the response

    Ok(())
}

fn generate_auth_message(api_key: &str, api_secret: &str) -> WsMessage {
    fn sign_message(secret: &str, message: &str) -> String {
        let mut signed_key = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("secret should have a valid length");
        signed_key.update(message.as_bytes());
        HexEncoder.encode(signed_key.finalize().into_bytes())
    }

    let expires_at = (Utc::now() + Duration::seconds(5)).timestamp_millis();
    let message = format!("GET/realtime{}", expires_at);
    let signature = sign_message(&api_secret, &message);

    WsMessage::text(
        serde_json::json!({
            "op": "auth",
            "args": [
                api_key,
                expires_at,
                signature,
            ]
        })
        .to_string(),
    )
}

pub async fn extract_event(
    payload: BybitPayload,
    _assets: &[AssetNameExchange],
    instruments: &[InstrumentNameExchange],
) -> Result<Option<UnindexedAccountEvent>, ()> {
    let event = match payload.topic.as_str() {
        "order" => {
            let order = serde_json::from_str::<OrderUpdateData>(payload.data.get()).unwrap();

            instruments
                .contains(&order.symbol)
                .then(|| to_unified_order(order, payload.timestamp))
        }
        "execution" => {
            let execution = serde_json::from_str::<OrderExecutionData>(payload.data.get()).unwrap();

            instruments
                .contains(&execution.symbol)
                .then(|| to_unified_execution(execution, payload.timestamp))
                .flatten()
        }
        // TODO: Add balance and position updates
        _ => {
            error!(?payload, "message from unknown topic received");
            None
        }
    };

    Ok(event)
}

fn to_unified_order(order: OrderUpdateData, time_exchange: DateTime<Utc>) -> UnindexedAccountEvent {
    let kind = match order.status {
        BybitOrderStatus::New
        | BybitOrderStatus::PartiallyFilled
        | BybitOrderStatus::Untriggered => {
            // AccountEventKind::OrderOpened::<ExchangeId, AssetNameExchange, InstrumentNameExchange>(
            //     Order {
            //         exchange: ExchangeId::BybitSpot,
            //         instrument: order.symbol,
            //         strategy: StrategyId::unknown(),
            //         cid: ClientOrderId::new(order.client_order_id.unwrap()),
            //         side: order.side,
            //         state: Ok(Open {
            //             id: order.exchange_order_id,
            //             time_exchange,
            //             price: Decimal::from_f64(order.original_price).unwrap(),
            //             // TODO: We should probably also add an average price
            //             quantity: Decimal::from_f64(order.original_quantity).unwrap(),
            //             filled_quantity: Decimal::from_f64(order.cumulative_executed_quantity)
            //                 .unwrap(),
            //         }),
            //     },
            // )
            todo!()
        }
        BybitOrderStatus::Rejected
        | BybitOrderStatus::PartiallyFilledCanceled
        | BybitOrderStatus::Filled
        | BybitOrderStatus::Cancelled
        | BybitOrderStatus::Triggered
        | BybitOrderStatus::Deactivated => {
            // AccountEventKind::OrderCancelled::<ExchangeId, AssetNameExchange, InstrumentNameExchange>(
            //     Order {
            //         exchange: ExchangeId::BybitSpot,
            //         instrument: order.symbol,
            //         strategy: StrategyId::unknown(),
            //         cid: ClientOrderId::new(order.client_order_id.unwrap()),
            //         side: order.side,
            //         state: Ok(Cancelled {
            //             id: order.exchange_order_id,
            //             time_exchange,
            //         }),
            //     },
            // )
            todo!()
        }
    };

    AccountEvent {
        exchange: ExchangeId::BybitSpot,
        kind,
    }
}

pub fn to_unified_execution(
    execution: OrderExecutionData,
    time_exchange: DateTime<Utc>,
) -> Option<UnindexedAccountEvent> {
    Some(AccountEvent {
        exchange: ExchangeId::BybitSpot,
        kind: AccountEventKind::Trade(Trade {
            id: execution.trade_id,
            order_id: execution.exchange_order_id,
            instrument: execution.symbol,
            strategy: StrategyId::unknown(),
            time_exchange,
            // TODO: Handle side
            side: todo!(),
            price: Decimal::from_f64(execution.exec_price)?,
            quantity: Decimal::from_f64(execution.exec_qty)?,
            // TODO: Handle fees
            fees: AssetFees {
                asset: QuoteAsset,
                fees: todo!(),
            },
        }),
    })
}
