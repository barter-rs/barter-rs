use barter_integration::error::SocketError;
use serde::Deserialize;

use crate::exchange::ibkr::subscription::IbkrSubResponse;



/// ### System Response
/// ```json
/// {
///   "topic":"system",
///   "success":"some_username",
///   "isFT":false,
///   "isPaper":false
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct IbkrSystemResponse {
    #[serde(rename = "success")]
    pub username: String,
    #[serde(rename = "isFT")]
    pub is_ft: bool,
    pub is_paper: bool,
}

impl IbkrSystemResponse {
    pub fn validate(self) -> Result<IbkrSubResponse, SocketError> {
        // TODO: not sure if a zero-length string is indicator of error
        //       (i.e. not successful, because no username string)
        if self.username.len() > 0 {
            Ok(IbkrSubResponse::System(self))
        } else {
            Err(SocketError::Subscribe(format!(
                "received failed system response success value: {}",
                self.username
            )))
        }
    }
}
