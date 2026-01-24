# Monitoring Dashboard for Gateway

## Context

### Original Request
Add a live terminal monitoring dashboard to the Rust proxy gateway that displays active connections, request latency, CPU/memory usage, request rate, error rate, and token counts in real-time.

### Interview Summary
**Key Discussions**:
- Display: Live Terminal Dashboard using ratatui (NOT Prometheus endpoint)
- Metrics: Active connections, request latency (p50/p95/p99), CPU & memory, request rate, error rate, token counts, per-model stats
- Architecture: Integrated with server, toggle via `--dashboard` flag AND 'd' keyboard shortcut at runtime
- Refresh: 250ms (4 fps)
- History: 15 minutes of data in charts/sparklines
- Logs: Show in scrollable panel within dashboard
- Layout: Responsive (adapts to terminal size)
- Tests: After implementation

**Research Findings**:
- Codebase uses `Arc<RwLock<T>>`, `Arc<DashMap>` for shared state
- `AppState` struct exists - can extend with metrics collector
- `tracing` crate already in use for logging
- Natural collection points: `routes/mod.rs`, `http_client.rs`, `streaming/mod.rs`
- Recommended crates: `metrics` (0.24), `sysinfo`, `ratatui`, `crossterm`

### Metis Review
**Identified Gaps** (addressed):
- Tracing integration: Logs go to stdout, dashboard takes over stdout - need custom tracing Layer to capture logs to ring buffer
- Log panel scope: Default to `info` level and above
- Dashboard exit behavior: 'q' terminates server (same as Ctrl+C)
- Non-TTY fallback: Error and exit if `--dashboard` passed but stdout isn't a terminal
- Runtime toggle complexity: User confirmed they want both flag AND 'd' key toggle

---

## Work Objectives

### Core Objective
Add a real-time terminal monitoring dashboard that displays gateway metrics (connections, latency, throughput, errors, resources) with responsive layout and integrated log viewing.

### Concrete Deliverables
- `src/metrics/mod.rs` - Metrics collection module with ring buffers
- `src/metrics/collector.rs` - MetricsCollector struct with atomic counters and histograms
- `src/dashboard/mod.rs` - Main dashboard module with ratatui TUI
- `src/dashboard/widgets.rs` - Custom widgets (gauges, sparklines, tables)
- `src/dashboard/log_layer.rs` - Custom tracing Layer for log capture
- `src/dashboard/event_handler.rs` - Keyboard event handling
- Modified `src/main.rs` - Conditional startup, dashboard integration
- Modified `src/config.rs` - `--dashboard` CLI flag
- Modified `src/routes/mod.rs` - Instrumented with metrics calls
- Modified `Cargo.toml` - New dependencies

### Definition of Done
- [ ] `cargo run --release -- --dashboard` shows live TUI with all metrics
- [ ] Pressing 'd' toggles dashboard on/off at runtime
- [ ] Pressing 'q' exits the application cleanly
- [ ] Dashboard adapts layout to terminal size
- [ ] Log panel shows recent tracing output (info+)
- [ ] All metrics update at 4 fps (250ms)
- [ ] Charts show 15 minutes of history
- [ ] `cargo test --lib` passes with new tests
- [ ] `cargo clippy` has no warnings

### Must Have
- Active connections counter (increment on request start, decrement on end)
- Request latency histogram with p50, p95, p99 percentiles
- CPU and memory usage gauges (via sysinfo)
- Request rate sparkline (requests per second)
- Error rate by type (auth, timeout, upstream, validation)
- Token counts (input/output per request and totals)
- Per-model statistics breakdown
- Scrollable log panel with recent tracing output
- `--dashboard` CLI flag
- 'd' key runtime toggle
- 'q' key to quit
- Responsive layout

