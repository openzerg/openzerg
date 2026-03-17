use crate::protocol::AgentStatus;
use sysinfo::System;

pub fn collect_status() -> AgentStatus {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_percent = sys.global_cpu_usage();

    let memory_used_mb = sys.used_memory() / 1024 / 1024;
    let memory_total_mb = sys.total_memory() / 1024 / 1024;

    let disk_used_gb = 0.0;
    let disk_total_gb = 0.0;

    AgentStatus {
        online: true,
        cpu_percent,
        memory_used_mb,
        memory_total_mb,
        disk_used_gb,
        disk_total_gb,
    }
}
