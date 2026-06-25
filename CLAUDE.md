# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Purpose

A Windows-to-Linux image handoff workflow. A Windows client uploads clipboard/copied images to shared NAS storage over SSH/SFTP; a Linux stdio MCP server reads that storage and serves the newest image back to Claude. This lets a user screenshot on Windows and ask Claude (running over SSH on Linux) to "look at the image I just copied."

## Workspace Layout

Cargo workspace (`edition = "2024"`, all versions/deps pinned in root `Cargo.toml` `[workspace.dependencies]`):

- `crates/sync_image_core` — shared, platform-agnostic logic: config (`ClientConfig`/`UploadConfig`, TOML), hotkey parsing, the storage-contract filename logic (`format_upload_filename`/`parse_upload_filename`/`find_latest_image`), and user-dir path building/validation. Both binaries depend on this.
- `crates/sync_image_client` — Windows CLI binary `sync-image-client`. Windows-only deps (`arboard`, `windows-sys`, `winrt-notification`) are gated under `[target.'cfg(windows)'.dependencies]`.
- `crates/sync_image_mcp` — Linux stdio MCP server binary `sync-image-mcp`.
- `apps/sync_image_desktop` — Tauri 2 + React/Vite desktop GUI wrapping the client flow. Its `src-tauri` crate is a workspace member and depends on `sync_image_client` as a library.

## Build, Test, Run

```bash
# Build everything / a single crate
cargo build
cargo build -p sync_image_mcp

# Test everything / a single crate / a single test
cargo test
cargo test -p sync_image_core
cargo test -p sync_image_core finds_latest_image_by_filename

# Build the MCP container (run from repo root — needs workspace Cargo.toml + Cargo.lock)
docker build -f docker/Dockerfile.mcp -t claude-image-sync-mcp:latest .

# Desktop GUI (run inside apps/sync_image_desktop)
npm ci
npm run tauri -- build --target x86_64-pc-windows-msvc   # full bundle
npm run dev                                              # frontend dev server only
```

Tests live inline as `#[cfg(test)] mod tests` in each module (config, hotkey, image_files, paths, image_input, latest, protocol). There is no separate `tests/` dir.

## Critical Cross-Cutting Contract

The storage path/filename format is the integration point between client and server and **must stay identical on both sides** (it lives in `sync_image_core::image_files`):

```
<shared_image_root>/<user_name>/<YYYYMMDD_HHMMSS_mmm>.png
```

- Format string: `%Y%m%d_%H%M%S_%3f`, always `.png`, timestamp is **UTC upload-completion** time.
- "Latest" is chosen by lexicographic filename comparison (NOT mtime) — the format is designed so string order equals time order. Files that don't parse to this exact pattern are ignored.
- Changing this format breaks the client→server handoff. Update `format_upload_filename`, `parse_upload_filename`, and any tests together.

## Architecture Notes

**Client action model (shared by CLI `run`/`check` and the desktop GUI).** SSH connection logic is uniform across all three entry points via `ClientActionState<T>` (`crates/sync_image_client/src/client.rs`): an operation returns either `Ready(T)` or `NeedsInteraction(InteractionRequest)`. Interaction requests are TOFU host-key trust prompts and private-key passphrase prompts. This is what lets the same `connect_uploader`/`run_check` code drive a terminal prompt (CLI) or a Tauri dialog round-trip (desktop) — the caller resolves the interaction and re-invokes with an `InteractionResponse`. Passphrases are never persisted.

**MCP server** (`crates/sync_image_mcp/src/protocol.rs`) is a hand-rolled JSON-RPC 2.0 stdio loop (no MCP SDK). It handles `initialize`, `tools/list`, `tools/call`, swallows `notifications/initialized`. The single tool `get_latest_screenshot` returns a text block (`selected_file` + `upload_time`) plus a base64 `image/png` block. Config comes from env: `CLAUDE_IMAGE_ROOT` and `CLAUDE_IMAGE_USER` (see `config_from_env`).

**Desktop GUI** exposes Tauri commands `load_config`, `save_config`, `check_connection`, `start_runtime`, `stop_runtime`, `runtime_status` (`apps/sync_image_desktop/src-tauri/src/lib.rs`). It reuses `sync_image_client`'s `RuntimeManager` and the `ClientActionState` interaction protocol via DTO enums. The CLI remains the source of truth for upload behavior.

**SSH** uses `ssh2` with `vendored-openssl` (OpenSSL is compiled in, not a system dep). Upload does one automatic reconnect-and-retry on failure (`SftpUploader::upload_png`).

## Platform Constraints

- The client's clipboard/hotkey/notification paths are Windows-only. On Linux those code paths are `cfg`-gated and marked `allow(dead_code)`; building the client on Linux compiles but does not exercise the real hotkey/clipboard logic.
- The Tauri Windows build cannot be fully verified on Linux (needs WebKitGTK). **Verify Windows GUI/client builds through GitHub Actions**, not local Linux builds.

## Release CI

`.github/workflows/release.yml` runs on `v*` tags (or manual dispatch) on `windows-latest`, target `x86_64-pc-windows-msvc`:
- `build-windows-client` → `sync-image-client-windows-x86_64.zip` (exe + `examples/config.toml`).
- `build-windows-desktop` → runs `npm run apply-build-config` before `tauri build`, produces `sync-image-desktop-windows-x86_64.zip`.

Cut a release with `git tag v0.1.0 && git push origin v0.1.0`.

## Conventions

- Errors use `anyhow` with `.context(...)`; the desktop layer converts to `String` at the Tauri command boundary.
- Config validation goes through `ClientConfig::validate()` on every load/save/parse path.
- Out of MVP scope (do not add unless asked): auto-upload on clipboard change, tray/service/installer, retention/cleanup, older-image listing, WebP/GIF/PDF input, system `scp`/ControlMaster.

## Trellis

This project is managed by Trellis (`AGENTS.md` + `.trellis/`). Active/archived tasks, PRDs, and per-layer coding specs live under `.trellis/` — consult `.trellis/workflow.md` and `.trellis/spec/` before larger changes, and prefer `/trellis:*` commands when available.
