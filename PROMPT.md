# Universal File Converter — Project Brief

## PROJECT OVERVIEW

Design and build a cross-platform desktop application: a Universal File
Converter that runs entirely offline with no LLMs, no cloud APIs, no
telemetry, and no paid services.

The application converts files across as many format categories as feasible,
preserving formatting, metadata, embedded resources, and document structure.
It uses a plugin-based architecture where every format converter is an
isolated, independently addable/removable plugin.

## TARGET USER

Technical and semi-technical professionals (developers, designers, analysts,
archivists) who need reliable, offline batch conversion with fidelity
guarantees. They value correctness and transparency over flashy UI.

## TECHNOLOGY STACK (preferred, justify deviations)

- Language: Rust (core engine + plugins) or TypeScript/Node with native
  bindings. Justify your choice based on performance needs, plugin
  sandboxing, and cross-platform requirements.
- UI framework: Tauri, Electron, or native (justify selection).
- Build system, test framework, and CI: your recommendation with rationale.

## FORMAT CATEGORIES (prioritized)

Tier 1 — Core (must work reliably):
  Documents, Images, Audio, Video, Archives

Tier 2 — Important:
  eBooks, Office Files, Vector Graphics, Fonts, CSV/JSON/XML/HTML/Markdown

Tier 3 — Extended:
  Subtitles, CAD, 3D Models, Databases, Configuration/Log files, Source Code

## KEY FEATURES (prioritized)

Tier 1 — Core:
  Drag-and-drop, batch conversion, automatic format detection, progress
  tracking, cancel/pause/resume, integrity verification (checksums),
  conversion queue, metadata preservation, robust error handling

Tier 2 — Important:
  Folder/recursive conversion, conversion profiles, preview before
  conversion, duplicate detection, conversion history, dark mode,
  keyboard shortcuts, CLI interface

Tier 3 — Extended:
  Watch folders, REST API, portable mode, localization, accessibility,
  plugin marketplace (local), file comparison, embedded asset preservation

## PLUGIN ARCHITECTURE REQUIREMENTS

Every format converter is an isolated plugin. Each plugin declares:
  - Input/output formats (with MIME types and extensions)
  - Capabilities (what it preserves: metadata, structure, embedded assets)
  - External dependencies and version requirements
  - Processing priority for format conflicts
  - Known limitations
  - Estimated fidelity score (0–100)
  - Plugin version and API compatibility

Plugins must never crash the main application. Each plugin runs in a
sandboxed context. Plugins can be added or removed independently with no
effect on the rest of the system.

## INTERMEDIATE REPRESENTATION DESIGN

Use intermediate representations to avoid N×M pairwise converters:

  Source Format → Domain-Specific IR → Target Format

Design reusable IRs for at least:
  - Documents (structure, styles, annotations)
  - Images (layers, color spaces, compression metadata)
  - Audio (sample rate, channels, codec parameters)
  - Video (frames, audio tracks, subtitles, chapters)
  - Vector graphics (paths, transforms, gradients)
  - Tables/structured data (schema, types, relations)
  - Rich text (formatting, embedded media references)
  - Archives (directory tree, compression, encryption)
  - 3D meshes (vertices, faces, materials, scene graph)

For each IR, specify: data model, serialization format, what information is
losslessly preserved, and what is lossy.

## QUALITY STANDARDS

Architecture:
  - Clean architecture with strict layer separation
  - SOLID principles, dependency injection, repository pattern where justified
  - Strict typing throughout — no any-types, no implicit casts
  - Zero duplicated logic; shared utilities for common patterns
  - No magic numbers, no unresolved TODOs

Testing:
  - Unit tests for every plugin and core module
  - Integration tests for plugin ↔ engine interactions
  - Golden file tests (known input → expected output)
  - Property-based tests for IR round-trip fidelity
  - Fuzz testing for malformed file handling
  - Performance benchmarks (throughput, memory, latency)
  - Stress tests (millions of files, very large files, concurrent conversion)

Security:
  - Sandbox all converter plugins (process isolation or WASM sandboxing)
  - Validate all file inputs before processing
  - Graceful recovery from corrupted/malicious files
  - Prevent plugin privilege escalation
  - No file I/O outside designated temp and output directories

Performance:
  - Streaming and incremental processing for large files
  - Parallel conversion across available cores
  - Configurable memory limits per conversion
  - GPU acceleration for image/video when beneficial

## WHAT TO PRODUCE — PHASE 1

1. REQUIREMENTS ANALYSIS
2. ARCHITECTURE DESIGN
3. TECHNOLOGY STACK JUSTIFICATION
4. IMPLEMENTATION ROADMAP
5. PROJECT STRUCTURE

## DEVELOPMENT PRINCIPLES

- Correctness > Maintainability > Extensibility > Performance > Convenience
- When uncertain: explain assumptions, compare alternatives, justify choice
- At every milestone boundary: identify technical debt, propose refactoring,
  verify architectural consistency
- Build as if this codebase must remain maintainable for 15 years
- No placeholders, no stubs, no "implement later"
