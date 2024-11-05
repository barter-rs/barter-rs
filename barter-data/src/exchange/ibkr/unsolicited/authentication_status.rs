use serde::{Deserialize, Serialize};

/// ### Status Response
/// ```json
/// {
///   "topic": "sts",
///   "args": {
///     "authenticated": true,
///     "competing": false,
///     "message": "",
///     "fail": "",
///     "serverName": "some_servername",
///     "serverVersion": "Build 10.28.0c, Apr 1, 2024 6:35:40 PM",
///     "username": "some_username"
///   }
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct IbkrAuthnStatusResponse {
    #[serde(rename = "args")]
    pub args: AuthnStatusArgs,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthnStatusArgs {
    pub authenticated: bool,
    pub competing: bool,
    pub message: String,
    pub fail: String,
    pub server_name: String,
    pub server_version: String,
    pub username: String,
}
