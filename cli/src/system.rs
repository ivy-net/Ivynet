use sysinfo::{Disks, System};

pub fn get_system_information() -> (u64, u64, u64) {
    let mut sys = System::new();
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list();

    let cpu_cores = sys.cpus().len() as u64;
    let total_memory = sys.total_memory();
    let free_disk = disks[0].available_space();
    (cpu_cores, total_memory, free_disk)
}

#[allow(clippy::type_complexity)]
pub fn get_detailed_system_information() -> (u64, f64, u64, u64, u64, u64, u64) {
    let mut sys = System::new();
    sys.refresh_all();

    let memory_usage = sys.used_memory();
    let memory_free = sys.free_memory();

    let cores = sys.cpus().len() as u64;
    let mut cpu_usage = 0.0;
    for cpu in sys.cpus() {
        cpu_usage += cpu.cpu_usage() as f64;
    }
    let mut disk_usage = 0;
    let mut free_disk = 0;
    for disk in &Disks::new_with_refreshed_list() {
        disk_usage += disk.total_space() - disk.available_space();
        free_disk += disk.available_space();
    }
    let uptime = System::uptime();
    (cores, cpu_usage, memory_usage, memory_free, disk_usage, free_disk, uptime)
}