### Must NOT Have (Guardrails)
- NO persistent storage / saving metrics to disk
- NO Prometheus `/metrics` HTTP endpoint
- NO alerting or threshold notifications
- NO first token latency tracking
- NO external metrics aggregation services
- DO NOT modify existing API behavior or response formats
- DO NOT add blocking operations in the metrics hot path
- DO NOT use `std::sync` primitives in async code (use `tokio::sync`)

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES (tokio-test, mockito, proptest)
- **User wants tests**: YES (Tests-after)
- **Framework**: `cargo test --lib`

### Manual Execution Verification
Each TODO includes verification steps using the terminal to confirm functionality.

---

## Task Flow

```
Task 0 (Dependencies)
    ↓
Task 1 (Metrics Module) ──┬──► Task 3 (Instrument Handlers)
    ↓                     │
Task 2 (Dashboard Module) ◄┘
    ↓
Task 4 (Log Layer)
    ↓
Task 5 (Event Handler)
    ↓
Task 6 (CLI Flag)
    ↓
Task 7 (Main Integration)
    ↓
Task 8 (Runtime Toggle)
    ↓
Task 9 (Tests)
```

## Parallelization

| Group | Tasks | Reason |
|-------|-------|--------|
| A | 1, 2 | Independent modules after dependencies |

| Task | Depends On | Reason |
|------|------------|--------|
| 1 | 0 | Needs metrics crate |
| 2 | 0 | Needs ratatui/crossterm |
| 3 | 1 | Needs MetricsCollector |
| 4 | 2 | Needs dashboard module structure |
| 5 | 2 | Needs dashboard module structure |
| 6 | 0 | Only needs clap |
| 7 | 1, 2, 4, 5, 6 | Integrates all components |
| 8 | 7 | Needs basic integration working |
| 9 | 8 | Tests full functionality |

---

## TODOs

- [x] 0. Add Dependencies to Cargo.toml

  **What to do**:
  - Add `ratatui = "0.29"` to dependencies
  - Add `crossterm = "0.28"` to dependencies
  - Add `sysinfo = "0.32"` to dependencies
  - Keep existing dependencies unchanged

  **Must NOT do**:
  - Do NOT add prometheus or metrics-exporter crates
  - Do NOT remove any existing dependencies

  **Parallelizable**: NO (must be first)

  **References**:
  - `Cargo.toml` - Current dependencies list, add new ones in alphabetical order within sections

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds with new dependencies
  - [ ] `cargo clippy` has no new warnings

  **Commit**: YES
  - Message: `feat(deps): add ratatui, crossterm, sysinfo for monitoring dashboard`
  - Files: `Cargo.toml`, `Cargo.lock`

---

