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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_status() {
        let status = collect_status();
        assert!(status.online);
        assert!(status.memory_total_mb > 0);
    }

    #[test]
    fn test_collect_status_cpu() {
        let status = collect_status();
        assert!(status.cpu_percent >= 0.0);
    }

    #[test]
    fn test_collect_status_memory() {
        let status = collect_status();
        assert!(status.memory_used_mb <= status.memory_total_mb);
    }
}
