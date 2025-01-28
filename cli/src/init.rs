use dialoguer::{Input, Password};
use ivynet_grpc::{
    backend::backend_client::BackendClient, client::create_channel,
    messages::RegistrationCredentials, tonic::Request,
};
use ivynet_signer::IvyWallet;

use crate::{config::IvyConfig, error::Error};

pub async fn register_node(mut config: IvyConfig) -> Result<(), Error> {
    if config.identity_wallet().is_err() {
        set_backend_connection(&mut config).await?;
    }
    let wallet = config.identity_wallet()?;
    let client_key = wallet.address();

    println!("Node registration for key {:?} successful.", client_key);

    Ok(())
}

pub async fn set_backend_connection(config: &mut IvyConfig) -> Result<(), Error> {
    let (identity_key, client_key) = match config.identity_wallet() {
        Ok(key) => (key.to_private_key(), key.address()),
        _ => {
            let new_key = IvyWallet::new();
            (new_key.to_private_key(), new_key.address())
        }
    };

    loop {
        let email = Input::new()
            .with_prompt("Provide email address to IvyNet system")
            .interact_text()
            .expect("No no email provided");
        let password = Password::new()
            .with_prompt("Enter a password to IvyNet system")
            .interact()
            .expect("No password provided");
        let mut backend = BackendClient::new(
            create_channel(config.get_server_url()?, {
                let ca = config.get_server_ca();
                if ca.is_empty() {
                    None
                } else {
                    Some(ca.clone())
                }
            })
            .await?,
        );
        let hostname =
            { String::from_utf8(rustix::system::uname().nodename().to_bytes().to_vec()) }
                .expect("Cannot fetch hostname from the node");
        if backend
            .register(Request::new(RegistrationCredentials {
                machine_id: config.machine_id.into(),
                email,
                password,
                hostname,
                public_key: client_key.as_bytes().to_vec(),
            }))
            .await
            .is_ok()
        {
            break;
        }
    }
    println!("Node properly registered with key {:?}", client_key);
    config.backend_info.identity_key = identity_key;
    config.store()?;
    Ok(())
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::{future::Future, path::PathBuf};
    use tokio::fs;

    pub async fn build_test_dir<F, Fut, T>(test_dir: &str, test_logic: F) -> T
    where
        F: FnOnce(PathBuf) -> Fut,
        Fut: Future<Output = T>,
    {
        let test_path = std::env::current_dir().unwrap().join(format!("testing{}", test_dir));
        fs::create_dir_all(&test_path).await.expect("Failed to create testing_temp directory");
        let result = test_logic(test_path.clone()).await;
        fs::remove_dir_all(test_path).await.expect("Failed to delete testing_temp directory");

        result
    }

    #[tokio::test]
    async fn test_config_file_builds_init() {
        build_test_dir("test_initialization", |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());
            config.store().expect("Config not working");
            let config_file_path = test_path.join("ivy-config.toml");
            assert!(config_file_path.exists());
        })
        .await;
    }
}
