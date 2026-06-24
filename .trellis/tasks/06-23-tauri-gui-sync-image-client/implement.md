# Implementation Plan

## Execution Checklist

- [x] Add a library surface to `sync_image_client` and keep the CLI as a thin
      wrapper over shared services.
- [x] Refactor SSH trust and private-key auth flows to return structured
      interaction requests instead of reading stdin directly.
- [x] Add config save/serialize helpers in `sync_image_core` so GUI and CLI use
      the same config write path.
- [x] Refactor the hotkey runtime into a start/stop-capable controller suitable
      for Tauri ownership.
- [x] Scaffold the Tauri app with React + TypeScript + Vite and add the
      `src-tauri` crate to the workspace.
- [x] Implement Tauri backend commands for config load/save, check, runtime
      start/stop, status, and private-key path selection.
- [x] Build the minimal control-panel UI and wire modal flows for host trust and
      passphrase entry.
- [x] Run formatting, Rust tests/checks, frontend build checks, and add GitHub
      Actions Windows GUI build verification.

## Ordered Work

### 1. Extract reusable Rust surfaces

Target files:

- `crates/sync_image_client/src/main.rs`
- new `crates/sync_image_client/src/lib.rs`
- supporting modules under `crates/sync_image_client/src/`

Work:

- move CLI-owned orchestration into library functions
- keep `main.rs` focused on argument parsing and terminal adapter behavior
- avoid changing upload semantics during extraction

Rollback point:

- if the refactor becomes unstable, stop after lib extraction and verify the CLI
  still behaves identically before introducing Tauri code

### 2. Replace terminal-bound interaction with structured outcomes

Target files:

- `crates/sync_image_client/src/upload.rs`
- new interaction/result types under `crates/sync_image_client/src/`

Work:

- remove direct stdin/stdout prompting from uploader/auth flow
- introduce typed outcomes for trust confirmation and passphrase requests
- add a CLI adapter loop that preserves the current terminal UX on top of the
  new interaction contract

Rollback point:

- CLI `check` and `run` must still work before any GUI integration begins

### 3. Add config write support

Target files:

- `crates/sync_image_core/src/config.rs`

Work:

- add serialization/write helpers
- preserve existing default config path behavior
- keep validation centralized in Rust

### 4. Introduce runtime ownership primitives

Target files:

- `crates/sync_image_client/src/hotkey_runtime.rs`
- new runtime manager/controller module(s)

Work:

- change runtime startup from a forever-blocking helper into a controllable
  runtime handle
- implement Windows stop via a clean message-loop exit path
- define a small backend-facing runtime status model

Risk check:

- this is the highest-risk Rust change because it alters the lifetime model of
  the running client

### 5. Scaffold the Tauri desktop app

Target paths:

- `apps/sync_image_desktop/`
- `apps/sync_image_desktop/src-tauri/`
- workspace `Cargo.toml`

Work:

- create the Tauri v2 app
- use React + TypeScript + Vite for the frontend
- add the Tauri backend crate to the workspace
- wire backend dependencies on `sync_image_core` and `sync_image_client`

### 6. Implement backend commands and state

Target files:

- Tauri backend `src-tauri/src/*.rs`

Work:

- command handlers for config load/save
- command handlers for check/start/stop/status
- runtime manager stored in Tauri app state
- structured interaction request/reply handling
- file-picker command for private-key path

### 7. Implement the MVP UI

Target files:

- frontend `src/*.tsx`
- frontend styles/components

Work:

- build a single-page control panel
- add form binding for the seven config fields
- add save/check/start/stop actions
- add modal flow for host trust confirmation
- add modal flow for passphrase entry without persistence
- show only minimal status, not logs/history

### 8. Validate and tighten

Work:

- run Rust formatting and tests
- run workspace compilation checks
- run frontend build checks
- manually verify start/stop and prompt flows on Windows
- confirm the CLI still works after refactors

## Validation Commands

Run the exact commands that exist after scaffolding is in place. Expected
minimum validation set:

```bash
cargo fmt --check
cargo test -p sync_image_core -p sync_image_client
cargo check --workspace
cd apps/sync_image_desktop && npm run build
cargo check --manifest-path apps/sync_image_desktop/src-tauri/Cargo.toml
```

If frontend package tooling differs after scaffolding, update the app-local
build command accordingly before `task.py start`.

## Verification Notes

- `cargo fmt --check` passed.
- `.github/workflows/release.yml` parses as YAML.
- `cargo metadata --locked --format-version 1` passed.
- `cargo check -p sync_image_core -p sync_image_client -p sync_image_mcp`
  passed.
- `cargo clippy -p sync_image_core -p sync_image_client -p sync_image_mcp
  --all-targets -- -D warnings` passed.
- `cargo test -p sync_image_core -p sync_image_client -p sync_image_mcp`
  passed.
- `npm run build` passed in `apps/sync_image_desktop`.
- `cargo check --manifest-path apps/sync_image_desktop/src-tauri/Cargo.toml`
  is blocked locally by missing Linux WebKitGTK/pkg-config system libraries
  (`gdk-3.0`; previous local probes also reported related GTK/WebKitGTK
  libraries such as `pango`, `atk`, `cairo`, `gdk-pixbuf-2.0`,
  `javascriptcoregtk-4.1`, and `libsoup-3.0`).
- Windows desktop build verification is handled by the
  `build-windows-desktop` GitHub Actions job on `windows-latest`; its bundle
  collection step searches both workspace and app-local Tauri target roots and
  fails when no bundle files are found.

## Review Gates Before `task.py start`

- `prd.md` still matches the chosen MVP scope
- `design.md` clearly preserves Rust as the source of truth
- runtime stop approach is accepted
- structured interaction contract is accepted
- Tauri app location in the repo is accepted

## Risky Files

- `crates/sync_image_client/src/upload.rs`
- `crates/sync_image_client/src/hotkey_runtime.rs`
- `crates/sync_image_client/src/main.rs`
- `crates/sync_image_core/src/config.rs`
- root `Cargo.toml`

## Notes

- Inline Trellis mode is active, so JSONL curation is intentionally skipped for
  this planning task.
- Do not start implementation until these planning artifacts are reviewed and
  the task is explicitly started.
