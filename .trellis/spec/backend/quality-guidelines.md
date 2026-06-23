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

## Scenario: Windows Client Release Workflow

### 1. Scope / Trigger

- Trigger: changes to GitHub Actions release packaging for the Windows upload client.
- Scope: `.github/workflows/release.yml`, `examples/config.toml`, and release documentation.

### 2. Signatures

- Workflow path: `.github/workflows/release.yml`.
- Job id: `build-windows-client`.
- Build command: `cargo build --locked --release --target x86_64-pc-windows-msvc -p sync_image_client`.
- Binary path: `target/x86_64-pc-windows-msvc/release/sync-image-client.exe`.

### 3. Contracts

- Tags matching `v*` create a GitHub Release asset.
- `workflow_dispatch` supports manual build verification without creating a release.
- The release zip must be named `sync-image-client-windows-x86_64.zip`.
- The release zip must contain `sync-image-client.exe` and `config.toml`.
- The workflow must grant `contents: write` so `softprops/action-gh-release` can attach release assets.

### 4. Validation & Error Matrix

- Unknown Cargo package -> `cargo metadata --locked --format-version 1` or the workflow build fails.
- Wrong binary path -> package step fails while copying `sync-image-client.exe`.
- Missing release permission -> tag builds pass but release upload fails.
- Missing example config -> package step fails while copying `examples/config.toml`.

### 5. Good/Base/Bad Cases

- Good: pushing `v0.1.0` builds on `windows-latest` and attaches `sync-image-client-windows-x86_64.zip`.
- Base: manual workflow run uploads the same zip as an Actions artifact but does not create a GitHub Release.
- Bad: uploading only the `.exe` without `config.toml`, because the MVP distribution contract is one executable plus example config.

### 6. Tests Required

- Parse `.github/workflows/release.yml` as YAML.
- Run `cargo metadata --locked --format-version 1` after package-name changes.
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
