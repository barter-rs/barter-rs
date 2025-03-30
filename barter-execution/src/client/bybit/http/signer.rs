use barter_integration::{
    error::SocketError,
    protocol::http::{
        private::{RequestSigner, Signer, encoder::HexEncoder},
        rest::RestRequest,
    },
};
use chrono::Utc;
use derive_more::derive::Constructor;

const RECV_WINDOW: &str = "5000";

pub type BybitRequestSigner = RequestSigner<BybitSigner, hmac::Hmac<sha2::Sha256>, HexEncoder>;

#[derive(Debug, Clone, Constructor)]
pub struct BybitSigner {
    pub api_key: String,
}

pub struct BybitSignConfig<'a> {
    api_key: &'a str,
    timestamp: i64,
    params_to_sign: String,
    body_to_sign: Option<String>,
}

impl Signer for BybitSigner {
    type Config<'a>
        = BybitSignConfig<'a>
    where
        Self: 'a;

    fn config<'a, Request>(
        &'a self,
        request: Request,
        _builder: &reqwest::RequestBuilder,
    ) -> Result<Self::Config<'a>, SocketError>
    where
        Request: RestRequest,
    {
        let params_to_sign = match request.query_params() {
            Some(params) => serde_urlencoded::to_string(&params)?,
            None => String::default(),
        };

        let body_to_sign = request
            .body()
            .map(|body| serde_json::to_string(body).expect("serialization should not fail"));

        Ok(Self::Config {
            api_key: self.api_key.as_str(),
            timestamp: Utc::now().timestamp_millis(),
            params_to_sign,
            body_to_sign,
        })
    }

    fn add_bytes_to_sign<M>(mac: &mut M, config: &Self::Config<'_>)
    where
        M: hmac::Mac,
    {
        // The message being signed is "{timestamp}{api_key}{rec_window}{query}{body}"
        mac.update(config.timestamp.to_string().as_bytes());
        mac.update(config.api_key.as_bytes());
        mac.update(RECV_WINDOW.as_bytes());
        mac.update(config.params_to_sign.as_bytes());
        if let Some(body) = &config.body_to_sign {
            mac.update(body.as_bytes());
        }
    }

    fn build_signed_request<'a>(
        config: Self::Config<'a>,
        builder: reqwest::RequestBuilder,
        signature: String,
    ) -> Result<reqwest::Request, SocketError> {
        const KEY_HEADER: &str = "X-BAPI-API-KEY";
        const TIMESTAMP_HEADER: &str = "X-BAPI-TIMESTAMP";
        const SIGNATURE_HEADER: &str = "X-BAPI-SIGN";
        const RECEIVE_HEADER: &str = "x-bapi-recv-window";

        builder
            .header(KEY_HEADER, config.api_key)
            .header(TIMESTAMP_HEADER, config.timestamp)
            .header(SIGNATURE_HEADER, signature)
            .header(RECEIVE_HEADER, RECV_WINDOW)
            .build()
            .map_err(SocketError::from)
    }
}
