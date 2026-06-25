# Claude Image Sync MCP

Rust workspace for a Windows-to-Linux image handoff workflow:

- `sync-image-client`: Windows foreground CLI. It registers `Ctrl+Alt+U`, reads the current clipboard image or copied PNG/JPEG file, normalizes it to PNG, and uploads it to shared NAS storage through SSH/SFTP.
- `sync-image-mcp`: Linux stdio MCP server. It reads the shared NAS mount, selects the newest timestamped PNG for `CLAUDE_IMAGE_USER`, and returns it as MCP image content.

## Storage Contract

Uploads are stored as:

```text
<shared_image_root>/<user_name>/<YYYYMMDD_HHMMSS_mmm>.png
```

The initial shared root is:

```text
/mnt/xy_internel/share/claude
```

Filenames use UTC upload completion time and sort lexicographically by recency.

## Windows Client

Default config path:

```text
%APPDATA%\claude-image-sync\config.toml
```

Override with:

```powershell
sync-image-client --config C:\path\config.toml run
```

Example config:

```toml
user_name = "alice"
hotkey = "Ctrl+Alt+U"

[upload]
host = "upload-host.example.internal"
port = 22
user = "claude-upload"
auth_method = "key"
private_key_path = "C:\\Users\\Alice\\.ssh\\claude_upload_ed25519"
shared_image_root = "/mnt/xy_internel/share/claude"
```

### SSH Authentication

`upload.auth_method` selects how the client authenticates:

- `"key"` (default, backward compatible): public-key auth using `private_key_path`. If the key is passphrase-protected, the passphrase is requested on demand and never persisted.
- `"password"`: password auth. Provide `upload.password` directly, or leave it unset to be prompted on the first connection. On the first successful connection the entered password is written back to `config.toml`.

```toml
[upload]
host = "upload-host.example.internal"
port = 22
user = "claude-upload"
auth_method = "password"
# password = "your-ssh-password"   # optional; prompted and saved on first connect
shared_image_root = "/mnt/xy_internel/share/claude"
```

> ⚠️ Password auth stores the SSH password in plain text in `config.toml`
> (`%APPDATA%\claude-image-sync\config.toml`). Prefer key auth where possible.

Run a deployment check:

```powershell
sync-image-client --config C:\path\config.toml check
```

`check` validates config, SSH/SFTP authentication, TOFU host key behavior, and remote write access by creating and deleting `.claude-image-sync-check.tmp`.

Run the foreground hotkey client:

```powershell
sync-image-client --config C:\path\config.toml run
```

While it is running, press `Ctrl+Alt+U` to upload the current clipboard image or a copied PNG/JPEG file. Success and failure are reported through terminal logs and Windows toast notifications.

## Windows Release Builds

GitHub Actions builds the Windows CLI client and the Windows Tauri desktop app on `windows-latest` when a tag matching `v*` is pushed.

The CLI release asset is:

```text
sync-image-client-windows-x86_64.zip
```

The zip contains:

- `sync-image-client.exe`
- `config.toml`

The desktop GUI artifact is:

```text
sync-image-desktop-windows-x86_64.zip
```

The desktop bundle is produced by the `build-windows-desktop` GitHub Actions job. Windows GUI build verification should be performed through GitHub Actions because local Linux builds require WebKitGTK system libraries and do not exercise the Windows hotkey path.

Create a release build by pushing a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow can also be run manually from the GitHub Actions page for build verification without creating a release.

## Windows Desktop GUI

The Tauri desktop app lives in:

```text
apps/sync_image_desktop
```

It is a minimal control panel over the existing Windows client flow:

- edit and save the default client config
- run the same SSH/SFTP connectivity check as the CLI
- start and stop the hotkey upload runtime
- confirm first-time SSH host keys in-app
- enter private-key passphrases on demand without persisting them

The existing CLI remains supported and is still the underlying source of truth for upload behavior.

### Desktop Upload Service

The desktop app is a control panel for the Windows upload runtime. It uses the same config format as the CLI and reads the default file from:

```text
%APPDATA%\claude-image-sync\config.toml
```

Run it from the desktop bundle, edit the upload fields, then save the config and start the runtime. The key upload settings are:

```toml
[upload]
host = "upload-host.example.internal"
port = 22
user = "claude-upload"
private_key_path = "C:\\Users\\Alice\\.ssh\\claude_upload_ed25519"
shared_image_root = "/mnt/xy_internel/share/claude"
```

The desktop app exposes the same connectivity check as the CLI. Use it before enabling the hotkey runtime to confirm SSH/SFTP access and remote write permission.

## MCP Server

Build the container:

```bash
docker build -f docker/Dockerfile.mcp -t claude-image-sync-mcp:latest .
```

The Dockerfile builds the Rust MCP binary in a Rust builder stage and copies the `sync-image-mcp` executable into a small runtime image. Build from the repository root so the workspace `Cargo.toml` and `Cargo.lock` are available.

Example stdio MCP command:

```bash
docker run -i --rm \
  -e CLAUDE_IMAGE_ROOT=/data/claude-images \
  -e CLAUDE_IMAGE_USER="$USER" \
  -v /mnt/xy_internel/share/claude:/data/claude-images:ro \
  claude-image-sync-mcp:latest
```

The MCP tool is named `get_latest_screenshot`. It returns:

- selected PNG image content
- selected filename
- upload time parsed from the filename

### Add to Claude MCP

Register the server in Claude Desktop's MCP config as a stdio server that runs the container above. Replace `alice` with your actual Claude image user:

```json
{
  "mcpServers": {
    "sync-image-mcp": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e",
        "CLAUDE_IMAGE_ROOT=/data/claude-images",
        "-e",
        "CLAUDE_IMAGE_USER=alice",
        "-v",
        "/mnt/xy_internel/share/claude:/data/claude-images:ro",
        "claude-image-sync-mcp:latest"
      ]
    }
  }
}
```

If your Claude Desktop config uses a different path or platform-specific format, keep the same container command and mount, and adjust only the config file location around it. Restart Claude after adding the server.

## MVP Limits

Out of scope for MVP:

- automatic upload on clipboard changes
- installer, tray app, service mode, or startup registration
- strong per-user storage isolation
- retention, quota, cleanup, or pruning
- listing or selecting older images
- WebP, animated GIF, PDF, or multi-file drag/drop input
- system `scp` or OpenSSH ControlMaster
