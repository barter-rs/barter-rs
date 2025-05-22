use jackbot_execution::client::gateio::{GateIoClient, GateIoConfig};
use jackbot_execution::client::ExecutionClient;

#[test]
fn can_instantiate_gateio_client() {
    let _client = GateIoClient::new(GateIoConfig::default());
}

