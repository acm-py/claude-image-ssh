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
private_key_path = "C:\\Users\\Alice\\.ssh\\claude_upload_ed25519"
shared_image_root = "/mnt/xy_internel/share/claude"
```

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

GitHub Actions builds the Windows client on `windows-latest` when a tag matching `v*` is pushed. The release asset is:

```text
sync-image-client-windows-x86_64.zip
```

The zip contains:

- `sync-image-client.exe`
- `config.toml`

Create a release build by pushing a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow can also be run manually from the GitHub Actions page for build verification without creating a release.

## MCP Server

Build the container:

```bash
docker build -f docker/Dockerfile.mcp -t claude-image-sync-mcp:latest .
```

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

## MVP Limits

Out of scope for MVP:

- automatic upload on clipboard changes
- installer, tray app, service mode, or startup registration
- strong per-user storage isolation
- retention, quota, cleanup, or pruning
- listing or selecting older images
- WebP, animated GIF, PDF, or multi-file drag/drop input
- password SSH login
- system `scp` or OpenSSH ControlMaster
