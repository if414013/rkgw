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


## Metrics Instrumentation in Route Handlers

### Implementation Pattern
Successfully instrumented both OpenAI and Anthropic route handlers with metrics collection using a RAII guard pattern:

1. **RequestGuard struct**: Automatically manages request lifecycle
   - Calls `record_request_start()` in constructor
   - Calls `record_request_end()` in `complete()` method with latency/tokens
   - Implements Drop trait to decrement active connections on any exit path
   - Prevents connection leaks even when errors occur

2. **Error Recording**: All error paths record error type via `inspect_err()`
   - Used `inspect_err()` instead of `map_err()` for cleaner code (clippy suggestion)
   - Error type mapping function converts ApiError variants to metric strings

3. **Token Counting**:
   - Input tokens: Calculated before request using existing tokenizer functions
   - Output tokens (streaming): Set to 0 (accurate count unavailable until stream completes)
   - Output tokens (non-streaming): Extracted from JSON response using `.get()` chain

### Key Technical Decisions

1. **Made `active_connections` field public** in MetricsCollector
   - Required for Drop implementation to decrement counter
   - Alternative would be adding a public method, but direct access is simpler

2. **Used `inspect_err()` over `map_err()`**
   - Clippy recommendation for side-effect-only error handling
   - Cleaner than `map_err(|e| { side_effect(); e })`

3. **Used `std::io::Error::other()` over `Error::new(ErrorKind::Other, ...)`**
   - Clippy recommendation for simpler error construction
   - Available in Rust 1.74+

### Files Modified
- `src/routes/mod.rs`: Added metrics instrumentation to both handlers
- `src/main.rs`: Initialize MetricsCollector and add to AppState
- `src/middleware/mod.rs`: Update test helper with metrics
- `src/metrics/collector.rs`: Made active_connections public

### Verification
- ✅ `cargo build` succeeds
- ✅ `cargo clippy` shows 0 warnings in routes/mod.rs
- ✅ All error paths record metrics
- ✅ Guard pattern ensures cleanup on all exit paths

## Keyboard Event Handler Implementation (Task 5)

### Crossterm Event Handling Pattern
Successfully implemented non-blocking keyboard input using crossterm's poll/read pattern:

1. **Non-blocking Poll**: Use `event::poll(Duration)` with 250ms timeout
   - Returns `Ok(true)` if event is available
   - Returns `Ok(false)` if timeout expires (no blocking)
   - Allows main loop to continue rendering at ~4 fps

2. **Event Reading**: Only call `event::read()` after successful poll
   - Guaranteed not to block since poll confirmed event availability
   - Pattern: `if event::poll(timeout)? { let event = event::read()?; }`

3. **Key Event Filtering**: Check `KeyEvent.kind` to ignore key release events
   - Only process `KeyEventKind::Press` events
   - Prevents double-triggering on key press/release

### Key Bindings Implemented
- **Quit**: `q`, `Esc`, or `Ctrl+C` → sets `should_quit = true`
- **Toggle Dashboard**: `d` → toggles `dashboard_visible`
- **Scroll**: `Up`/`Down` → single line scroll
- **Page Scroll**: `PageUp`/`PageDown` → scroll by 10 lines
- **Home**: Jump to top (`log_scroll = 0`)

### Technical Decisions

1. **Saturating Arithmetic**: Used `saturating_sub()` and `saturating_add()`
   - Prevents underflow when scrolling up at position 0
   - Prevents overflow on scroll down (though unbounded in practice)

2. **Modifier Checking**: Used `KeyModifiers::contains()` for Ctrl+C
   - Bitflag-based checking for modifier keys
   - Pattern: `if key.modifiers.contains(KeyModifiers::CONTROL)`

3. **Match Exhaustiveness**: Used `_ => {}` for unhandled keys
   - Explicitly ignores other keys without action
   - Cleaner than multiple if-let chains

### Testing Strategy
Implemented 12 unit tests covering:
- All quit key combinations (q, Esc, Ctrl+C)
- Dashboard visibility toggle
- Scroll operations (up, down, page up/down, home)
- Boundary conditions (scroll at zero, page up at low position)
- Ignored keys (verify state unchanged)

### Code Organization
- `handle_events()`: Public API, polls and dispatches to handler
- `handle_key_event()`: Private function, processes individual key events
- Clean separation: polling logic vs. key mapping logic

### Integration Points
- Modifies `DashboardApp` state fields: `should_quit`, `dashboard_visible`, `log_scroll`
- Ready for integration in main dashboard loop (Task 7)
- No dependencies on other dashboard modules (app.rs only)

### Verification
- ✅ `cargo build --lib` succeeds
- ✅ `cargo test --lib event_handler::` passes (12/12 tests)
- ✅ No clippy warnings in event_handler.rs
- ✅ Commit created: `feat(dashboard): add keyboard event handler`

## Task 6: Add --dashboard CLI flag (2026-01-24)

### Implementation
- Added `dashboard: bool` field to `CliArgs` struct with `#[arg(long, default_value = "false")]`
- Added `dashboard: bool` field to `Config` struct in new "Dashboard" section
- Added `dashboard: args.dashboard` to Config::load() method
- Imported `std::io::IsTerminal` trait for TTY detection

### TTY Detection
- Added validation in `Config::validate()` that checks `std::io::stdout().is_terminal()`
- Returns error: "--dashboard requires a terminal (TTY). Cannot run dashboard mode when stdout is not a terminal."
- Uses stable Rust API (IsTerminal trait, stable since Rust 1.70)

### Verification
- `cargo run --release -- --help` shows: "Enable monitoring dashboard TUI"
- `cargo build` succeeds with no errors (only warnings for unused dashboard code)
- Flag is opt-in (default: false), no short flag (to avoid conflict with `-d` for db_file)

### Pattern Consistency
- Follows existing clap derive pattern in CliArgs
- Follows existing section comment pattern in Config struct
- Doc comment on CLI arg becomes help text (functional, not just documentation)

## Dashboard Integration into main.rs

### Conditional Initialization Pattern
- Created shared log buffer (`Arc<Mutex<VecDeque<LogEntry>>>`) before tracing init
- Conditional tracing setup:
  - Dashboard mode: Uses `DashboardLayer` to capture logs into buffer
  - Normal mode: Uses standard `fmt()` subscriber with file/line numbers
- Both modes use same `EnvFilter` for consistent log level filtering

### Server Startup Flow
- Dashboard mode:
  - Spawns dashboard in separate tokio task
  - Uses `tokio::select!` to run server and dashboard concurrently
  - Either task ending triggers shutdown of the other
  - No startup banner (dashboard provides UI)
- Normal mode:
  - Prints startup banner
  - Runs server with standard graceful shutdown
  - Logs to stdout with formatting

### Terminal Management
- `enable_raw_mode()` before terminal setup
- `EnterAlternateScreen` to preserve existing terminal content
- Panic hook ensures terminal restoration on crash
- Proper cleanup: `disable_raw_mode()`, `LeaveAlternateScreen`, `show_cursor()`

### Ownership Pattern
- Cloned `metrics` Arc before building app (app_state consumes it)
- Passed cloned references to dashboard task
- Log buffer shared between tracing layer and dashboard

### Test Configuration Updates
- Added `dashboard: false` to all test Config structs in:
  - src/middleware/mod.rs
  - src/routes/mod.rs
  - src/converters/openai_to_kiro.rs

