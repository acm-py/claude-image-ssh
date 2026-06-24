# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

<!--
Document your project's quality standards here.

Questions to answer:
- What patterns are forbidden?
- What linting rules do you enforce?
- What are your testing requirements?
- What code review standards apply?
-->

(To be filled by the team)

## Scenario: Windows Release Workflow

### 1. Scope / Trigger

- Trigger: changes to GitHub Actions release packaging for the Windows upload client or Tauri desktop GUI.
- Scope: `.github/workflows/release.yml`, `examples/config.toml`, desktop app package files, and release documentation.

### 2. Signatures

- Workflow path: `.github/workflows/release.yml`.
- CLI job id: `build-windows-client`.
- Desktop job id: `build-windows-desktop`.
- CLI build command: `cargo build --locked --release --target x86_64-pc-windows-msvc -p sync_image_client`.
- Desktop build command: `npm run tauri -- build --target x86_64-pc-windows-msvc` from `apps/sync_image_desktop`.
- Binary path: `target/x86_64-pc-windows-msvc/release/sync-image-client.exe`.

### 3. Contracts

- Tags matching `v*` create a GitHub Release asset.
- `workflow_dispatch` supports manual build verification without creating a release.
- The release zip must be named `sync-image-client-windows-x86_64.zip`.
- The release zip must contain `sync-image-client.exe` and `config.toml`.
- The desktop GUI artifact must be named `sync-image-desktop-windows-x86_64`.
- The desktop job must run on `windows-latest`; Linux local checks may fail without WebKitGTK/pkg-config system libraries and do not validate the Windows hotkey path.
- The desktop bundle collection step must search both the workspace target
  directory (`target/<target>/release/bundle`) and the app-local Tauri target
  directory (`apps/sync_image_desktop/src-tauri/target/<target>/release/bundle`),
  then fail if no bundle files are found.
- The workflow must grant `contents: write` so `softprops/action-gh-release` can attach release assets.

### 4. Validation & Error Matrix

- Unknown Cargo package -> `cargo metadata --locked --format-version 1` or the workflow build fails.
- Wrong binary path -> package step fails while copying `sync-image-client.exe`.
- Missing desktop `package-lock.json` -> `npm ci` fails in the desktop job.
- Wrong Tauri bundle collection path -> desktop artifact upload fails or uploads an empty zip.
- Missing explicit empty-bundle check -> release can appear green while the desktop artifact contains no installer/bundle files.
- Missing release permission -> tag builds pass but release upload fails.
- Missing example config -> package step fails while copying `examples/config.toml`.

### 5. Good/Base/Bad Cases

- Good: pushing `v0.1.0` builds on `windows-latest` and attaches both `sync-image-client-windows-x86_64.zip` and `sync-image-desktop-windows-x86_64.zip`.
- Base: manual workflow run uploads the CLI and desktop artifacts but does not create a GitHub Release.
- Bad: uploading only the CLI `.exe` without `config.toml`, or adding a desktop GUI without a Windows Actions build job.

### 6. Tests Required

- Parse `.github/workflows/release.yml` as YAML.
- Run `cargo metadata --locked --format-version 1` after package-name changes.
- For desktop package changes, run `npm run build` in `apps/sync_image_desktop` and verify `package-lock.json` is committed.
- For desktop workflow changes, verify the bundle collection step checks both
  candidate Tauri target roots and throws when no files are found.
- Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked` before release workflow changes are committed.
- Run the GitHub Actions workflow on a tag before announcing a Windows release.

### 7. Wrong vs Correct

#### Wrong

```yaml
run: cargo build --release --target x86_64-pc-windows-msvc
path: target/x86_64-pc-windows-msvc/release/your_project_name.exe
```

#### Correct

```yaml
run: cargo build --locked --release --target $env:WINDOWS_TARGET -p sync_image_client
path: dist/${{ env.WINDOWS_RELEASE_ZIP }}
```

For the desktop app:

#### Wrong

```yaml
run: npm run build
```

#### Correct

```yaml
working-directory: apps/sync_image_desktop
run: npm run tauri -- build --target $env:WINDOWS_TARGET
```

## Scenario: Windows FFI With windows-sys 0.61

### 1. Scope / Trigger

- Trigger: changes to Windows-only client code that calls Win32 APIs through `windows-sys`.
- Scope: `crates/sync_image_client/src/*`, Windows target dependency features in `crates/sync_image_client/Cargo.toml`, and GitHub Actions Windows builds.

### 2. Signatures

- Hotkey APIs:
  - `RegisterHotKey(HWND, i32, HOT_KEY_MODIFIERS, u32) -> BOOL`
  - `UnregisterHotKey(HWND, i32) -> BOOL`
  - `GetMessageW(*mut MSG, HWND, u32, u32) -> BOOL`
  - `PostThreadMessageW(u32, u32, WPARAM, LPARAM) -> BOOL`
  - `GetCurrentThreadId() -> u32`
- Clipboard file APIs:
  - `IsClipboardFormatAvailable(u32) -> BOOL`
  - `GetClipboardData(u32) -> HANDLE`
  - `OpenClipboard(HWND) -> BOOL`
  - `DragQueryFileW(HDROP, u32, PWSTR, u32) -> u32`

### 3. Contracts

- `RegisterHotKey`, `UnregisterHotKey`, and `MOD_*` constants come from `windows_sys::Win32::UI::Input::KeyboardAndMouse`.
- `GetMessageW`, `PostThreadMessageW`, `MSG`, `WM_HOTKEY`, and `WM_QUIT` come from `windows_sys::Win32::UI::WindowsAndMessaging`.
- `GetCurrentThreadId` comes from `windows_sys::Win32::System::Threading`.
- `CF_HDROP` comes from `windows_sys::Win32::System::Ole` and must be converted to `u32` before passing to `DataExchange` APIs.
- Null `HWND` / `HANDLE` values are raw pointers in `windows-sys` 0.61; use `std::ptr::null_mut()` for inputs and `.is_null()` for returned handles.
- `Cargo.toml` must enable all feature modules used by imports, including `Win32_UI_Input_KeyboardAndMouse`, `Win32_UI_WindowsAndMessaging`, `Win32_System_Threading`, and `Win32_System_Ole`.

### 4. Validation & Error Matrix

- Missing `Win32_UI_Input_KeyboardAndMouse` feature -> unresolved import for `RegisterHotKey` or `MOD_*`.
- Missing `Win32_System_Threading` feature -> unresolved import for `GetCurrentThreadId` on Windows CI.
- Importing `RegisterHotKey` from `WindowsAndMessaging` -> unresolved import on Windows CI.
- Importing `CF_HDROP` from `System::DataExchange` -> unresolved import on Windows CI.
- Passing integer `0` where `HWND` is expected -> pointer type mismatch.
- Comparing `HANDLE` with integer `0` -> pointer type mismatch.
- Returning a runtime handle before `RegisterHotKey` succeeds -> GUI may show "running" even though the Windows hotkey failed to register.

### 5. Good/Base/Bad Cases

- Good: Windows CI builds `sync_image_client` for `x86_64-pc-windows-msvc` with `--locked`.
- Base: Linux CI passes native `cargo check`, `cargo test`, and `cargo clippy`, but this does not exercise Windows-only `cfg(windows)` code.
- Bad: relying only on Linux native checks after changing Windows FFI imports.

### 6. Tests Required

- For Windows FFI changes, run `cargo check --locked --target x86_64-pc-windows-msvc -p sync_image_client` in an environment with the target installed, or run the GitHub Actions Windows release workflow.
- Always run native `cargo fmt --check`, `cargo check --workspace --locked`, `cargo test --workspace --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MOD_ALT, RegisterHotKey, UnregisterHotKey,
};

RegisterHotKey(0, HOTKEY_ID, modifiers, key_code);
if GetClipboardData(CF_HDROP) == 0 {
    return Ok(None);
}
```

#### Correct

```rust
use std::ptr;
use windows_sys::Win32::System::Ole::CF_HDROP;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, RegisterHotKey, UnregisterHotKey,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PostThreadMessageW, WM_HOTKEY, WM_QUIT,
};

RegisterHotKey(ptr::null_mut(), HOTKEY_ID, modifiers, key_code);
let thread_id = unsafe { GetCurrentThreadId() };
PostThreadMessageW(thread_id, WM_QUIT, 0, 0);

let file_drop_format = u32::from(CF_HDROP);
let handle = GetClipboardData(file_drop_format);
if handle.is_null() {
    return Ok(None);
}
```

---

## Forbidden Patterns

<!-- Patterns that should never be used and why -->

(To be filled by the team)

---

## Required Patterns

<!-- Patterns that must always be used -->

(To be filled by the team)

---

## Testing Requirements

<!-- What level of testing is expected -->

(To be filled by the team)

---

## Code Review Checklist

<!-- What reviewers should check -->

(To be filled by the team)
