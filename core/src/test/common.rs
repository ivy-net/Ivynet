use ethers::types::Address;
use once_cell::sync::Lazy;

pub static DEFAULT_OPERATOR_ADDRESS: Lazy<Address> =
    Lazy::new(|| "0xABcdeabCDeABCDEaBCdeAbCDeABcdEAbCDEaBcde".parse::<Address>().unwrap());

pub static DEFAULT_EIGENLAYER_MULTISIG_ADDRESS: Lazy<Address> =
    Lazy::new(|| "0x123463a4b065722e99115d6c222f267d9cabb524".parse::<Address>().unwrap());