- [x] 1. Create Metrics Collection Module

  **What to do**:
  - Create `src/metrics/mod.rs` with module exports
  - Create `src/metrics/collector.rs` with `MetricsCollector` struct containing:
    - `active_connections: AtomicU64` - current active requests
    - `total_requests: AtomicU64` - lifetime request count
    - `total_errors: AtomicU64` - lifetime error count
    - `errors_by_type: DashMap<String, AtomicU64>` - errors keyed by type
    - `latency_samples: Mutex<VecDeque<(Instant, f64)>>` - ring buffer for latency (15 min)
    - `request_rate_samples: Mutex<VecDeque<(Instant, u64)>>` - ring buffer for rate
    - `token_counts: Mutex<VecDeque<(Instant, u64, u64)>>` - (time, input, output)
    - `per_model_stats: DashMap<String, ModelStats>` - per-model breakdown
  - Implement methods:
    - `new()` - constructor
    - `record_request_start()` - increment active connections
    - `record_request_end(latency_ms: f64, model: &str, input_tokens: u64, output_tokens: u64)` - decrement active, record latency/tokens
    - `record_error(error_type: &str)` - increment error counters
    - `get_active_connections() -> u64`
    - `get_latency_percentiles() -> (f64, f64, f64)` - p50, p95, p99
    - `get_request_rate() -> f64` - requests per second (last minute)
    - `get_error_rate() -> f64` - errors per second
    - `get_token_totals() -> (u64, u64)` - total input, output
    - `get_model_stats() -> Vec<(String, ModelStats)>`
    - `cleanup_old_samples()` - remove samples older than 15 minutes
  - Create `ModelStats` struct with request count, total latency, token counts
  - Use `std::sync::atomic::Ordering::Relaxed` for counters (performance)
  - Ring buffers should hold ~3600 samples (15 min at 4 samples/sec)

  **Must NOT do**:
  - Do NOT use blocking locks in async context
  - Do NOT store unbounded data (use ring buffers)
  - Do NOT add Prometheus export functionality

  **Parallelizable**: YES (with Task 2, after Task 0)

  **References**:
  - `src/cache.rs` - Example of DashMap usage pattern in this codebase
  - `src/auth/manager.rs:15-30` - Example of Arc<RwLock<T>> pattern for shared state
  - `src/streaming/mod.rs:45-60` - Example of Mutex usage for accumulating state

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy` has no warnings
  - [ ] Unit test: `record_request_start` increments active connections
  - [ ] Unit test: `record_request_end` decrements active connections and records latency
  - [ ] Unit test: `get_latency_percentiles` returns correct p50/p95/p99 for known data
  - [ ] Unit test: Ring buffer discards samples older than 15 minutes

  **Commit**: YES
  - Message: `feat(metrics): add metrics collection module with ring buffers`
  - Files: `src/metrics/mod.rs`, `src/metrics/collector.rs`
  - Pre-commit: `cargo test --lib metrics::`

---

- [x] 2. Create Dashboard Module Structure

  **What to do**:
  - Create `src/dashboard/mod.rs` with module exports
  - Create `src/dashboard/app.rs` with `DashboardApp` struct containing:
    - `metrics: Arc<MetricsCollector>` - shared metrics reference
    - `log_buffer: Arc<Mutex<VecDeque<LogEntry>>>` - shared log buffer
    - `system: sysinfo::System` - for CPU/memory
    - `should_quit: bool` - exit flag
    - `dashboard_visible: bool` - toggle state
  - Create `src/dashboard/ui.rs` with `render(frame: &mut Frame, app: &DashboardApp)` function:
    - Use `ratatui::layout::Layout` for responsive grid
    - Top row: Active connections gauge, CPU gauge, Memory gauge
    - Middle row: Request rate sparkline, Latency chart
    - Bottom row: Error table, Token counts, Log panel
  - Create `src/dashboard/widgets.rs` with helper functions:
    - `render_gauge(title: &str, value: f64, max: f64) -> Gauge`
    - `render_sparkline(title: &str, data: &[u64]) -> Sparkline`
    - `render_table(headers: &[&str], rows: Vec<Vec<String>>) -> Table`
    - `render_log_panel(logs: &[LogEntry]) -> List`
  - Create `LogEntry` struct: `timestamp: DateTime<Utc>`, `level: Level`, `message: String`

  **Must NOT do**:
  - Do NOT implement event handling yet (Task 5)
  - Do NOT implement log capture yet (Task 4)
  - Do NOT add any HTTP endpoints

  **Parallelizable**: YES (with Task 1, after Task 0)

  **References**:
  - ratatui docs: Layout, Gauge, Sparkline, Table, List widgets
  - `src/main.rs:270-299` - Current terminal output pattern (will be replaced)

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy` has no warnings
  - [ ] Module compiles with placeholder data
  - [ ] Layout adapts to different terminal sizes (test with `resize` in terminal)

  **Commit**: YES
  - Message: `feat(dashboard): add dashboard module structure with ratatui UI`
  - Files: `src/dashboard/mod.rs`, `src/dashboard/app.rs`, `src/dashboard/ui.rs`, `src/dashboard/widgets.rs`

---

