use sysinfo::{Disks, System};

use crate::error::IvyError;

pub fn get_system_information() -> Result<(u64, u64, u64), IvyError> {
    let mut sys = System::new();
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list();

    let cpu_cores = sys.cpus().len() as u64;
    let total_memory = sys.total_memory();
    let free_disk = disks[0].available_space();
    Ok((cpu_cores, total_memory, free_disk))
}
