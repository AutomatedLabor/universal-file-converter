# ADR-002: WASM Sandboxing (Hybrid Model)

## Status
Accepted

## Context
Plugins must be isolated from the host and each other. A malicious or buggy plugin must never crash the main application or access unauthorized resources.

## Decision
**WASM (wasmtime)** as default sandbox; **process isolation** as fallback for plugins requiring native libraries.

## Alternatives Considered

### Pure WASM
- Pros: Strongest isolation, portable binaries
- Cons: Limited system access (no native codecs), slower for complex operations

### Process Isolation Only
- Pros: Full system access, simple implementation
- Cons: Higher overhead, platform-specific, harder to enforce resource limits

### V8 Isolates
- Pros: Fast startup, familiar to JS developers
- Cons: Not designed for file processing, limited memory controls

## Rationale

1. **Most plugins are pure Rust**: Image, document, archive, and structured data plugins have no native dependencies. They compile cleanly to WASM.

2. **Video/audio need native libs**: FFmpeg, some audio codecs require C libraries. These run as child processes with resource limits.

3. **WASM provides**:
   - Memory limits per instance (256MB default)
   - CPU time limits
   - No filesystem access (I/O via host callbacks)
   - No network access
   - Panic/crash does not affect host

4. **Process isolation provides**:
   - Full system access for native codecs
   - Killed on timeout or resource excess
   - Communication via protobuf over stdin/stdout

## Consequences

- Most plugins are portable `.wasm` files
- Video plugins require platform-specific native binaries
- Plugin API is designed to work in both contexts
- Testing must cover both sandbox modes