- [ ] 3. Instrument Request Handlers with Metrics

  **What to do**:
  - Add `metrics: Arc<MetricsCollector>` field to `AppState` struct
  - Modify `chat_completions_handler` in `src/routes/mod.rs`:
    - Call `metrics.record_request_start()` at handler entry
    - Record `Instant::now()` for latency measurement
    - Call `metrics.record_request_end(latency, model, input_tokens, output_tokens)` on success
    - Call `metrics.record_error(error_type)` on failure
    - Use a guard pattern or `defer!` to ensure decrement on all exit paths
  - Modify `anthropic_messages_handler` similarly
  - Extract token counts from response (look at existing response parsing)
  - Map error types to strings: "auth", "validation", "upstream", "timeout", "internal"

  **Must NOT do**:
  - Do NOT add blocking operations
  - Do NOT modify response format
  - Do NOT add latency to responses

  **Parallelizable**: NO (depends on Task 1)

  **References**:
  - `src/routes/mod.rs:chat_completions_handler` - Main OpenAI handler to instrument
  - `src/routes/mod.rs:anthropic_messages_handler` - Anthropic handler to instrument
  - `src/error.rs:ApiError` - Error types to map to metric labels
  - `src/models/openai.rs:Usage` - Token count structure in responses

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy` has no warnings
  - [ ] Manual test: Make request, verify `get_active_connections()` increments then decrements
  - [ ] Manual test: Make request, verify latency is recorded
  - [ ] Manual test: Make invalid request, verify error is recorded

  **Commit**: YES
  - Message: `feat(routes): instrument handlers with metrics collection`
  - Files: `src/routes/mod.rs`, `src/lib.rs` (AppState)
  - Pre-commit: `cargo test --lib routes::`

---

- [ ] 4. Create Custom Tracing Layer for Log Capture

  **What to do**:
  - Create `src/dashboard/log_layer.rs` with `DashboardLayer` struct
  - Implement `tracing_subscriber::Layer` trait:
    - `on_event()` - capture log events to shared ring buffer
    - Extract: timestamp, level, message, file, line number
    - Filter to `info` level and above by default
  - Ring buffer capacity: 1000 entries (configurable)
  - `LogEntry` struct: timestamp, level, target, message
  - Thread-safe: use `Arc<Mutex<VecDeque<LogEntry>>>`
  - Implement `DashboardLayer::new(buffer: Arc<Mutex<VecDeque<LogEntry>>>) -> Self`

  **Must NOT do**:
  - Do NOT write to stdout when dashboard is active
  - Do NOT block on mutex (use try_lock with fallback)
  - Do NOT capture debug/trace levels (too noisy)

  **Parallelizable**: NO (depends on Task 2)

  **References**:
  - `src/main.rs:41-47` - Current tracing subscriber setup (will be conditionally replaced)
  - tracing-subscriber docs: Layer trait implementation
  - `src/middleware/debug.rs` - Example of capturing request/response data

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy` has no warnings
  - [ ] Unit test: Layer captures info-level events
  - [ ] Unit test: Layer ignores debug-level events
  - [ ] Unit test: Ring buffer evicts oldest when full

  **Commit**: YES
  - Message: `feat(dashboard): add custom tracing layer for log capture`
  - Files: `src/dashboard/log_layer.rs`, `src/dashboard/mod.rs`
  - Pre-commit: `cargo test --lib dashboard::log_layer::`

---

