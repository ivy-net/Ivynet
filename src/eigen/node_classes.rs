use std::cmp::Ordering;

use crate::config;

#[derive(PartialEq, PartialOrd)]
pub enum NodeClass {
    USELESS,
    LRG,    //  cpus: 2, mem: 8gb, bandwidth: 5mbps,
    XL,     //  cpus: 4, mem: 16gb, bandwidth: 25mbps,
    FOURXL, //  cpus: 16, mem: 64gb, bandwidth: 5000mbps,
}

pub fn get_node_class() -> Result<NodeClass, Box<dyn std::error::Error>> {
    let (cpus, mem_info, disk_info) = config::get_system_information()?;
    if cpus >= 16 && mem_info >= 64000000 {
        return Ok(NodeClass::FOURXL);
    } else if cpus >= 4 && mem_info >= 16000000 {
        return Ok(NodeClass::XL);
    } else if cpus >= 2 && mem_info >= 8000000 {
        return Ok(NodeClass::LRG);
    }
    Ok(NodeClass::USELESS)
}

impl Ord for NodeClass {
    fn cmp(&self, other: &Self) -> Ordering {
        fn rank(nc: &NodeClass) -> u8 {
            match nc {
                NodeClass::USELESS => 0,
                NodeClass::LRG => 1,
                NodeClass::XL => 2,
                NodeClass::FOURXL => 3,
            }
        }

        rank(self).cmp(&rank(other))
    }
}

impl Eq for NodeClass {}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_class_ordering() {
        assert!(NodeClass::USELESS < NodeClass::LRG);
        assert!(NodeClass::LRG < NodeClass::XL);
        assert!(NodeClass::XL < NodeClass::FOURXL);
    }
}
