use jackbot_execution::client::cryptocom::{CryptocomClient, CryptocomConfig};
use jackbot_execution::client::ExecutionClient;

#[test]
fn can_instantiate_cryptocom_client() {
    let _client = CryptocomClient::new(CryptocomConfig::default());
}