- [ ] 5. Create Keyboard Event Handler

  **What to do**:
  - Create `src/dashboard/event_handler.rs` with event handling logic
  - Implement `handle_events(app: &mut DashboardApp) -> io::Result<()>`:
    - Use `crossterm::event::poll()` with 250ms timeout
    - Handle `KeyCode::Char('q')` - set `should_quit = true`
    - Handle `KeyCode::Char('d')` - toggle `dashboard_visible`
    - Handle `KeyCode::Up/Down` - scroll log panel
    - Handle `KeyCode::Esc` - same as 'q'
  - Non-blocking: use `poll()` with timeout, not `read()`
  - Integrate with main event loop

  **Must NOT do**:
  - Do NOT block indefinitely on input
  - Do NOT handle mouse events (keep simple)

  **Parallelizable**: NO (depends on Task 2)

  **References**:
  - crossterm docs: event module, KeyCode enum
  - `src/dashboard/app.rs` - DashboardApp struct to modify

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy` has no warnings
  - [ ] Manual test: Press 'q' exits application
  - [ ] Manual test: Press 'd' toggles dashboard visibility
  - [ ] Manual test: Up/Down scrolls log panel

  **Commit**: YES
  - Message: `feat(dashboard): add keyboard event handler`
  - Files: `src/dashboard/event_handler.rs`, `src/dashboard/mod.rs`

---

- [ ] 6. Add --dashboard CLI Flag

  **What to do**:
  - Modify `src/config.rs` to add `dashboard: bool` field to Config
  - Add CLI argument parsing for `--dashboard` flag
  - Default to `false` (dashboard disabled)
  - Add TTY detection: if `--dashboard` and not TTY, print error and exit
  - Use `std::io::IsTerminal` trait (Rust 1.70+) or `atty` crate

  **Must NOT do**:
  - Do NOT change existing config fields
  - Do NOT make dashboard the default

  **Parallelizable**: YES (only depends on Task 0)

  **References**:
  - `src/config.rs` - Current config structure and CLI parsing
  - `src/main.rs:20-40` - Config loading in main

  **Acceptance Criteria**:
  - [ ] `cargo run --release -- --help` shows `--dashboard` option
  - [ ] `cargo run --release -- --dashboard` enables dashboard mode
  - [ ] Running with `--dashboard` when stdout is not TTY prints error

  **Commit**: YES
  - Message: `feat(config): add --dashboard CLI flag`
  - Files: `src/config.rs`
  - Pre-commit: `cargo test --lib config::`

---

- [ ] 7. Integrate Dashboard into Main

  **What to do**:
  - Modify `src/main.rs` to conditionally initialize dashboard:
    - If `config.dashboard`:
      - Create shared `log_buffer: Arc<Mutex<VecDeque<LogEntry>>>`
      - Create `DashboardLayer` with log_buffer
      - Initialize tracing with `registry().with(DashboardLayer).with(EnvFilter)`
      - Create `DashboardApp` with metrics and log_buffer
      - Initialize terminal with crossterm (alternate screen, raw mode)
      - Spawn dashboard render loop (250ms tick)
    - Else:
      - Use existing `tracing_subscriber::fmt()` setup
  - Create `MetricsCollector` and add to `AppState`
  - Dashboard render loop:
    - Poll for keyboard events
    - Refresh sysinfo (CPU/memory)
    - Call `cleanup_old_samples()` on metrics
    - Render UI
  - Handle graceful shutdown: restore terminal on exit/panic

  **Must NOT do**:
  - Do NOT remove existing logging when dashboard is disabled
  - Do NOT block the main server task with dashboard rendering

  **Parallelizable**: NO (depends on Tasks 1-6)

  **References**:
  - `src/main.rs` - Current initialization flow
  - `src/main.rs:270-299` - Startup banner (skip when dashboard enabled)
  - ratatui docs: Terminal initialization, alternate screen

  **Acceptance Criteria**:
  - [ ] `cargo run --release` works as before (no dashboard)
  - [ ] `cargo run --release -- --dashboard` shows TUI
  - [ ] Dashboard displays all metrics (connections, latency, CPU, memory, rate, errors, tokens)
  - [ ] Log panel shows server logs
  - [ ] 'q' exits cleanly, terminal restored
  - [ ] Ctrl+C exits cleanly, terminal restored

  **Commit**: YES
  - Message: `feat(main): integrate dashboard with conditional startup`
  - Files: `src/main.rs`
  - Pre-commit: `cargo build --release`

---

- [ ] 8. Implement Runtime Dashboard Toggle

  **What to do**:
  - Modify event handler to support 'd' key toggle when dashboard is visible
  - When toggling OFF:
    - Leave alternate screen (restore normal terminal)
    - Resume normal log output to stdout
    - Keep metrics collection running
  - When toggling ON:
    - Enter alternate screen
    - Suppress stdout logging (capture to buffer only)
    - Resume dashboard rendering
  - This requires dynamic subscriber switching or dual-output layer
  - Alternative: Always capture to buffer, conditionally render to stdout OR dashboard

  **Must NOT do**:
  - Do NOT lose log entries during toggle
  - Do NOT stop metrics collection during toggle

  **Parallelizable**: NO (depends on Task 7)

  **References**:
  - `src/dashboard/event_handler.rs` - Keyboard handling
  - `src/dashboard/log_layer.rs` - Log capture layer
  - crossterm docs: `EnterAlternateScreen`, `LeaveAlternateScreen`

  **Acceptance Criteria**:
  - [ ] Press 'd' hides dashboard, shows normal logs
  - [ ] Press 'd' again shows dashboard
  - [ ] Metrics continue updating during toggle
  - [ ] No log entries lost during toggle

  **Commit**: YES
  - Message: `feat(dashboard): implement runtime toggle with 'd' key`
  - Files: `src/dashboard/event_handler.rs`, `src/dashboard/app.rs`, `src/main.rs`

---

- [ ] 9. Add Tests for Metrics and Dashboard

  **What to do**:
  - Add unit tests to `src/metrics/collector.rs`:
    - Test atomic counter operations
    - Test ring buffer capacity limits
    - Test percentile calculations
    - Test cleanup of old samples
  - Add unit tests to `src/dashboard/log_layer.rs`:
    - Test log capture at different levels
    - Test ring buffer eviction
  - Add integration test for metrics flow:
    - Create MetricsCollector
    - Simulate request start/end
    - Verify metrics values

  **Must NOT do**:
  - Do NOT test TUI rendering (too complex, manual verification sufficient)
  - Do NOT add flaky timing-dependent tests

  **Parallelizable**: NO (depends on Task 8)

  **References**:
  - `src/converters/openai_to_kiro.rs` - Example test patterns in codebase
  - `src/auth/manager.rs` - Example async test patterns

  **Acceptance Criteria**:
  - [ ] `cargo test --lib metrics::` passes
  - [ ] `cargo test --lib dashboard::log_layer::` passes
  - [ ] All tests are deterministic (no flaky failures)

  **Commit**: YES
  - Message: `test(metrics,dashboard): add unit tests for metrics and log layer`
  - Files: `src/metrics/collector.rs`, `src/dashboard/log_layer.rs`
  - Pre-commit: `cargo test --lib`

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 0 | `feat(deps): add ratatui, crossterm, sysinfo` | Cargo.toml | `cargo build` |
| 1 | `feat(metrics): add metrics collection module` | src/metrics/* | `cargo test --lib metrics::` |
| 2 | `feat(dashboard): add dashboard module structure` | src/dashboard/* | `cargo build` |
| 3 | `feat(routes): instrument handlers with metrics` | src/routes/mod.rs | `cargo test --lib routes::` |
| 4 | `feat(dashboard): add custom tracing layer` | src/dashboard/log_layer.rs | `cargo test --lib dashboard::` |
| 5 | `feat(dashboard): add keyboard event handler` | src/dashboard/event_handler.rs | `cargo build` |
| 6 | `feat(config): add --dashboard CLI flag` | src/config.rs | `cargo test --lib config::` |
| 7 | `feat(main): integrate dashboard` | src/main.rs | `cargo run --release -- --dashboard` |
| 8 | `feat(dashboard): implement runtime toggle` | src/dashboard/* | manual test |
| 9 | `test(metrics,dashboard): add unit tests` | src/metrics/*, src/dashboard/* | `cargo test --lib` |

---

## Success Criteria

### Verification Commands
```bash
cargo build --release          # Expected: success, no errors
cargo clippy                   # Expected: no warnings
cargo test --lib               # Expected: all tests pass
cargo run --release -- --dashboard  # Expected: TUI appears with metrics
```

### Final Checklist
- [ ] All "Must Have" features present
- [ ] All "Must NOT Have" guardrails respected
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Dashboard responsive to terminal resize
- [ ] Clean exit on 'q' or Ctrl+C
- [ ] Runtime toggle works with 'd' key
