use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use sysinfo::{Pid, ProcessesToUpdate, System};
use tui_input::Input;

use crate::metrics::MetricsCollector;

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: tracing::Level,
    pub message: String,
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum InputMode {
    #[default]
    Normal,
    Search,
}

pub struct DashboardApp {
    pub metrics: Arc<MetricsCollector>,
    pub log_buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    pub system: System,
    pub pid: Pid,
    pub should_quit: bool,
    pub dashboard_visible: bool,
    pub log_scroll: usize,
    pub input_mode: InputMode,
    pub search_input: Input,
    pub search_query: String,
    pub show_session_view: bool,
    pub middle_panel_height: u16,
    pub log_panel_height: u16,
}

impl DashboardApp {
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
            input_mode: InputMode::Normal,
            search_input: Input::default(),
            search_query: String::new(),
            show_session_view: false,
            middle_panel_height: 10,
            log_panel_height: 15,
        }
    }

    pub fn refresh_system_info(&mut self) {
        self.system
            .refresh_processes(ProcessesToUpdate::Some(&[self.pid]), false);
    }

    pub fn get_cpu_usage(&self) -> f32 {
        self.system
            .process(self.pid)
            .map(|p| p.cpu_usage())
            .unwrap_or(0.0)
    }

    pub fn get_memory_usage(&self) -> u64 {
        self.system
            .process(self.pid)
            .map(|p| p.memory())
            .unwrap_or(0)
    }

    pub fn apply_search(&mut self) {
        self.search_query = self.search_input.value().to_string();
        self.log_scroll = 0;
    }

    pub fn clear_search(&mut self) {
        self.search_input.reset();
        self.search_query.clear();
        self.log_scroll = 0;
    }

    pub fn increase_log_height(&mut self) {
        if self.log_panel_height < 30 {
            self.log_panel_height += 1;
        }
    }

    pub fn decrease_log_height(&mut self) {
        if self.log_panel_height > 8 {
            self.log_panel_height -= 1;
        }
    }

    pub fn increase_middle_height(&mut self) {
        if self.middle_panel_height < 20 {
            self.middle_panel_height += 1;
        }
    }

    pub fn decrease_middle_height(&mut self) {
        if self.middle_panel_height > 5 {
            self.middle_panel_height -= 1;
        }
    }
}
