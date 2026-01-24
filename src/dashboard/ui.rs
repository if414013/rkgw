use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

use super::app::DashboardApp;
use super::widgets;

pub fn render(frame: &mut Frame, app: &DashboardApp) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Top row: gauges
            Constraint::Length(10), // Middle row: charts
            Constraint::Min(15),    // Bottom row: logs (expanded)
        ])
        .split(size);

    render_top_row(frame, app, chunks[0]);
    render_middle_row(frame, app, chunks[1]);
    render_log_panel(frame, app, chunks[2]);
}

fn render_top_row(frame: &mut Frame, app: &DashboardApp, area: Rect) {
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    let active = app.metrics.get_active_connections();
    let connections_gauge = widgets::render_connections_gauge(active);
    frame.render_widget(connections_gauge, top_chunks[0]);

    let cpu = app.get_cpu_usage() as f64;
    let cpu_gauge = widgets::render_cpu_gauge(cpu);
    frame.render_widget(cpu_gauge, top_chunks[1]);

    let (used, total) = app.get_memory_usage();
    let mem_gauge = widgets::render_memory_gauge(used, total);
    frame.render_widget(mem_gauge, top_chunks[2]);
}

fn render_middle_row(frame: &mut Frame, app: &DashboardApp, area: Rect) {
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let rate_history = app.metrics.get_request_rate_history();
    let sparkline = widgets::render_sparkline("Requests/sec", &rate_history);
    frame.render_widget(sparkline, middle_chunks[0]);

    let (p50, p95, p99) = app.metrics.get_latency_percentiles();
    let latency_info = widgets::render_latency_block(p50, p95, p99);
    frame.render_widget(latency_info, middle_chunks[1]);
}

fn render_log_panel(frame: &mut Frame, app: &DashboardApp, area: Rect) {
    let logs = app.log_buffer.lock().unwrap();
    let log_entries: Vec<_> = logs.iter().cloned().collect();
    drop(logs);

    let log_panel = widgets::render_log_panel(&log_entries, app.log_scroll);
    frame.render_widget(log_panel, area);
}
