use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline};

use super::app::LogEntry;

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
            .title("Logs (↑/↓ to scroll)"),
    )
}
