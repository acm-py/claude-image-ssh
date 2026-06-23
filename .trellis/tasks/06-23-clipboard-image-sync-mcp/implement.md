# Clipboard Image Sync MCP Implementation Plan

## Scope

Implement the MVP described in `prd.md` and `design.md` as a single Rust workspace with two binaries:

- Windows upload client.
- Linux stdio MCP server packaged in Docker.

Task has been activated and implementation has started.

## Work Plan

- [x] Scaffold Rust workspace.
  - Add root `Cargo.toml`.
  - Add crates for shared logic, Windows client, and MCP server as needed.
  - Add formatting/lint/test commands to project docs.

- [x] Implement shared storage/path contracts.
  - Timestamped UTC filename generation: `YYYYMMDD_HHMMSS_mmm.png`.
  - Filename parser and sorter for MCP latest-selection logic.
  - Path builder for `<shared_image_root>/<user_name>/`.
  - Tests for filename sorting, invalid filename filtering, and user path construction.

- [x] Implement Windows client configuration.
  - Default config path: `%APPDATA%\claude-image-sync\config.toml`.
  - `--config <path>` override.
  - Example `config.toml`.
  - Validation for required fields, default hotkey, and path fields.

- [x] Implement SSH/SFTP upload layer.
  - Rust-native SSH/SFTP client integration.
  - Private key authentication.
  - Interactive passphrase prompt without persistence.
  - TOFU host key cache under `%APPDATA%\claude-image-sync\`.
  - Startup connection and authentication.
  - Reconnect on next upload trigger after disconnect.
  - Remote directory creation.
  - Temp upload then rename to final timestamped PNG.

- [x] Implement `check` command.
  - Load and validate config.
  - Connect and authenticate.
  - Verify host key behavior.
  - Ensure remote user directory exists or can be created.
  - Create and delete `.claude-image-sync-check.tmp`.
  - Do not read clipboard or upload real screenshots.

- [x] Implement clipboard and image normalization.
  - Read static clipboard image.
  - Read copied PNG/JPEG file path.
  - Reject unsupported inputs: animated GIF, WebP, PDF, multi-file drag/drop.
  - Normalize accepted input to PNG.
  - Preserve original resolution.
  - Do not impose application-level size cap.

- [x] Implement Windows hotkey runtime.
  - Foreground CLI process.
  - Register default `Ctrl+Alt+U`, with config override.
  - Report hotkey registration failure clearly.
  - On hotkey trigger, read current clipboard input and upload a new timestamped PNG.
  - Log and toast success/failure.

- [x] Implement MCP server.
  - Read `CLAUDE_IMAGE_ROOT` and `CLAUDE_IMAGE_USER`.
  - Scan `<CLAUDE_IMAGE_ROOT>/<CLAUDE_IMAGE_USER>/`.
  - Select newest timestamped PNG by filename.
  - Return image content plus selected filename and parsed upload time.
  - Return clear errors for missing env vars, missing directory, no images, and read failures.

- [x] Add Docker packaging for MCP server.
  - Multi-stage Dockerfile if appropriate.
  - Runtime image runs stdio MCP server.
  - Document `docker run -i --rm` MCP configuration.
  - Mount `/mnt/xy_internel/share/claude:/data/claude-images:ro`.
  - Set `CLAUDE_IMAGE_ROOT=/data/claude-images` and `CLAUDE_IMAGE_USER=$USER`.

- [x] Write setup and usage docs.
  - Windows `.exe` plus example `config.toml`.
  - Upload host and NAS assumptions.
  - Shared upload SSH account/key model.
  - `check` command.
  - Hotkey workflow.
  - MCP Docker configuration.
  - Known MVP exclusions.

## Validation Plan

- [x] Run Rust formatting.
- [x] Run Rust linting if configured.
- [x] Run unit tests for:
  - Timestamp filename generation/parsing/sorting.
  - Config loading and validation.
  - Path construction.
  - MCP latest-image selection.
  - Unsupported input classification where testable without a real Windows clipboard.
- [ ] Run `sync-image-client check` against a test upload host or documented local test double if the real host is unavailable. Not run: no upload host/key in this environment.
- [ ] Validate Windows hotkey behavior on a Windows machine. Not run: current environment is Linux.
- [ ] Validate upload of: Not run: current environment is Linux and has no Windows clipboard or upload host.
  - Clipboard screenshot/static image.
  - Copied PNG file.
  - Copied JPEG file.
- [x] Validate rejection of unsupported input with clear messages.
- [ ] Build MCP Docker image. Not run: Docker is not installed in this environment.
- [ ] Run MCP server through Docker with a mounted fixture directory. Not run: Docker is not installed in this environment.
- [x] Verify MCP returns:
  - Latest PNG by filename.
  - Selected filename.
  - Parsed upload time.
  - Clear error when no image exists.

## Rollback Points

- After workspace scaffold: can remove new Rust workspace files.
- After upload layer: keep path/selection tests while replacing SSH/SFTP crate if needed.
- After MCP JSON-RPC adapter integration: storage selection logic should remain independent and reusable if a dedicated MCP SDK replaces the adapter later.
- Before task completion: if Windows toast proves unreliable, keep terminal logs and mark toast issue explicitly for follow-up rather than blocking core upload/MCP behavior unless acceptance is changed.

## Current Status

- Core implementation is complete.
- Local Linux validation passed for formatting, linting, unit tests, CLI help, and MCP stdio smoke test.
- Remaining validation requires Windows, Docker, and a real upload host/NAS environment.
