# S6: Snapshot Performance Profiling

## Overview

Added instrumentation to the AX tree walk (`ax_engine.rs`) to measure:
- Per-depth-level timing and element counts
- Total AX attribute queries
- Total elements visited
- Wall-clock duration

Results collected via `agent-desktop snapshot -i --verbose --app <name>`.

## Per-App Breakdown

### Ghostty (terminal)
```
total: 8.9ms | 28 elements | 254 AX queries
per-depth breakdown:
  depth  0:    0.5ms |     1 elements | 502µs/elem
  depth  1:    2.5ms |     8 elements | 314µs/elem
  depth  2:    1.7ms |     6 elements | 288µs/elem
  depth  3:    2.5ms |    11 elements | 223µs/elem
  depth  4:    1.4ms |     2 elements | 700µs/elem
```
**Assessment:** Fast. Simple AX tree (28 elements, max depth 4). ~9 AX queries/element average.

### Finder (file manager)
```
total: 28.2ms | 23 elements | 209 AX queries
per-depth breakdown:
  depth  0:    1.9ms |     1 elements | 1892µs/elem
  depth  1:    0.8ms |     1 elements | 800µs/elem
  depth  2:   11.2ms |    21 elements | 534µs/elem
```
**Assessment:** Moderate. Shallow tree (max depth 2) but depth 0 is slow (~1.9ms). Desktop-only Finder window (no open folder).

### System Settings (native macOS)
```
total: 818.1ms | 197 elements | 1775 AX queries
per-depth breakdown:
  depth  0:    0.9ms |     1 elements | 877µs/elem
  depth  1:    6.1ms |     6 elements | 1009µs/elem
  depth  2:    2.7ms |     3 elements | 900µs/elem
  depth  3:    4.3ms |     6 elements | 709µs/elem
  depth  4:   28.3ms |     5 elements | 5661µs/elem
  depth  5:    8.5ms |     4 elements | 2122µs/elem
  depth  6:   53.9ms |    65 elements | 829µs/elem
  depth  7:  330.0ms |    64 elements | 5156µs/elem
  depth  8:  281.8ms |    43 elements | 6554µs/elem
```
**Assessment:** Very slow. 818ms for 197 elements. Bottleneck is at depth 7 and 8 — 64 and 43 elements respectively, each taking ~5-6ms. These deep elements likely have heavy AX attribute retrieval (perhaps text content or computed values). 9 AX queries/element.

### Slack (Electron/CDP)
Slack is detected as a CDP app, so AX profiling is not applicable. The snapshot goes through agent-browser. 164 interactive elements found via CDP.

## Key Findings

1. **AX query cost is ~200-700µs per element** for simple apps (Ghostty, Finder), but **~5-6ms per element** for complex native apps (System Settings depth 7-8). This is a macOS AX API characteristic.

2. **Depth is the main bottleneck** — deep trees have exponentially more elements and each deep element can be slow. System Settings depth 7+8 account for 612ms out of 818ms total (75%).

3. **~9 AX queries per element** is consistent across apps. Each element needs: batch (role, title, description, value) + position + size + actions + children + child role lookups.

4. **Electron apps bypass AX entirely** via CDP, which is a separate optimization path.

## Optimization Recommendations (future change)

- **Early pruning**: Skip subtrees that have no interactive descendants (already done in formatting, but the full tree is still walked).
- **Lazy attribute fetching**: Only fetch frame/actions for interactive elements (saves ~3 queries per non-interactive element).
- **Depth limit tuning**: Default depth 20 is excessive for most apps. Consider adaptive depth based on element count.
- **Partial tree caching**: Cache subtrees that haven't changed (needs staleness detection).
- **Parallel subtree walks**: Walk window subtrees in parallel threads (AX API is thread-safe per-element).
