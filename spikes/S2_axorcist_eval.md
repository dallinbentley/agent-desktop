# S2: AXorcist Evaluation

**Date**: 2026-03-24  
**Repository**: https://github.com/steipete/AXorcist  
**Version evaluated**: Latest main branch

## Summary

**Recommendation: Roll our own** 🛠️

AXorcist is a well-built library, but it's overengineered for our use case. The raw AXUIElement API is straightforward enough that our own implementation gives us better control with less complexity.

## What AXorcist Provides

### Library Structure
- **16,887 lines** across 105 Swift source files
- SPM-compatible as a library target (`AXorcist`)
- Includes a CLI tool (`axorc`) for interactive use
- Dependencies: `swift-log`, `Commander` (CLI only)

### Key API Surface

```swift
// Main entry point - command-based architecture
let axorcist = AXorcist.shared
let response = axorcist.runCommand(AXCommandEnvelope(
    commandID: "id",
    command: .collectAll(CollectAllCommand(
        appIdentifier: "Safari",
        maxDepth: 10
    ))
))

// Element wrapper
let element = Element(axUIElement)
element.role       // String?
element.title      // String?
element.children() // [Element]?
element.performAction(.press)

// collectAll - recursive tree traversal with filtering
CollectAllCommand(
    appIdentifier: String?,
    attributesToReturn: [String]?,
    maxDepth: Int,           // default: 10
    filterCriteria: [String: String]?
)
```

### Features
- ✅ Element wrapper with type-safe properties
- ✅ `collectAll` for full tree traversal (exactly what we need)
- ✅ Query system with fuzzy matching and locators
- ✅ Action execution (press, setValue, etc.)
- ✅ Attribute extraction with value formatting
- ✅ Observation/notification subscriptions
- ✅ Batch command execution
- ✅ Cycle detection (CFHash-based visited set)
- ✅ Text extraction utilities
- ✅ Window resolution helpers

## Evaluation Criteria

### ✅ Can be added as SPM dependency?
Yes. It's a proper SPM package with a library product:
```swift
.package(url: "https://github.com/steipete/AXorcist.git", from: "x.x.x")
// Then: .product(name: "AXorcist", package: "AXorcist")
```

### ⚠️ Platform requirements
- **Swift tools version 6.2** (our project uses 5.9)
- **macOS 14.0+** (our project targets macOS 13+)
- **Swift 6 language mode** with strict concurrency

This is a breaking issue — we'd need to bump our minimum deployment target and Swift version.

### ⚠️ Does collectAll give us what we need?
Mostly. `collectAll` does recursive tree collection with depth limits and filtering. However:
- It's behind a command-envelope pattern that adds serialization overhead
- Results come back as `AXResponse` with `AnyCodable` payloads — needs deserialization
- It uses `@MainActor` isolation everywhere — limits concurrent usage
- No direct way to do our "two-pass" traversal (fast role scan + selective attribute fetch)

### ⚠️ API design alignment
AXorcist is designed as a **command-based automation tool** (think: Selenium for desktop). Our use case is a **snapshot builder** that needs raw speed. Their abstractions add overhead we don't need:
- Command envelope → execution → response → deserialization
- GlobalAXLogger for every operation
- String-keyed attribute dictionaries
- AnyCodable wrapping/unwrapping

## Comparison: AXorcist vs Raw AXUIElement

| Aspect | AXorcist | Raw API (our spike) |
|--------|----------|---------------------|
| Lines of code | 16,887 | ~200 |
| Dependencies | swift-log, Commander | None |
| Min macOS | 14.0 | 13.0 |
| Swift version | 6.2 | 5.9 |
| Tree traversal | collectAll command | Direct recursion |
| Per-element overhead | High (wrappers, logging, serialization) | Minimal |
| Concurrency | @MainActor | Flexible |
| Action execution | Full support | ~20 lines to implement |
| Edge cases handled | Many (cycles, timeouts, permissions) | Basic |
| Maintenance burden | External dependency | We own it |

## What We'd Take From AXorcist

Even though we're rolling our own, AXorcist has good ideas worth borrowing:

1. **Cycle detection**: Using `CFHash(element)` with a `Set<UInt>` to detect cycles — important for apps with circular AX hierarchies
2. **Timeout policy**: Their `AXTimeoutPolicy` for handling unresponsive apps
3. **Value formatting**: Their approach to converting AXValue types (CGPoint, CGSize, etc.)
4. **Interactive element classification**: Their role-based type checking helpers
5. **Permission handling**: Their accessibility permission check flow

## Risk Assessment

| Risk | Using AXorcist | Rolling our own |
|------|---------------|-----------------|
| API breaks | Medium (active development) | None |
| Performance ceiling | Limited by abstractions | Full control |
| Feature gaps | Low (very comprehensive) | Need to implement as needed |
| Maintenance | External | Internal |
| Bug fixes | Wait for upstream | Fix immediately |
| Swift version coupling | Must track their Swift 6.2 | Our choice |

## Decision: Roll Our Own

### Rationale

1. **Simplicity**: Our S1 spike shows the core traversal is ~200 lines. AXorcist is 16K+ lines of code we'd be pulling in for something we can write in a day.

2. **Performance**: We need < 100ms snapshot generation. AXorcist's command-envelope pattern, logging, and serialization add overhead we can't afford. Our raw traversal does 300 elements in 50ms.

3. **Version compatibility**: AXorcist requires Swift 6.2 and macOS 14. We target Swift 5.9 / macOS 13. Bumping these just for AXorcist isn't justified.

4. **Design mismatch**: AXorcist is a general-purpose automation framework. We need a focused snapshot builder. Their abstractions would fight our use case.

5. **Dependency risk**: Taking a dependency on an actively-evolving project by a single maintainer for a core system component is risky.

### What we'll build ourselves (borrowing ideas from AXorcist)
- Element wrapper struct (simpler, no AnyCodable)
- Recursive tree traversal with depth limits
- Cycle detection via CFHash
- Selective attribute extraction (role-first, then details for interactive elements)
- Action execution helpers (press, setValue)
- Accessibility permission checking
- Timeout handling for unresponsive apps
