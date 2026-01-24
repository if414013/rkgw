use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline};
use std::sync::atomic::Ordering;
use tui_input::Input;

use super::app::LogEntry;
use crate::metrics::ModelStats;

pub fn render_connections_gauge(active: u64) -> Paragraph<'static> {
    let color = if active > 10 {
        Color::Red
    } else if active > 5 {
        Color::Yellow
    } else {
        Color::Green
    };

    let text = Line::from(vec![Span::styled(
        format!("{}", active),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )]);

    Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Active Connections"),
        )
        .centered()
}

pub fn render_cpu_gauge(cpu: f64) -> Gauge<'static> {
    let ratio = (cpu / 100.0).clamp(0.0, 1.0);
    let color = if cpu > 80.0 {
        Color::Red
    } else if cpu > 60.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("CPU Usage"))
        .gauge_style(Style::default().fg(color))
        .ratio(ratio)
        .label(format!("{:.1}%", cpu))
}

pub fn render_memory_gauge(used: u64, total: u64) -> Gauge<'static> {
    let ratio = if total > 0 {
        (used as f64 / total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let percent = ratio * 100.0;

    let color = if percent > 80.0 {
        Color::Red
    } else if percent > 60.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let used_gb = used as f64 / 1024.0 / 1024.0 / 1024.0;
    let total_gb = total as f64 / 1024.0 / 1024.0 / 1024.0;

    Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Memory"))
        .gauge_style(Style::default().fg(color))
        .ratio(ratio)
        .label(format!("{:.1}/{:.1} GB", used_gb, total_gb))
}

pub fn render_process_memory_gauge(bytes: u64) -> Paragraph<'static> {
    let mb = bytes as f64 / 1024.0 / 1024.0;

    let (value_str, color) = if mb > 500.0 {
        (format!("{:.0} MB", mb), Color::Red)
    } else if mb > 200.0 {
        (format!("{:.0} MB", mb), Color::Yellow)
    } else {
        (format!("{:.1} MB", mb), Color::Green)
    };

    let text = Line::from(vec![Span::styled(
        value_str,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )]);

    Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Memory (rkgw)"),
        )
        .centered()
}

#[allow(dead_code)]
pub fn render_gauge(title: &str, value: f64, max: f64) -> Gauge<'static> {
    let ratio = (value / max).clamp(0.0, 1.0);
    let color = if ratio > 0.8 {
        Color::Red
    } else if ratio > 0.6 {
        Color::Yellow
    } else {
        Color::Green
    };

    Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_string()),
        )
        .gauge_style(Style::default().fg(color))
        .ratio(ratio)
        .label(format!("{:.1}", value))
}

pub fn render_sparkline(title: &str, data: &[u64]) -> Sparkline<'static> {
    Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_string()),
        )
        .data(data)
        .style(Style::default().fg(Color::Cyan))
}

pub fn render_latency_block(p50: f64, p95: f64, p99: f64) -> Paragraph<'static> {
    let text = vec![
        Line::from(vec![
            Span::styled("p50: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:.1}ms", p50), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("p95: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:.1}ms", p95), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("p99: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:.1}ms", p99), Style::default().fg(Color::Red)),
        ]),
    ];

    Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Latency"))
}

pub fn render_log_panel(logs: &[LogEntry], scroll: usize) -> List<'static> {
    render_log_panel_with_title(logs, scroll, "Logs (↑/↓ to scroll)")
}

pub fn render_log_panel_with_title(logs: &[LogEntry], scroll: usize, title: &str) -> List<'static> {
    let items: Vec<ListItem> = logs
        .iter()
        .skip(scroll)
        .map(|entry| {
            let level_color = match entry.level {
                tracing::Level::ERROR => Color::Red,
                tracing::Level::WARN => Color::Yellow,
                tracing::Level::INFO => Color::Green,
                tracing::Level::DEBUG => Color::Blue,
                tracing::Level::TRACE => Color::Gray,
            };

            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp.format("%H:%M:%S")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:5} ", entry.level),
                    Style::default()
                        .fg(level_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(entry.message.clone()),
            ]);

            ListItem::new(content)
        })
        .collect();

    List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title.to_string()),
    )
}

pub fn render_token_usage_panel(
    model_stats: &[(String, ModelStats)],
    _show_session: bool,
) -> Paragraph<'static> {
    let mut lines: Vec<Line> = Vec::new();

    if model_stats.is_empty() {
        lines.push(Line::from(Span::styled(
            "No requests yet",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (model, stats) in model_stats {
            let input = stats.total_input_tokens.load(Ordering::Relaxed);
            let output = stats.total_output_tokens.load(Ordering::Relaxed);
            let requests = stats.request_count.load(Ordering::Relaxed);

            let model_short = if model.len() > 15 {
                format!("{}...", &model[..12])
            } else {
                model.clone()
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<15} ", model_short),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(format!("{}r ", requests), Style::default().fg(Color::Gray)),
            ]));

            lines.push(Line::from(vec![
                Span::styled("  in:", Style::default().fg(Color::DarkGray)),
                Span::styled(format_tokens(input), Style::default().fg(Color::Green)),
                Span::styled(" out:", Style::default().fg(Color::DarkGray)),
                Span::styled(format_tokens(output), Style::default().fg(Color::Yellow)),
            ]));
        }
    }

    Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Token Usage (s toggle)"),
    )
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{}", tokens)
    }
}

pub fn render_search_input(input: &Input) -> Paragraph<'static> {
    Paragraph::new(input.value().to_string())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search (Enter to apply, Esc to cancel)")
                .border_style(Style::default().fg(Color::Yellow)),
        )
}
