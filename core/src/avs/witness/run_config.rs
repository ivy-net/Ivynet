/// Management utility for the config file to be passed to the docker instance. This is a partial
/// reconstruction, as the original contains duplicate fields as comments, as well as a trailing
/// comma, and is made to be maintianed by hand.
///
/// The source can be found here: https://raw.githubusercontent.com/witnesschain-com/diligencewatchtower-client/testnet/config.json.example
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RunConfig {
    pub private_key: String,
    pub watchtower_address: String,
    pub host_name: String,
    pub watchtower_failure_alert_url: String,
    pub eth_testnet_websocket_url: String,
    currently_watching_l1: String,
    eth_testnet_chain_id: u64,
    proof_submission_chain_url: String,
    proof_submission_chain_id: u64,
    currently_watching_l2: String,
    witnesschain_coordinator_url: String,
    l2: L2Config,
    alert_manager_address: String,
    diligence_proof_manager_address: String,
    operator_registry: String,
    receipt_timeout: u64,
    watchtower_retries: u32,
    external_signer_endpoint: String,
    gocryptfs_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct L2Config {
    op_super_chains: Vec<OpSuperChain>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpSuperChain {
    chain_name: String,
    chain_id: u64,
    op_geth_websocket_url: String,
    op_geth_rpc_url: String,
    op_node_rpc_url: String,
    l2oo_address: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_witness_runconfig() {
        let config_str = r#"{
    "// ---- This is a template config file for watchtower            " : "-----",
    "// ---- Please fill the mandatory entries below marked with TODO " : "-----",
    "// ---- DO NOT Delete any entries in this file                   " : "-----",

     "private_key"                           : "",    "// TODO" : "set the private key for the watchtower",

             "// EXAMPLE"                    : "abcdef0123456789abcdef01234567890abcdef0123456789abcdef012345678",
             "// How to Generate one?"       : "You may use MetaMask or any other tool to generate one",

             "// WARNING 1"                  : "This PRIVATE key is for the identity of watchtower, and NOT operator's private key",
             "// WARNING 2"                  : "NEVER expose your operator's private key anywhere",
     "watchtower_address"                    : "",

     "host_name"                             : "",

             "// EXAMPLE"                    : "my-watchtower.my-domain.com",
             "// RECOMMENDED"                : "to set host name to view your rewards via WitnessChain's watchtower dashboard",

      "watchtower_failure_alert_url"         : "",

             "// EXAMPLE"                    : "https://my-failure-alert/url",
             "// RECOMMENDED"                : "to set it to a url which will be called (POST) when the watchtower fails",

      "eth_testnet_websocket_url"            : "",    "// TODO" : "set the L1 websocket URL",

             "// EXAMPLE"                    : "wss://my-L1-node -OR- ws://my-L1-node",
      
      "currently_watching_l1"                : "eth_testnet",
      "eth_testnet_chain_id"                 : 17000,
      "proof_submission_chain_url"           : "https://blue-orangutan-rpc.eu-north-2.gateway.fm/",
      "proof_submission_chain_id"            : 1237146866,
      "currently_watching_l2"                : "base", "// TODO" : "set the chain to watch, choose between 'optimsm', 'base'",
      "witnesschain_coordinator_url"         : "https://api.witnesschain.com/",

      "l2"                                   : {
           "op_super_chains"                 : 
           [

                  {
                         "chain_name"           : "base",

                         "chain_id"             : 84532,

                         "op_geth_websocket_url": "ws://my-base-node:8546/",         "// TODO" : "set the op-geth's websocket URL",

                         "op_geth_rpc_url"      : "http://my-base-node:8545",        "// TODO" : "set the op-geth's RPC URL",

                         "op_node_rpc_url"      : "http://my-base-node:9545",        "// TODO" : "set the op-node's RPC URL",

                         "l2oo_address"         : "0x84457ca9D0163FbC4bbfe4Dfbb20ba46e48DF254"
                  },

                  {
                         "chain_name"           : "optimism",

                         "chain_id"             : 11155420,

                         "op_geth_websocket_url": "ws://my-optimism-node:8546/",     "// TODO" : "set the op-geth's websocket URL",

                         "op_geth_rpc_url"      : "http://my-optimism-node:8545",    "// TODO" : "set the op-geth's RPC URL",

                         "op_node_rpc_url"      : "http://my-optimism-node:9545",    "// TODO" : "set the op-node's RPC URL",

                         "l2oo_address"         : "0x90E9c4f8a994a250F6aEfd61CAFb4F2e895D458F"
                  }
           ]
      },

      "alert_manager_address"                : "0xF9696529cB591E0EA9f08BBB5908Ae4a342a1F14",
      "diligence_proof_manager_address"      : "0x7AB3b14F3177935d4539d80289906633615393F2",
      "operator_registry"                    : "0x26710e60A36Ace8A44e1C3D7B33dc8B80eAb6cb7",
      "receipt_timeout"                      : 300,
      "watchtower_retries"                   : 5,
      "external_signer_endpoint"             : "http://localhost:9000",
      "gocryptfs_key"                        : "",    "// TODO" : "set the private key path+name used for the watchtower key in gocryptfs (this is optional)",
                                                             "" : "if only file name is given, the key will be taken from the default path ~/witnesschain/.encrypted_directory)",
}"#;
        let run_config = serde_jsonrc::from_str::<RunConfig>(config_str).unwrap();
        assert_eq!(run_config.private_key, "");
        assert_eq!(run_config.watchtower_address, "");
    }
}
