# ADR-001: Language Choice — Rust

## Status
Accepted

## Context
We need a language for the core engine that supports:
- WASM sandboxing for plugins
- High-performance file I/O
- Cross-platform reliability
- Long-term maintainability

## Decision
**Rust** for the core engine and all plugins.

## Alternatives Considered

### TypeScript/Node.js
- Pros: Massive npm ecosystem, fast development, easy plugin authoring
- Cons: V8 isolates are limited for sandboxing, GC pauses for large files, weaker typing

### C++
- Pros: Native performance, mature ecosystem
- Cons: Manual memory management, header/ABI complexity, build system fragmentation

## Rationale

1. **WASM sandboxing**: Rust has first-class WASM support via wasmtime. Plugins compile to `.wasm` files with memory/timeout limits.

2. **Memory safety**: The borrow checker prevents entire categories of bugs at compile time. Critical for handling untrusted file inputs.

3. **Performance**: Zero-cost abstractions, no GC pauses. Streaming large files (10GB+) requires predictable memory behavior.

4. **Cargo**: The best build system in any language. Workspace crates, dependency management, cross-compilation — all excellent.

5. **Type system**: Strict typing prevents regressions. No `any` types, no implicit casts. The IR data models are fully typed.

6. **Cross-platform**: Excellent support for Windows, macOS, Linux with consistent behavior.

## Consequences

- Higher initial development cost (learning curve)
- Plugin authors must know Rust
- Smaller talent pool than TypeScript
- Mitigated by clear API docs and examples
