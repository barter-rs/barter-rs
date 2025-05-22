use jackbot_execution::client::okx::{OkxClient, OkxConfig};
use jackbot_execution::client::ExecutionClient;

#[test]
fn can_instantiate_okx_client() {
    let _client = OkxClient::new(OkxConfig::default());
}


