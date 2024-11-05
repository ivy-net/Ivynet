use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::avs::AvsProvider;

pub struct NodeManager(HashMap<String, AvsProvider>);

impl Deref for NodeManager {
    type Target = HashMap<String, AvsProvider>;

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
    fn new() -> Self {
        Self(HashMap::new())
    }
    fn list_nodes(&self) -> Vec<String> {
        self.keys().cloned().collect()
    }
}
