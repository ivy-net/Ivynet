use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::avs::IvyNode;

#[derive(Debug)]
pub struct NodeManager(HashMap<String, IvyNode>);

impl Deref for NodeManager {
    type Target = HashMap<String, IvyNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NodeManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl NodeManager {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn list_nodes(&self) -> Vec<String> {
        self.keys().cloned().collect()
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}
