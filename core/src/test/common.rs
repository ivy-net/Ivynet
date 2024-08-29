use ethers::types::Address;
use once_cell::sync::Lazy;

pub static DEFAULT_OPERATOR_ADDRESS: Lazy<Address> =
    Lazy::new(|| "0xABcdeabCDeABCDEaBCdeAbCDeABcdEAbCDEaBcde".parse::<Address>().unwrap());
