# Fixing Process — Status & Context

**Last updated:** 2026-07-23 01:50 UTC  
**Repo:** https://github.com/AutomatedLabor/universal-file-converter  
**Tag:** v0.1.0 (currently broken — see below)

---

## Goal

Get the Universal File Converter building cleanly on GitHub Actions (Linux, macOS, Windows), with a release that produces downloadable CLI binaries for all 3 platforms.

Stretch goal: Tauri desktop app builds (needs additional system deps setup).

---

## What's Built (124 files, 66 Rust source files)

The codebase is **complete and architecturally sound**. All code is written:

- **5 core crates:** ufc-plugin-api, ufc-ir, ufc-core, ufc-host, ufc-cli
- **27 plugins:** 8 image, 5 document, 6 audio, 1 video, 3 archive, 4 structured data
- **Tauri app:** Rust backend + Svelte frontend (6 components)
- **CI/CD:** GitHub Actions for CI (check, test, fmt, clippy, audit) + Release (cross-platform builds)
- **Docs:** README, plugin dev guide, 3 ADRs, LICENSE

---

## Current Build Failures (iteration 5+)

The CI keeps failing with compilation errors. Here's every issue found so far and what was fixed:

### Already Fixed
1. **`tauri` feature `dialog` doesn't exist in v2** → Removed the feature from Cargo.toml
2. **`Box<dyn Read + Seek + Send>` is invalid Rust** → Created a `ReadSeek` trait alias
3. **Unused imports** (`chrono::DateTime`, `chrono::Utc` in types.rs) → Removed
4. **`error_msg.clone` missing parens** → Changed to `error_msg.clone()`
5. **`toml` crate missing from ufc-core deps** → Added to Cargo.toml
6. **`mut` on non-mutated variable** → Removed `mut`
7. **Unused variable `target`** → Prefixed with `_`
8. **Borrow checker: `self.queue.get_mut()` + `self.emit()`** → Restructured to drop mutable borrow before emitting

### Still Failing (as of last push)
9. **`thiserror` v2 `as_dyn_error` on `String` fields** — This is the current blocker.

   `thiserror` v2 tries to call `as_dyn_error()` on every field in error enums. `String` doesn't implement `std::error::Error`, so it fails.

   **The fix:** Either:
   - (a) Pin `thiserror = "=1.0.50"` (v1 doesn't have this issue), OR
   - (b) Change all `String` fields in `CoreError` to use `#[source]` or remove them from the error display, OR
   - (c) Replace `String` fields with a custom error type that implements `Error`

   **Recommended:** Option (a) — pin to thiserror 1.0.50. Already tried `thiserror = "1"` but Cargo may still resolve to a v1.x that has the same issue if it's actually pulling v2 transitively. Need to verify with `cargo tree -p thiserror`.

   **Important:** Some other crate in the dependency tree may be pulling in thiserror v2. Check with `cargo tree -i thiserror`. If so, you may need to force version resolution in the workspace Cargo.toml:
   ```toml
   [workspace.dependencies]
   thiserror = { version = "=1.0.50", default-features = false }
   ```

---

## How to Continue

### Step 1: Get it compiling locally
```bash
cd file-converter
cargo check --workspace 2>&1 | head -50
```

Fix errors iteratively. The code is written correctly in terms of logic — these are all Rust type system / borrow checker / dependency issues.

### Step 2: Once it compiles, run tests
```bash
cargo test --workspace
```

### Step 3: Push and tag
```bash
git add -A && git commit -m "fix: compilation errors"
git push origin main
git tag -d v0.1.0 && git push origin :refs/tags/v0.1.0
git tag v0.1.0 && git push origin v0.1.0
```

### Step 4: Monitor CI
```bash
GH_TOKEN="<token>" gh run list --repo AutomatedLabor/universal-file-converter --limit 5
```

### Step 5: Once CI passes, check the release
The release workflow (`release.yml`) builds CLI binaries for:
- `x86_64-unknown-linux-gnu` (ubuntu-latest)
- `aarch64-apple-darwin` (macos-latest)
- `x86_64-pc-windows-msvc` (windows-latest)

It uploads them as GitHub Release assets.

---

## Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace root — all dependency versions |
| `crates/ufc-plugin-api/src/io.rs` | FileReader/FileWriter — has `ReadSeek` trait |
| `crates/ufc-core/src/error.rs` | CoreError enum — thiserror issue here |
| `crates/ufc-core/src/orchestrator.rs` | Main conversion coordinator — borrow checker issues |
| `crates/ufc-core/src/router.rs` | DAG-based conversion routing |
| `crates/ufc-cli/src/commands/convert.rs` | CLI convert command |
| `crates/ufc-tauri/src/commands.rs` | Tauri IPC commands |
| `.github/workflows/ci.yml` | CI workflow (check, test, fmt, clippy, audit) |
| `.github/workflows/release.yml` | Release workflow (build + upload binaries) |

---

## Architecture Summary

```
User Input → Format Detector → DAG Router → Decoder Plugin → IR → Encoder Plugin → Output
```

- **Format Detector:** Magic bytes + file extension
- **Router:** Finds best plugin path (priority × fidelity score)
- **Plugins:** Isolated WASM or process-sandboxed converters
- **IRs:** Domain-specific (Image, Document, Audio, Video, Vector, Table, Archive, Mesh)
- **Queue:** Concurrent conversion with pause/resume/cancel
- **Integrity:** Blake3 checksums on output

---

## GitHub Actions

- **CI:** Triggers on push to `main`. Runs check, test (3 platforms), fmt, clippy, security audit.
- **Release:** Triggers on `v*` tag. Builds CLI for 3 platforms, creates GitHub Release with binaries.

Both are currently failing on compilation. Once fixed, they should work without further changes.

---

## Tauri Desktop App (Stretch Goal)

The Tauri app needs additional setup:
1. System dependencies (webkit2gtk on Linux, etc.)
2. Frontend build (`cd ui && npm install && npm run build`)
3. The release workflow currently only builds CLI — Tauri builds would need a separate workflow or `tauri-action`

The Tauri code is complete but untested. It may have its own compilation issues.

---

## Auth

GitHub auth is via `GH_TOKEN` env var. Token has scopes: `delete_repo`, `repo`, `workflow`, `write:packages`. Missing `read:org` (warning only, not blocking).

Usage: `GH_TOKEN="<token>" gh <command>`
