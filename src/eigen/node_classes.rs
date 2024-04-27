use crate::config;

pub enum NodeClass {
    USELESS,
    LRG,    //  cpus: 2, mem: 8gb, bandwidth: 5mbps,
    XL,     //  cpus: 4, mem: 16gb, bandwidth: 25mbps,
    FOURXL, //  cpus: 16, mem: 64gb, bandwidth: 5000mbps,
}

pub fn get_node_class() -> Result<NodeClass, Box<dyn std::error::Error>> {
    let (cpus, mem_info, disk_info) = config::get_system_information()?;
    if cpus >= 16 && mem_info.total >= 64000000 {
        return Ok(NodeClass::FOURXL);
    } else if cpus >= 4 && mem_info.total >= 16000000 {
        return Ok(NodeClass::XL);
    } else if cpus >= 2 && mem_info.total >= 8000000 {
        return Ok(NodeClass::LRG);
    }
    Ok(NodeClass::USELESS)
}
