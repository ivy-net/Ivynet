pub struct VaultConfig {
    chain_id: u64,
    address: Address,
    private_key: String,
    /// Watchtower endpoint
    endpoint: String,
    encrypted_key: String,
    key_type: String,
}

pub struct Vault {
    name: String,
}
