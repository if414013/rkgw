# Learnings - monitoring-dashboard

## Conventions Discovered

## Patterns Found

## Technical Notes


### Cargo.toml Structure (2026-01-24)
- Dependencies are organized by purpose with comment headers (Web framework, Async runtime, HTTP client, Serialization, Configuration, Logging, Error handling, Time handling, Tokenization, SQLite support, Utilities, Interactive prompts)
- Within each section, dependencies are kept in **alphabetical order**
- The Utilities section spans lines 55-62 and contains general-purpose crates

### Build System (2026-01-24)
- `Cargo.lock` is in `.gitignore` - only `Cargo.toml` should be committed
- `cargo build` successfully resolves and downloads new dependencies
- Pre-existing clippy warnings (30 total) are unrelated to dependency changes

## Dashboard Module Structure (Task 3)

### Ratatui Widget Usage
- **Gauge**: Used for progress/percentage displays (connections, CPU, memory)
  - Color-coded based on thresholds (green < 60%, yellow < 80%, red >= 80%)
  - Takes ratio (0.0-1.0) and displays with label
- **Sparkline**: Used for time-series data visualization (request rate history)
  - Takes `&[u64]` data slice
  - Renders compact inline charts
- **List**: Used for log display with scrolling
  - Each ListItem can have styled content with multiple Spans
  - Supports color-coding by log level
- **Paragraph**: Used for multi-line text with rich formatting
  - Used for latency percentiles display (p50, p95, p99)
  - Supports Line and Span for granular styling

### Layout Strategy
- Three-tier vertical layout:
  1. Top row (3 lines): Gauges for active connections, CPU, memory
  2. Middle row (flexible): Sparkline for request rate + latency info
  3. Bottom row (10 lines): Scrollable log panel
- Horizontal splits use percentage constraints for responsive sizing

### DashboardApp State Management
- Uses `Arc<MetricsCollector>` for shared metrics access
- Uses `Arc<Mutex<VecDeque<LogEntry>>>` for thread-safe log buffer
- Integrates `sysinfo::System` for CPU/memory monitoring
- Tracks UI state: quit flag, visibility, scroll position

### Code Organization
- `mod.rs`: Module exports and public API
- `app.rs`: Application state and system info methods
- `ui.rs`: Main render function and layout logic
- `widgets.rs`: Reusable widget rendering functions

### Integration Points
- Depends on `crate::metrics::MetricsCollector` from Task 1
- LogEntry struct uses `chrono::DateTime<Utc>` and `tracing::Level`
- Ready for event handling (Task 5) and log capture (Task 4)

