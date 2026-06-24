# Design

## Summary

Build a Windows-focused Tauri desktop app as a thin control shell around the
existing `sync_image_client` workflow. The GUI owns form input, modal prompts,
and runtime controls. The existing Rust upload, clipboard, hotkey, and SSH/SFTP
logic remains the system of record and is refactored into reusable library
surfaces instead of being duplicated in frontend code.

## Scope

In scope for the first delivery:

- single-window Tauri desktop app
- configuration form for the existing client settings
- save/load config flow
- connectivity check flow
- runtime start/stop controls
- basic running/stopped status
- in-app host-key trust confirmation
- in-app private-key passphrase prompt without persistence

Out of scope for the first delivery:

- tray mode, startup registration, or background service installation
- log viewer, upload history, or dashboard UI
- Linux MCP behavior changes
- replacement of the existing CLI entry point
- passphrase persistence

## Proposed Architecture

### 1. Existing core stays authoritative

Keep these responsibilities in Rust domain code:

- `sync_image_core`: config schema, config validation, hotkey parsing, path and
  filename rules
- `sync_image_client`: clipboard capture, SSH/SFTP upload, runtime orchestration

The GUI must call into these layers, not recreate them in TypeScript.

### 2. Turn `sync_image_client` into a reusable library plus CLI binary

Today `sync_image_client` is only a binary crate. Refactor it into:

- `crates/sync_image_client/src/lib.rs`
- existing `src/main.rs` kept as a thin CLI wrapper

Expose reusable services for:

- config-oriented operations needed by the GUI
- connectivity check
- runtime start/stop orchestration
- interaction-aware SSH authentication and trust handling

This keeps CLI and GUI behavior aligned.

### 3. Add a new Tauri app

Add a new app directory, for example:

```text
apps/sync_image_desktop/
  package.json
  src/                # React + TypeScript + Vite frontend
  src-tauri/          # Tauri Rust crate
```

Add `apps/sync_image_desktop/src-tauri` to the workspace members so the Tauri
backend can depend on `sync_image_core` and `sync_image_client`.

### 4. Frontend stack

Use Tauri v2 with React + TypeScript + Vite.

Reasoning:

- mature Tauri template path
- simple component model for forms and modal flows
- strong typing at the UI/backend boundary
- no need for a global state library at MVP scope

Frontend state should stay local to the single control panel. Runtime status can
be polled or updated from direct command results; there is no MVP need for a
cross-window store or event log reducer.

## Cross-Layer Boundaries

### Boundary A: config form -> Rust config model

Frontend owns only display-friendly field state.

Rust remains the source of truth for:

- required fields
- hotkey syntax
- user name validation
- port validation

Planned contract:

- frontend sends a draft config payload
- backend converts it to `ClientConfig` and validates it
- backend returns field-safe validation errors or success

Frontend may do lightweight empty-field checks for UX, but must not become the
authoritative validator.

### Boundary B: GUI actions -> SSH/auth prompts

The current uploader performs terminal IO internally for:

- first-time host-key trust
- passphrase prompt after key auth fallback

That terminal assumption must be removed from domain logic.

Refactor the uploader/auth flow to return structured interaction requests
instead of reading stdin directly.

Proposed contract:

```text
Operation request
  -> Success
  -> Failure(message)
  -> NeedsInteraction(kind, payload)
```

`kind` for MVP:

- `trust_host_key`
- `private_key_passphrase`

The frontend handles the modal UX and re-submits the operation with the user's
answer. This is preferred over parsing CLI stdout or blocking a Tauri command on
frontend UI callbacks.

### Boundary C: GUI controls -> runtime lifecycle

The current hotkey runtime blocks inside a Windows message loop and has no GUI
stop hook. For GUI control, runtime ownership must move behind a controllable
handle.

Proposed backend-owned state:

- one `RuntimeManager`
- current status: `stopped | starting | running | stopping`
- optional active runtime handle

Proposed runtime handle responsibilities:

- spawn the hotkey loop on a background thread
- retain enough Windows/thread state to request shutdown
- join or clean up on stop

On Windows, implement stop by posting a quit message to the runtime thread
instead of force-killing the process.

## Service Refactor Plan

### Config operations

`sync_image_core::ClientConfig` currently supports loading and validation. Add
save/serialize helpers so GUI and CLI use the same write path.

Expected additions:

- `ClientConfig::to_toml()`
- `ClientConfig::save(path: Option<PathBuf>)`

### Interaction-aware uploader

Split the current uploader/auth logic into:

- pure SSH/SFTP work
- trust/passphrase interaction decisions

One workable shape:

- `ConnectOutcome::Ready(SftpUploader)`
- `ConnectOutcome::NeedsInteraction(InteractionRequest)`

`InteractionRequest` should carry structured data, for example:

- host, port, fingerprint for trust confirmation
- key path for passphrase request

The CLI wrapper can keep the current terminal experience by looping on the same
outcomes and resolving them through stdin/stdout prompts.

### Runtime controller

Replace the current `run(hotkey, on_trigger)` shape with a controller that can
be started and stopped.

Possible API shape:

- `RuntimeController::start(config) -> Result<RuntimeHandle, StartOutcome>`
- `RuntimeHandle::stop() -> Result<()>`

This controller should still call the existing clipboard capture and upload
logic so upload semantics do not drift.

## Tauri Backend Commands

Minimal backend command surface for MVP:

- `load_config() -> ConfigView`
- `save_config(config: ConfigDraft) -> SaveResult`
- `check_connection(config: ConfigDraft, interaction: Option<InteractionReply>)`
- `start_runtime(config: ConfigDraft, interaction: Option<InteractionReply>)`
- `stop_runtime() -> StatusResult`
- `runtime_status() -> StatusResult`
- `pick_private_key_path() -> Option<String>`

Shared response model for `check_connection` and `start_runtime`:

- `ok`
- `validation_error`
- `operation_error`
- `needs_interaction`

This keeps frontend branching explicit and avoids mixing prompt logic into
transport errors.

## Frontend UI Shape

Single window with three areas:

### Configuration form

Fields:

- user name
- hotkey
- upload host
- upload port
- upload user
- private key path
- shared image root

Controls:

- browse private key path
- save/apply
- reset to loaded config

### Connection actions

- run check
- inline success/error message

### Runtime controls

- start
- stop
- running/stopped indicator

No log pane, upload feed, or diagnostics drawer in MVP.

## Compatibility Notes

- Existing `sync-image-client` CLI remains supported.
- Existing config file path remains the default path.
- Existing upload path format and timestamp contract remain unchanged.
- Windows remains the only supported runtime target for hotkey upload behavior
  in the MVP.

## Risks and Mitigations

### Risk: prompt flow becomes tangled across CLI and GUI

Mitigation:

- define one shared structured interaction contract in Rust
- keep CLI and GUI as adapters over that contract

### Risk: runtime stop is unreliable

Mitigation:

- refactor hotkey runtime around an owned handle with explicit Windows shutdown
  path
- add tests around non-Windows no-op/error behavior where possible and manual
  verification on Windows

### Risk: frontend duplicates backend validation

Mitigation:

- keep authoritative validation in Rust
- only use frontend validation for immediate UX hints

## Alternatives Considered

### Launch the existing CLI as a subprocess from Tauri

Rejected because it would require fragile parsing of stdout/stderr, would not
give a clean start/stop lifecycle, and would turn host-key/passphrase prompts
into terminal-emulation work.

### Rebuild upload/auth logic directly in TypeScript

Rejected because it would duplicate the existing Rust behavior and create long-
term drift between CLI and GUI.
