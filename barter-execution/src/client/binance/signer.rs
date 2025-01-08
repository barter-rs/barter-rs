use derive_more::Constructor;
use barter_integration::error::SocketError;
use barter_integration::protocol::http::private::{RequestSigner, Signer};
use barter_integration::protocol::http::private::encoder::HexEncoder;
use barter_integration::protocol::http::rest::RestRequest;


pub type BinanceSpotSigner = RequestSigner<BinanceSigner, hmac::Hmac<sha2::Sha256>, HexEncoder>;


#[derive(Debug, Clone, PartialEq, Constructor)]
pub struct BinanceSigner {
    api_key: String,
}


pub struct BinanceSignConfig<'a> {
    pub api_key: &'a str,
    pub request_params_to_sign: String,
}

impl Signer for BinanceSigner {
    type Config<'a> = BinanceSignConfig<'a>
    where
        Self: 'a;

    fn config<Request>(
        &self,
        request: Request,
        builder: &reqwest::RequestBuilder
    ) -> Result<Self::Config<'_>, SocketError>
    where
        Request: RestRequest
    {
        let request_params_to_sign = request
            .query_params()
            .map(serde_urlencoded::to_string)
            .expect("BinanceSpot private requests should all have QueryParams")?;

        Ok(Self::Config {
            api_key: self.api_key.as_str(),
            request_params_to_sign
        })
    }

    fn add_bytes_to_sign<M>(mac: &mut M, config: &Self::Config<'_>)
    where
        M: hmac::Mac
    {
        mac.update(config.request_params_to_sign.as_bytes());
    }

    fn build_signed_request(
        config: Self::Config<'_>,
        builder: reqwest::RequestBuilder,
        signature: String
    ) -> Result<reqwest::Request, SocketError>
    {
        const HEADER_KEY_API_KEY: &str = "X-MBX-APIKEY";
        const QUERY_KEY_SIGNATURE: &str = "signature";

        builder
            .header(HEADER_KEY_API_KEY, config.api_key)
            .query(&[(QUERY_KEY_SIGNATURE, signature)])
            .build()
            .map_err(SocketError::from)
    }
}

