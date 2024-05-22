use sysinfo::{Disks, System};

pub struct SystemInfo {
    pub bandwidth: u64,
    pub cpu_cores: u64,
    pub total_mem: u64,
    pub free_disk: u64,
}

impl SystemInfo {
    fn new_from_bandwidth(bandwidth: u64) -> Self {
        let mut sys = System::new();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let cpu_cores = sys.cpus().len() as u64;
        let total_mem = sys.total_memory();
        let free_disk = disks[0].available_space();
        Self { bandwidth, cpu_cores, total_mem, free_disk }
    }
}
