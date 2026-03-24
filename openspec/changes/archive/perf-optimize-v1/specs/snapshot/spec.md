## MODIFIED Requirements

### Requirement: Snapshot performance instrumentation
The AX tree walk includes timing instrumentation at each depth level and counts total AX attribute queries. Results are logged when `--verbose` is active.

#### Scenario: Verbose snapshot profiling
- **WHEN** `snapshot -i --verbose` is executed
- **THEN** output includes per-depth-level timing, total attribute query count, and total elements visited
- **THEN** data is printed to stderr (does not affect stdout snapshot output)

#### Scenario: Normal snapshot unchanged
- **WHEN** `snapshot -i` is executed without `--verbose`
- **THEN** behavior and output are identical to current implementation
- **THEN** instrumentation adds negligible overhead (<1ms)
