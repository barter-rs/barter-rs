use barter_integration::error::SocketError;
use serde::{Deserialize, Serialize};

use crate::exchange::ibkr::subscription::IbkrPlatformEvent;



/// ### System Response
/// ```json
/// {
///   "topic":"system",
///   "success":"some_username",
///   "isFT":false,
///   "isPaper":false
/// }
/// ```
///
/// ### System Response (after initial connection)
/// ```json
/// {
///   "topic":"system",
///   "hb":1729601500848
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IbkrSystemResponse {
    #[serde(rename = "success")]
    pub username: Option<String>,
    #[serde(rename = "isFT")]
    pub is_ft: Option<bool>,
    #[serde(rename = "isPaper")]
    pub is_paper: Option<bool>,
    #[serde(rename = "hb")]
    pub hb: Option<u64>,
}

impl IbkrSystemResponse {
    pub fn validate(self) -> Result<IbkrPlatformEvent, SocketError> {
        // TODO: not sure if a zero-length string is indicator of error
        //       (i.e. not successful, because no username string)
        if self.username.is_some() && self.username.clone().unwrap().len() > 0 {
            Ok(IbkrPlatformEvent::System(self))
        } else
        if self.hb.is_some() {
            Ok(IbkrPlatformEvent::System(self))
        } else {
            Err(SocketError::Subscribe(format!(
                "received failed system response success value"
            )))
        }
    }
}
