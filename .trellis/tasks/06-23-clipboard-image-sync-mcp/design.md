# Clipboard Image Sync MCP Design

## Summary

The MVP has two Rust binaries:

- `sync-image-client`: a Windows foreground CLI that registers a global hotkey, reads the current clipboard image or copied PNG/JPEG file, normalizes it to PNG, and uploads it over Rust-native SSH/SFTP to a dedicated upload host.
- `sync-image-mcp`: a Linux stdio MCP server packaged as a Docker image. It reads a shared image directory mounted from NAS, selects the newest timestamped PNG for `CLAUDE_IMAGE_USER`, and returns it as MCP image content.

The two binaries are intentionally coupled only through the shared storage contract:

```text
<shared_image_root>/<user_name>/<YYYYMMDD_HHMMSS_mmm>.png
```

The initial host shared root is `/mnt/xy_internel/share/claude`. The MCP container mounts that root read-only at `/data/claude-images` and reads it through `CLAUDE_IMAGE_ROOT=/data/claude-images`.

## Architecture

### Windows Client

Responsibilities:

- Load TOML configuration from `%APPDATA%\claude-image-sync\config.toml`, with `--config <path>` override.
- Provide `run` behavior as the default foreground mode.
- Provide `check` to validate configuration, SSH/SFTP authentication, host key verification, user directory creation, and write access.
- Register a configurable Windows global hotkey, default `Ctrl+Alt+U`.
- Establish SSH/SFTP connection at startup, including TOFU host key handling and private key/passphrase authentication.
- Reconnect on the next upload trigger if the connection drops.
- On hotkey trigger:
  - Read static clipboard image data, or a copied PNG/JPEG file path.
  - Reject unsupported input such as animated GIF, WebP, PDF, or multi-file drag/drop.
  - Normalize accepted input to PNG without resizing, compression policy changes, or application-level size cap.
  - Upload the PNG to `<shared_image_root>/<user_name>/`.
  - Show terminal logs and Windows toast notification for success/failure.

Recommended crate boundaries:

- `config`: TOML parsing, validation, default paths.
- `clipboard`: Windows clipboard image and copied file detection.
- `image_normalize`: PNG/JPEG decoding and PNG encoding.
- `upload`: SSH/SFTP session, TOFU store, directory creation, temp upload, rename.
- `hotkey`: Windows global hotkey registration and message loop.
- `notify`: terminal log plus Windows toast adapter.

### Upload Contract

The client writes a new PNG per successful trigger. It does not deduplicate repeated uploads.

Final filenames must be sortable by upload completion time:

```text
YYYYMMDD_HHMMSS_mmm.png
```

Use UTC for filenames. To avoid the MCP server reading a partially written latest file, upload to a temporary hidden name first, then rename to the final timestamped filename after SFTP write succeeds:

```text
.<uuid>.uploading
YYYYMMDD_HHMMSS_mmm.png
```

This is an implementation consistency detail, not a separate metadata mechanism.

The `check` command may create and delete:

```text
<shared_image_root>/<user_name>/.claude-image-sync-check.tmp
```

### MCP Server

Responsibilities:

- Run as a stdio MCP server on Linux.
- Be packaged as a Docker image.
- Read `CLAUDE_IMAGE_ROOT` and `CLAUDE_IMAGE_USER`.
- Scan `<CLAUDE_IMAGE_ROOT>/<CLAUDE_IMAGE_USER>/`.
- Filter to filenames matching the timestamped PNG pattern.
- Select the lexicographically greatest filename.
- Return image content as `image/png`.
- Include selected filename and parsed upload time in the tool response text.
- Return a clear error when:
  - Required environment variables are missing.
  - The user directory does not exist.
  - No timestamped PNG exists.
  - The selected PNG cannot be read.

The first target runtime is Claude Code / Claude CLI stdio MCP configuration using Docker:

```bash
docker run -i --rm \
  -e CLAUDE_IMAGE_ROOT=/data/claude-images \
  -e CLAUDE_IMAGE_USER="$USER" \
  -v /mnt/xy_internel/share/claude:/data/claude-images:ro \
  claude-image-sync-mcp:latest
```

### Shared Storage

The upload host writes to the NAS-mounted root. Remote Claude machines mount the same storage. MVP assumes this infrastructure exists and is reliable.

MVP does not handle:

- Retention, cleanup, pruning, or quota.
- Strong per-user isolation.
- Listing or selecting older images via MCP.

## Configuration

Example Windows client TOML:

```toml
user_name = "alice"
hotkey = "Ctrl+Alt+U"

[upload]
host = "upload-host.example.internal"
port = 22
user = "claude-upload"
private_key_path = "C:\\Users\\Alice\\.ssh\\claude_upload_ed25519"
shared_image_root = "/mnt/xy_internel/share/claude"
```

Host key TOFU cache should live under `%APPDATA%\claude-image-sync\`, separate from the main config. The private key passphrase is prompted interactively and never persisted.

## Security And Trust

MVP assumes an internal trusted-user environment:

- A shared upload SSH account/key is acceptable.
- `user_name` is a routing convention, not a security boundary.
- The MCP container mounts NAS read-only.
- Password SSH login is out of scope.
- System `scp` and OpenSSH ControlMaster are out of scope.

TOFU mitigates accidental upload-host substitution after first trust, but it is not a centralized host identity policy.

## Technical Choices

Implemented Rust dependencies:

- SSH/SFTP: `ssh2` with vendored OpenSSL, avoiding system `scp` and host OpenSSL headers.
- Image decoding/encoding: `image`.
- Clipboard image support: `arboard` for static clipboard images.
- Copied file path support on Windows: `windows-sys` for `CF_HDROP`.
- Global hotkey: Windows `RegisterHotKey` via `windows-sys`.
- Toast notification: `winrt-notification`.
- MCP server: minimal line-delimited JSON-RPC adapter with MCP-shaped `initialize`, `tools/list`, and `tools/call` responses. Storage selection logic remains isolated so a dedicated MCP SDK can replace the adapter later if needed.

The implementation should isolate third-party APIs behind small modules so crate substitutions remain local.

## Compatibility And Deployment

Windows client:

- Distributed as one `.exe` plus example `config.toml`.
- No installer, tray app, service mode, or startup registration in MVP.

MCP server:

- Distributed as a Docker image.
- Validated with `docker run -i --rm`.
- Podman-specific validation is out of scope.

## Risks

- Windows clipboard copied-file formats vary by source application. The client should clearly report unsupported clipboard states.
- Rust SSH/SFTP crate behavior around encrypted private keys must be verified early.
- Windows toast support can vary by terminal/runtime environment. Terminal logs remain the reliable fallback.
- MCP image return shape depends on the selected Rust MCP SDK. Keep storage and selection logic independent from SDK-specific response builders.
