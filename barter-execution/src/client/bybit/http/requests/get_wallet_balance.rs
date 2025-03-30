use std::borrow::Cow;

use barter_instrument::asset::name::AssetNameExchange;
use barter_integration::protocol::http::rest::RestRequest;
use derive_more::derive::Constructor;
use reqwest::Method;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::formats::CommaSeparator;
use serde_with::{DisplayFromStr, StringWithSeparator, serde_as, skip_serializing_none};

use crate::client::bybit::http::{BybitHttpResponse, ResultList};
use crate::client::bybit::types::AccountType;

/// https://bybit-exchange.github.io/docs/v5/account/wallet-balance
#[derive(Debug, Clone, Constructor)]
pub struct GetWalletBalanceRequest(GetWalletBalanceParams);

impl RestRequest for GetWalletBalanceRequest {
    type Response = GetWalletBalanceResponse;
    type QueryParams = GetWalletBalanceParams;
    type Body = ();

    fn path(&self) -> Cow<'static, str> {
        "/v5/account/wallet-balance".into()
    }

    fn method() -> Method {
        Method::GET
    }

    fn query_params(&self) -> Option<&Self::QueryParams> {
        Some(&self.0)
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct GetWalletBalanceParams {
    #[serde(rename = "accountType")]
    pub account_type: AccountType,

    #[serde(rename = "coin")]
    #[serde_as(as = "Option<StringWithSeparator::<CommaSeparator, AssetNameExchange>>")]
    pub coin: Option<Vec<AssetNameExchange>>,
}

pub type GetWalletBalanceResponse = BybitHttpResponse<ResultList<GetWalletBalanceResponseInner>>;

#[derive(Debug, Clone, Deserialize)]
pub struct GetWalletBalanceResponseInner {
    #[serde(rename = "accountType")]
    pub account_type: AccountType,

    #[serde(rename = "coin")]
    pub coin: Vec<GetWalletBalanceCoin>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct GetWalletBalanceCoin {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "walletBalance")]
    pub total_balance: Decimal,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "locked")]
    pub locked_balance: Decimal,

    #[serde(rename = "coin")]
    pub asset: AssetNameExchange,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod de {
        use super::*;

        #[test]
        fn test_get_wallet_balance() {
            let raw_response = r#"{
    "retCode": 0,
    "retMsg": "OK",
    "result": {
        "list": [
            {
                "totalEquity": "1067.12846971",
                "accountIMRate": "0.0356",
                "totalMarginBalance": "1067.12711688",
                "totalInitialMargin": "38.00025129",
                "accountType": "UNIFIED",
                "totalAvailableBalance": "1029.12686558",
                "accountMMRate": "0.0056",
                "totalPerpUPL": "-1.46933442",
                "totalWalletBalance": "1068.5964513",
                "accountLTV": "0",
                "totalMaintenanceMargin": "6.02648343",
                "coin": [
                    {
                        "availableToBorrow": "",
                        "bonus": "0",
                        "accruedInterest": "0",
                        "availableToWithdraw": "1029.0085296",
                        "totalOrderIM": "0",
                        "equity": "1067.00376569",
                        "totalPositionMM": "6.02579047",
                        "usdValue": "1067.12647112",
                        "unrealisedPnl": "-1.46916547",
                        "collateralSwitch": true,
                        "spotHedgingQty": "0",
                        "borrowAmount": "0.000000000000000000",
                        "totalPositionIM": "37.99588177",
                        "walletBalance": "1068.47293116",
                        "cumRealisedPnl": "-170.59218862",
                        "locked": "0",
                        "marginCollateral": true,
                        "coin": "USDT"
                    },
                    {
                        "availableToBorrow": "",
                        "bonus": "0",
                        "accruedInterest": "",
                        "availableToWithdraw": "0.000653",
                        "totalOrderIM": "0",
                        "equity": "0.000653",
                        "totalPositionMM": "0",
                        "usdValue": "0.00070706",
                        "unrealisedPnl": "0",
                        "collateralSwitch": false,
                        "spotHedgingQty": "0",
                        "borrowAmount": "0.000000000000000000",
                        "totalPositionIM": "0",
                        "walletBalance": "0.000653",
                        "cumRealisedPnl": "0",
                        "locked": "0",
                        "marginCollateral": false,
                        "coin": "EUR"
                    },
                    {
                        "availableToBorrow": "",
                        "bonus": "0",
                        "accruedInterest": "0",
                        "availableToWithdraw": "0.0077",
                        "totalOrderIM": "0",
                        "equity": "0.0077",
                        "totalPositionMM": "0",
                        "usdValue": "0.00129152",
                        "unrealisedPnl": "0",
                        "collateralSwitch": true,
                        "spotHedgingQty": "0",
                        "borrowAmount": "0.000000000000000000",
                        "totalPositionIM": "0",
                        "walletBalance": "0.0077",
                        "cumRealisedPnl": "0",
                        "locked": "0",
                        "marginCollateral": true,
                        "coin": "BLUR"
                    }
                ]
            }
        ]
    },
    "retExtInfo": {},
    "time": 1720521356353
}"#;

            serde_json::from_str::<GetWalletBalanceResponse>(raw_response).unwrap();
        }
    }
}
