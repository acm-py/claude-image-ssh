# Clipboard Image Sync MCP

## Goal

Build a local-to-remote screenshot/image handoff workflow so a user can copy or screenshot an image on their local machine, manually trigger upload, and have a remote Claude-compatible MCP client retrieve the latest image.

## Requirements

- Provide a local clipboard upload client that can read the current Windows clipboard image or copied PNG/JPEG file when the user explicitly triggers an upload.
- Target the MVP at Windows as the local desktop environment and Linux as the remote host environment.
- Run the Windows MVP daemon as a foreground CLI process launched from a terminal.
- Deliver the Windows client MVP as a single `.exe` plus an example `config.toml`; users can place and run it from any directory.
- While the Windows client is running, register a global hotkey so uploads can be triggered without focusing the terminal.
- Use `Ctrl+Alt+U` as the default upload hotkey and allow users to override it in configuration.
- Read Windows client configuration from `%APPDATA%\claude-image-sync\config.toml` by default, with `--config <path>` available for overrides.
- Provide a Windows client `check` command that validates configuration, performs SSH/SFTP authentication, verifies host key behavior, and confirms the configured remote user directory can be created/written without reading the clipboard or uploading a real image.
- The `check` command may create `.claude-image-sync-check.tmp` in the configured user directory and delete it immediately to verify remote write access.
- Upload only when the user triggers the foreground client with the configured Windows global hotkey; do not automatically upload on every clipboard change in MVP.
- Each successful hotkey trigger uploads a new PNG file; MVP does not deduplicate repeated uploads.
- Report upload results through both terminal logs and Windows toast notifications; success should include the uploaded filename, and failure should include a short actionable error.
- Implement remote upload through a Rust-native SSH/SFTP client library rather than shelling out to system `scp`.
- Establish the SSH/SFTP connection at Windows client startup, including host key verification and private key/passphrase authentication, so hotkey-triggered uploads can reuse an authenticated connection.
- If the SSH/SFTP connection drops while the client is running, attempt reconnect on the next upload trigger and report reconnect/upload failures clearly.
- Authenticate SSH/SFTP uploads with a configured private key file.
- If the private key is passphrase-protected, prompt interactively for the passphrase at startup or first connection and do not persist it.
- Verify the upload host key with TOFU behavior: show the fingerprint on first connection, save it after user confirmation, and reject later connections if the fingerprint changes.
- Use a shared upload SSH account/key for MVP; `user_name` is a routing convention for per-user image placement, not a strong security boundary.
- Store uploads under a per-user directory on shared storage, e.g. `<shared_image_root>/<user_name>/`, with the initial shared root planned as `/mnt/xy_internel/share/claude`.
- Configure the Windows uploader with `user_name`; during upload, create `<shared_image_root>/<user_name>/` automatically if it does not exist.
- Resolve the remote MCP user's image directory from the `CLAUDE_IMAGE_USER` environment variable passed when the MCP container starts, typically set from the host shell as `CLAUDE_IMAGE_USER=$USER`.
- Assume the dedicated upload host writes to storage that is also mounted by the remote Claude machines, such as a shared NAS.
- For MVP, accept static clipboard image content and copied PNG/JPEG image files, normalize all accepted inputs to PNG, and upload each result as a timestamped PNG file.
- Use a filename format that sorts by upload completion time, e.g. `YYYYMMDD_HHMMSS_mmm.png` in UTC.
- Preserve image resolution in MVP and do not impose an application-level upload size limit; upload/storage failures should surface as clear errors.
- Do not handle storage retention, quota, cleanup, or pruning in MVP.
- Provide a remote MCP server tool, initially named `get_latest_screenshot`, that scans the current user's shared storage directory, selects the newest uploaded PNG by timestamped filename, and returns it as MCP image content with the selected filename and parsed upload time.
- Implement the remote MCP server as a Linux stdio MCP server, with Claude Code/Claude CLI as the first target client/runtime.
- Package the MCP server as a container image so deployment can be configured by the MCP client command plus mounted shared storage.
- Use Docker `docker run -i --rm ...` as the first supported container runtime example for stdio MCP client configuration.
- Run the containerized MCP server with the NAS/shared image root mounted read-only into the container at `/data/claude-images`.
- Configure the MCP container with `CLAUDE_IMAGE_ROOT=/data/claude-images` so the image root is not hard-coded to the host NAS path.
- Configure the MCP container with `CLAUDE_IMAGE_USER` so container user identity (`root` or another runtime user) does not affect image lookup.
- Keep the normal user workflow simple: take a screenshot or copy an image locally, press the configured global hotkey, then ask the remote assistant to inspect the latest image.
- Existing project evidence:
  - `arch.md` contains the initial architecture proposal.
  - The repository does not currently contain Rust source, Cargo manifests, README files, or an existing MCP implementation.
  - The proposed architecture has two deliverables: a local sync daemon and a remote MCP image server.

