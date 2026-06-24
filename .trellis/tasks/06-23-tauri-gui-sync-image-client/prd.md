# Tauri GUI for sync-image client

## Goal

Add a desktop GUI version of the existing Windows `sync-image-client` using Tauri, so the user can configure, validate, and run the image upload workflow without relying on a terminal-first CLI experience.

## Confirmed Facts

- The current repository is a Rust workspace with three crates: `sync_image_core`, `sync_image_client`, and `sync_image_mcp`.
- `sync_image_client` is currently a Windows foreground CLI with two commands: `run` and `check`.
- `run` registers a hotkey, captures the current clipboard image or copied PNG/JPEG file, normalizes it to PNG, and uploads it through SSH/SFTP.
- `check` validates configuration, SSH/SFTP connectivity, host key behavior, and remote write access.
- The current config model already defines the main user-editable fields: `user_name`, `hotkey`, `upload.host`, `upload.port`, `upload.user`, `upload.private_key_path`, and `upload.shared_image_root`.
- Hotkey parsing currently supports modifier keys plus a single ASCII letter or digit.
- The current runtime emits operational feedback through terminal output and Windows toast notifications; it does not yet expose a structured event/state stream for a GUI.
- The current README explicitly lists GUI-related capabilities as out of scope for the MVP, including tray app and startup registration.
- There is no existing frontend application or Tauri scaffold in the repository today.

## Requirements

- Provide a GUI-based desktop entry point for the existing `sync-image-client` workflow.
- The first GUI release should be a thin management shell over the existing client flow, not a full product redesign.
- Preserve the existing core behavior and storage contract instead of redefining the upload protocol.
- Reuse the current Rust core/client logic where practical, rather than reimplementing upload behavior separately in frontend code.
- Allow the user to manage the client configuration without editing TOML manually.
- Allow the user to run the equivalent of the existing `check` flow from the GUI.
- Allow the user to start and stop the hotkey-driven upload runtime from the GUI, or otherwise make runtime status visible and controllable from the GUI.
- The first GUI MVP should provide a minimal control panel only: configuration form, save/apply, connectivity check, start/stop control, and a basic running/stopped indicator.
- The first GUI MVP should handle first-time SSH host-key trust confirmation in-app and prompt for private-key passphrases on demand without persisting the passphrase.
- Keep the initial GUI scope focused on the Windows desktop client, not the Linux MCP server.

## Constraints

- The solution should use Tauri as the desktop GUI framework.
- Planning must treat this as a complex task because it introduces a new frontend/Desktop layer into a currently Rust-only CLI workspace.
- The existing Rust crates are the source of truth for upload/config/runtime behavior; any new GUI should integrate with them instead of drifting from them.

## Acceptance Criteria

- [ ] A documented MVP scope exists for the Tauri desktop client, including what is in scope and explicitly out of scope.
- [ ] The planned GUI workflow covers configuration editing, connectivity checking, and runtime control/status for the Windows client.
- [ ] The plan identifies how the Tauri app will reuse or call into existing Rust logic instead of duplicating the upload implementation in the frontend layer.
- [ ] The plan identifies whether the existing CLI remains available alongside the GUI.
- [ ] The plan treats the Tauri app as a thin shell and keeps the current CLI available as the underlying and parallel entry point.
- [ ] The planned first-release UI stays at the minimal control-panel scope and does not require log panels or recent upload history.
- [ ] The planned GUI flow covers host-key trust confirmation and passphrase entry without requiring a terminal or storing the passphrase.

## Likely Out of Scope

- Changes to the Linux `sync_image_mcp` server behavior.
- Redesigning the NAS storage contract or upload naming contract.
- Non-Windows desktop targets in the first delivery unless later requested.
- Replacing the existing CLI as the only supported Windows entry point in the first delivery.
- Recent upload history, rolling event logs, and advanced runtime dashboards in the first delivery.
- Persisting SSH passphrases in application storage in the first delivery.
- Broader product changes such as image history browsing, retention management, or automatic clipboard-sync semantics unless later requested.

## Open Questions

- None at current MVP scope.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
