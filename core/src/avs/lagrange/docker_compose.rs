/// Module for editing the lagrange docker-compose file
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Services {
    services: ServicesDetail,
}

impl Services {
    pub fn set_rpc_url(&mut self, rpc_url: String) {
        self.services.worker.environment[0] = format!("RPC_URL={}", rpc_url);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ServicesDetail {
    worker: WorkerService,
    watchtower: WatchtowerService,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkerService {
    image: String,
    container_name: String,
    environment: Vec<String>,
    ports: Vec<String>,
    volumes: Vec<String>,
    restart: String,
    pull_policy: String,
    command: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WatchtowerService {
    image: String,
    container_name: String,
    volumes: Vec<String>,
    command: String,
}

#[cfg(test)]
mod tests {
    use crate::io::read_yaml;

    use super::*;

    #[test]
    fn test_deserialize_yaml() {
        let yaml_data = r#"
        services:
          worker:
            image: lagrangelabs/worker:${NETWORK}
            container_name: worker
            environment:
            - RPC_URL=https://eth.llamarpc.com
            - AVS__ETH_PWD=${AVS__ETH_PWD}
            - AVS__ETH_KEYSTORE=/config/priv_key.json
            - NETWORK=${NETWORK}
            - AVS__LAGR_KEYSTORE=/config/lagr_keystore.json
            - AVS__LAGR_PWD=${AVS__LAGR_PWD}
            - RUST_LOG=info,worker=debug
            - PUBLIC_PARAMS__SKIP_STORE=false
            ports:
              - "9090:9090"
            volumes:
              - ./config:/config
              - ./zkmr_params:/zkmr_params
            restart: always
            pull_policy: always
            command: ["worker", "--config", "/config/worker-conf.toml"]
          watchtower:
            image: containrrr/watchtower
            container_name: watchtower
            volumes:
              - /var/run/docker.sock:/var/run/docker.sock
            command: --interval 60 lagrangelabs/worker:${NETWORK}
        "#;

        let result = serde_yaml::from_str::<Services>(yaml_data);
        assert!(result.is_ok(), "YAML deserialization failed");
    }

    #[test]
    fn test_deserialize_lagrange_yaml() {
        let home_dir = dirs::home_dir().unwrap();
        let lagrage_worker_dir = home_dir.join(".eigenlayer/lagrange/lagrange-worker");
        let docker_compose_path = lagrage_worker_dir.join("docker-compose.yaml");
        let _yaml: Services = read_yaml(&docker_compose_path).unwrap();
    }
}
