use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use sysinfo::System;

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
        Self {
            metrics,
            log_buffer,
            system: System::new_all(),
            should_quit: false,
            dashboard_visible: true,
            log_scroll: 0,
        }
    }

    /// Refresh system information (CPU and memory)
    pub fn refresh_system_info(&mut self) {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
    }

    /// Get current CPU usage percentage
    pub fn get_cpu_usage(&self) -> f32 {
        self.system.global_cpu_usage()
    }

    /// Get memory usage (used, total) in bytes
    pub fn get_memory_usage(&self) -> (u64, u64) {
        (self.system.used_memory(), self.system.total_memory())
    }
}
