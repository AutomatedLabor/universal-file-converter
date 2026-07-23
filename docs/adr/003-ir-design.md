# ADR-003: Domain-Specific IRs (Not Universal)

## Status
Accepted

## Context
We need intermediate representations to avoid N×M pairwise converters. Should we use one universal IR or multiple domain-specific IRs?

## Decision
**Separate IR per domain**: Document, Image, Audio, Video, Vector, Table, Archive, Mesh.

## Alternatives Considered

### Universal IR
- Pros: Single data model, simple routing
- Cons: Impossibly complex, would be a superset of all formats, massive overhead

### No IR (Direct Conversion)
- Pros: No intermediate data loss
- Cons: N×M converters, each pair needs its own implementation

## Rationale

1. **Each domain has unique semantics**: A document's structure (headings, paragraphs, tables) has nothing in common with an image's pixels (color spaces, layers, animation).

2. **Focused data models**: Each IR captures exactly what's meaningful for its domain. ImageIR has pixel data; DocumentIR has block structure; AudioIR has samples.

3. **Clear boundaries**: Plugins know exactly what IR they produce/consume. No confusion about which fields apply.

4. **Versioning**: Each IR is versioned independently. A breaking change in AudioIR doesn't affect ImageIR.

5. **Cross-domain conversions**: Extracting an image from a PDF goes through the orchestrator (PDF → DocumentIR → extract image → ImageIR → PNG), not through a single IR.

## Consequences

- Clean, maintainable data models per domain
- Cross-domain conversions require orchestrator coordination
- Each IR must be complete enough to preserve meaningful information
- Lossy/lossless boundaries are explicit per field
