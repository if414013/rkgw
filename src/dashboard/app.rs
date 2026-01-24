use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::metrics::MetricsCollector;

/// Log entry for dashboard display
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: tracing::Level,
    pub message: String,
}

/// Dashboard application state
pub struct DashboardApp {
    /// Shared metrics collector
    pub metrics: Arc<MetricsCollector>,
    /// Log buffer for display
    pub log_buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    /// System info for CPU/memory
    pub system: System,
    /// Current process ID
    pub pid: Pid,
    /// Should quit flag
    pub should_quit: bool,
    /// Dashboard visible flag
    pub dashboard_visible: bool,
    /// Log scroll position
    pub log_scroll: usize,
}

impl DashboardApp {
    /// Create a new dashboard application
    pub fn new(metrics: Arc<MetricsCollector>, log_buffer: Arc<Mutex<VecDeque<LogEntry>>>) -> Self {
        let pid = Pid::from_u32(std::process::id());
        Self {
            metrics,
            log_buffer,
            system: System::new_all(),
            pid,
            should_quit: false,
            dashboard_visible: true,
            log_scroll: 0,
        }
    }

    /// Refresh system information (CPU and memory for this process)
    pub fn refresh_system_info(&mut self) {
        self.system
            .refresh_processes(ProcessesToUpdate::Some(&[self.pid]), false);
    }

    /// Get current CPU usage percentage for this process
    pub fn get_cpu_usage(&self) -> f32 {
        self.system
            .process(self.pid)
            .map(|p| p.cpu_usage())
            .unwrap_or(0.0)
    }

    /// Get memory usage in bytes for this process
    pub fn get_memory_usage(&self) -> u64 {
        self.system
            .process(self.pid)
            .map(|p| p.memory())
            .unwrap_or(0)
    }
}