## Acceptance Criteria

- [ ] A local client can read the current clipboard image or copied PNG/JPEG file and upload it to the configured remote cache path when manually triggered.
- [ ] The Windows client can be distributed as a single `.exe` with an example `config.toml`, without an installer.
- [ ] The Windows client registers a global hotkey while running and uses it to trigger upload without terminal focus.
- [ ] The default hotkey is `Ctrl+Alt+U`, and it can be changed in configuration.
- [ ] The Windows client loads TOML configuration from `%APPDATA%\claude-image-sync\config.toml` by default and supports `--config <path>`.
- [ ] The Windows client provides a `check` command that validates configuration, SSH/SFTP authentication, host key verification, and remote directory write access without uploading a real screenshot.
- [ ] The `check` command verifies write access by creating and deleting `.claude-image-sync-check.tmp` in the configured user directory.
- [ ] If the configured hotkey cannot be registered, the client reports a clear error.
- [ ] Upload success and failure are visible in terminal logs and Windows toast notifications.
- [ ] Upload works without requiring the system `scp` command to be installed or available on `PATH`.
- [ ] The Windows client establishes SSH/SFTP connection and completes authentication at startup before accepting hotkey uploads.
- [ ] If the SSH/SFTP connection drops, the next upload trigger attempts reconnect and reports failures clearly.
- [ ] Upload supports private key authentication, including passphrase-protected keys via interactive prompt.
- [ ] Upload host key verification uses TOFU and rejects unexpected host key changes.
- [ ] The MVP works with a shared upload SSH account/key and documents that per-user directories are not a security isolation mechanism.
- [ ] Each successful hotkey trigger writes a new timestamped PNG file under `<shared_image_root>/<user_name>/`.
- [ ] Timestamped PNG filenames sort lexicographically by upload completion time.
- [ ] The MCP server selects the newest uploaded PNG for `CLAUDE_IMAGE_USER` by timestamped filename.
- [ ] Upload creates the configured user's directory automatically when it is missing.
- [ ] The Windows client accepts static clipboard images and copied PNG/JPEG files, then writes a normalized PNG.
- [ ] Normalized PNG output preserves original resolution and has no application-level size cap.
- [ ] Unsupported inputs such as animated GIF, WebP, PDF, and multi-file drag/drop are rejected with a clear message.
- [ ] A remote MCP server can return the latest cached image for `CLAUDE_IMAGE_USER` as image content.
- [ ] The MCP response includes the selected PNG filename and upload time parsed from the timestamped filename.
- [ ] The MCP server can run as a stdio MCP process from a container image.
- [ ] Setup instructions include a Docker `docker run -i --rm ...` stdio MCP client configuration example.
- [ ] Container setup instructions include mounting `/mnt/xy_internel/share/claude` to `/data/claude-images:ro`, setting `CLAUDE_IMAGE_ROOT=/data/claude-images`, setting `CLAUDE_IMAGE_USER=$USER`, and MCP client command configuration.
- [ ] If no image has been uploaded yet, the MCP tool returns a clear error.
- [ ] Setup instructions cover local daemon configuration, upload host configuration, shared storage mount assumptions, SSH/SFTP configuration, per-user directory layout, and MCP client registration.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
- Open questions:
  - None for product scope. Rust crate choices will be finalized in `design.md` / implementation based on ecosystem fit.
- Out of scope for MVP:
  - User-named images.
  - Podman-specific validation or examples.
  - Windows installer, tray app, service mode, or startup registration.
  - MCP tools for listing or selecting older images.
  - OpenSSH ControlMaster connection reuse.
  - Shelling out to `scp` for normal uploads.
  - Password-based SSH login.
  - Persisting private key passphrases.
  - Animated GIF, WebP, PDF, and multi-file drag/drop input.
  - Strong per-user storage isolation.
  - Automatic image resizing, compression, or application-level upload size limits.
  - Automatic upload on clipboard changes.
  - Separate metadata files such as `latest.json`.
  - Storage retention, quota management, cleanup, or pruning.
