# Decisions - monitoring-dashboard

## Architectural Choices

## Implementation Decisions


### Dependency Versions (2026-01-24)
- **ratatui 0.29**: Terminal UI framework for rendering dashboard widgets
- **crossterm 0.28**: Cross-platform terminal manipulation (keyboard events, alternate screen mode)
- **sysinfo 0.32**: System information gathering (CPU, memory usage)
- Rationale: Used specific versions rather than latest to ensure stability

### Placement Strategy (2026-01-24)
- Added all three crates to the "Utilities" section (lines 55-62)
- Maintained alphabetical ordering: bytes, crossterm, dashmap, dirs, hostname, once_cell, ratatui, regex, sysinfo, uuid
- Did not add any feature flags to keep dependencies minimal
