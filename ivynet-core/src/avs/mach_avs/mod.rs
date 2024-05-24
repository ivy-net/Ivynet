pub mod altlayer;

pub use altlayer::AltLayer;
use ethers::types::U256;
use std::{collections::HashMap, error::Error};

use crate::{
    config,
    eigen::node_classes::{self, NodeClass},
};
