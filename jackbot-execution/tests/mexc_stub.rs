use jackbot_execution::client::mexc::{MexcClient, MexcConfig};
use jackbot_execution::client::ExecutionClient;

#[test]
fn can_instantiate_mexc_client() {
    let _client = MexcClient::new(MexcConfig::default());
}

