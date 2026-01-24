use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

use super::app::{DashboardApp, InputMode};
use super::widgets;

pub fn render(frame: &mut Frame, app: &DashboardApp) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(10),
            Constraint::Min(12),
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

    let memory_bytes = app.get_memory_usage();
    let mem_gauge = widgets::render_process_memory_gauge(memory_bytes);
    frame.render_widget(mem_gauge, top_chunks[2]);
}

fn render_middle_row(frame: &mut Frame, app: &DashboardApp, area: Rect) {
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(40),
        ])
        .split(area);

    let rate_history = app.metrics.get_request_rate_history();
    let sparkline = widgets::render_sparkline("Requests/sec", &rate_history);
    frame.render_widget(sparkline, middle_chunks[0]);

    let (p50, p95, p99) = app.metrics.get_latency_percentiles();
    let latency_info = widgets::render_latency_block(p50, p95, p99);
    frame.render_widget(latency_info, middle_chunks[1]);

    let model_stats = app.metrics.get_model_stats();
    let token_panel = widgets::render_token_usage_panel(&model_stats, app.show_session_view);
    frame.render_widget(token_panel, middle_chunks[2]);
}

fn render_log_panel(frame: &mut Frame, app: &DashboardApp, area: Rect) {
    let logs = app.log_buffer.lock().unwrap();
    let log_entries: Vec<_> = logs.iter().cloned().collect();
    drop(logs);

    let filtered_logs = if app.search_query.is_empty() {
        log_entries
    } else {
        let matcher = SkimMatcherV2::default();
        log_entries
            .into_iter()
            .filter(|entry| {
                matcher
                    .fuzzy_match(&entry.message, &app.search_query)
                    .is_some()
            })
            .collect()
    };

    let (log_area, search_area) = if app.input_mode == InputMode::Search {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let title = if app.search_query.is_empty() {
        "Logs (/ search, ↑↓ scroll, s tokens)".to_string()
    } else {
        format!(
            "Logs [filter: {}] ({} matches)",
            app.search_query,
            filtered_logs.len()
        )
    };

    let log_panel = widgets::render_log_panel_with_title(&filtered_logs, app.log_scroll, &title);
    frame.render_widget(log_panel, log_area);

    if let Some(search_area) = search_area {
        let search_widget = widgets::render_search_input(&app.search_input);
        frame.render_widget(search_widget, search_area);

        frame.set_cursor_position((
            search_area.x + app.search_input.visual_cursor() as u16 + 1,
            search_area.y + 1,
        ));
    }
}
