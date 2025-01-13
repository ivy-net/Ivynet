use crate::{error::IvyError, system::get_system_information};

#[derive(PartialEq, PartialOrd, Ord)]
pub enum NodeClass {
    // Note: Eigen's specified node classes do not contain disk requirements
    USELESS,
    LRG,    //  cpus: 2, mem: 8gb, bandwidth: 5mbps,
    XL,     //  cpus: 4, mem: 16gb, bandwidth: 25mbps,
    FOURXL, //  cpus: 16, mem: 64gb, bandwidth: 5000mbps,
}

pub fn get_node_class() -> Result<NodeClass, IvyError> {
    let (cpus, mem_info, _) = get_system_information();
    if cpus >= 16 && mem_info >= 64000000 {
        return Ok(NodeClass::FOURXL);
    } else if cpus >= 4 && mem_info >= 16000000 {
        return Ok(NodeClass::XL);
    } else if cpus >= 2 && mem_info >= 8000000 {
        return Ok(NodeClass::LRG);
    }
    Ok(NodeClass::USELESS)
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
