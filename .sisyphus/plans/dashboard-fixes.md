# Dashboard Fixes: Request Rate, Streaming Tokens, Resizable Panels

## Context

### Original Request
Fix three issues with the monitoring dashboard:
1. Requests/sec sparkline is empty
2. Output tokens always showing 0 for streaming requests
3. Fixed panel sizes - want resizable sections

### Metis Analysis Summary
- **Issue 1**: `request_rate_samples` is NEVER POPULATED in `record_request_end()` - simple bug
- **Issue 2**: `guard.complete()` called BEFORE stream consumption - architectural issue
- **Issue 3**: No panel size state in `DashboardApp` - straightforward UI enhancement

---

## Work Objectives

### Core Objective
Fix request rate tracking, implement streaming token counting, and add resizable dashboard panels.

### Concrete Deliverables
- Working requests/sec sparkline showing actual request rate
- Accurate output token counts for streaming responses
- Keyboard-resizable dashboard panels

### Definition of Done
- [ ] Requests/sec sparkline shows data when requests are made *(needs manual QA)*
- [ ] Token Usage panel shows non-zero output tokens for streaming requests *(needs manual QA)*
- [ ] Panels can be resized with keyboard shortcuts *(needs manual QA)*
- [x] `cargo test --lib` passes (224/224)
- [x] `cargo clippy` has no new warnings

### Must Have
- Request rate samples populated in metrics collector
- Streaming token tracking via callback mechanism
- Panel resize state and keyboard handlers
- Min/max bounds on panel sizes

### Must NOT Have (Guardrails)
- Do NOT modify streaming response format or timing
- Do NOT block response stream waiting for metrics
- Do NOT allow panels to resize below usable minimums
- Do NOT change existing API behavior

---

## Task Flow

```
Task 1 (Request Rate Fix) - Simple bug fix
    ↓
Task 2 (Resizable Panels) - UI state management
    ↓
Task 3 (Streaming Tokens) - Architectural change
```

---

## TODOs

- [x] 1. Fix Request Rate Tracking

  **What to do**:
  - In `src/metrics/collector.rs`, add population of `request_rate_samples` in `record_request_end()`
  - Follow existing `latency_samples` pattern at lines 121-126
  - Store `(Instant::now(), 1)` for each completed request
  - Update `get_request_rate_history()` to return meaningful data for sparkline

  **Must NOT do**:
  - Do NOT change the ring buffer capacity
  - Do NOT modify existing `get_request_rate()` calculation

  **References**:
  - `src/metrics/collector.rs:121-126` - Pattern for latency_samples population
  - `src/metrics/collector.rs:282-290` - get_request_rate_history() that reads the data
  - `src/dashboard/ui.rs:57-59` - Where sparkline is rendered

  **Acceptance Criteria**:
  - [ ] `request_rate_samples` populated on each request completion
  - [ ] Sparkline shows data points when requests are made
  - [ ] `cargo test --lib metrics::` passes

  **Commit**: YES
  - Message: `fix(metrics): populate request_rate_samples for sparkline`
  - Files: `src/metrics/collector.rs`

---

- [x] 2. Add Resizable Dashboard Panels

  **What to do**:
  - Add panel size state to `DashboardApp` in `src/dashboard/app.rs`:
    ```rust
    pub middle_panel_height: u16,  // default: 10
    pub log_panel_height: u16,     // default: 15
    ```
  - Add keyboard handlers in `src/dashboard/event_handler.rs`:
    - `+` / `=` - increase log panel height
    - `-` / `_` - decrease log panel height
    - `[` - decrease middle panel height
    - `]` - increase middle panel height
  - Modify `src/dashboard/ui.rs` to use dynamic constraints from app state
  - Enforce min/max bounds (middle: 5-20, log: 8-30)

  **Must NOT do**:
  - Do NOT allow panels below minimum usable height
  - Do NOT persist sizes across restarts (keep simple)

  **References**:
  - `src/dashboard/app.rs:17-32` - DashboardApp struct
  - `src/dashboard/event_handler.rs:29-64` - Existing key handlers
  - `src/dashboard/ui.rs:10-17` - Current fixed constraints

  **Acceptance Criteria**:
  - [ ] `+`/`-` keys resize log panel
  - [ ] `[`/`]` keys resize middle panel
  - [ ] Panels respect min/max bounds
  - [ ] `cargo test --lib dashboard::` passes

  **Commit**: YES
  - Message: `feat(dashboard): add resizable panels with keyboard controls`
  - Files: `src/dashboard/app.rs`, `src/dashboard/event_handler.rs`, `src/dashboard/ui.rs`

---

- [x] 3. Fix Streaming Output Token Tracking

  **What to do**:
  - Create a `StreamingMetrics` struct to track tokens during stream:
    ```rust
    pub struct StreamingMetrics {
        pub output_tokens: Arc<AtomicU64>,
        metrics: Arc<MetricsCollector>,
        model: String,
        input_tokens: u64,
        start_time: Instant,
    }
    ```
  - Modify `RequestGuard` to support deferred completion for streaming
  - In streaming handlers (`src/routes/mod.rs`):
    - Don't call `guard.complete()` immediately for streaming
    - Pass `StreamingMetrics` into the stream wrapper
    - Update token count when usage event arrives in stream
    - Call metrics recording in `Drop` impl or stream completion
  - Parse usage from final SSE chunk in `src/streaming/mod.rs`

  **Must NOT do**:
  - Do NOT block response stream waiting for metrics
  - Do NOT modify response format
  - Do NOT break non-streaming token tracking

  **References**:
  - `src/routes/mod.rs:324` - OpenAI streaming guard.complete(0)
  - `src/routes/mod.rs:529` - Anthropic streaming guard.complete(0)
  - `src/streaming/mod.rs:622-663` - Where usage event arrives
  - `src/routes/mod.rs:66-73` - Current RequestGuard.complete()

  **Acceptance Criteria**:
  - [ ] Streaming requests show actual output token counts
  - [ ] Non-streaming requests still work correctly
  - [ ] Interrupted streams don't cause panics
  - [ ] `cargo test --lib` passes

  **Commit**: YES
  - Message: `feat(metrics): track output tokens for streaming responses`
  - Files: `src/metrics/collector.rs`, `src/routes/mod.rs`, `src/streaming/mod.rs`

---

## Success Criteria

### Verification Commands
```bash
cargo build --release    # Expected: success
cargo test --lib         # Expected: all tests pass
cargo clippy             # Expected: no new warnings
```

### Manual Verification
- [ ] Start dashboard with `--dashboard` flag
- [ ] Make streaming request, verify output tokens > 0
- [ ] Verify requests/sec sparkline shows activity
- [ ] Press `+`/`-` to resize log panel
- [ ] Press `[`/`]` to resize middle panel
