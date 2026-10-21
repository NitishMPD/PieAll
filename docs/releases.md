# Windows releases
Author: Nitish

PieAll's release path is Tauri 2, per-user NSIS and the standard Tauri updater. The old Win32/custom-manifest handoff is preserved as history; do not publish its incompatible manifest format. Runtime/app architecture is documented in [tauri-port.md](tauri-port.md).

## Repository and signing setup

The configured repository and runtime feed are `NitishMPD/PieAll` and `https://github.com/NitishMPD/PieAll/releases/latest/download/latest.json`. The local origin matches. If the repository changes, update the runtime installer URL policy, updater endpoint and both release scripts together before shipping.

Use a dedicated PieAll signing key. Never reuse Captrain's key, put private material in the checkout or send it in chat. With the Tauri CLI installed, generate the key yourself outside this repository:

```powershell
pnpm tauri signer generate --write-keys "$env:USERPROFILE\.pieall-signing\pieall.key"
```

Create the parent directory first if necessary. Keep a separate protected backup of the private key and password. No production key was generated, read or configured by the implementation task. The checked-in public test vector is deliberately non-secret and must never be used for releases.

Configure GitHub Actions repository settings:

| Setting | Type | Value |
| --- | --- | --- |
| PIEALL_TAURI_PUBLIC_KEY | Variable | Complete content of the Tauri-generated `.key.pub` file, a base64-encoded public-key envelope |
| TAURI_SIGNING_PRIVATE_KEY | Secret | Complete Tauri private-key file content |
| TAURI_SIGNING_PRIVATE_KEY_PASSWORD | Secret | Key password, if set |

The public key is compiled into both the app and offline verifier. Missing or malformed public keys disable app update checks. Debug builds never check. The workflow checks configuration presence before building and verifies actual signatures against the compiled public key before publishing, so a mismatched key pair cannot pass the release gate. The updater enforces signed-version metadata; use the pinned CLI's automatic build signing or `pnpm tauri signer sign --app-version VERSION FILE` for manual artifacts. Do not use the legacy updater's raw-base64 key format.

No Windows Authenticode certificate is configured. Updater signing authenticates update artifacts; it is not Windows publisher signing and does not establish SmartScreen reputation. PieAll's own distribution-license choice remains an owner decision; third-party notices do not grant a license for PieAll itself.

## Automatic release flow

`.github/workflows/release.yml` runs on pushes to main/master, or a manual dispatch on those branches. It uses a Windows 2022 runner, pinned action commits, Node 24.11.1, pnpm 11.10.0, Python 3.12.10 and Rust MSVC 1.92.0. It runs source checks and targeted tests before packaging. Workflow runs share one concurrency group. Publishing also refuses a version older than or equal to the current latest release, so queue ordering cannot make an old run replace a newer feed.

The base version is the synchronized checked-in version. Release patch = base patch + GITHUB_RUN_NUMBER, with Windows version bounds and stable canonical versions enforced. The script changes only the root package.json version, Tauri config version, the two local Cargo package versions and their two Cargo.lock blocks. Dependency resolutions and pnpm-lock.yaml are unchanged. It records its run in ignored `.release/version.json`, making a retry of the same version step idempotent. A different run number requires a fresh checkout. CI does not commit version bumps.

Generated notes go into a draft release at the source commit. Drafts have no tag yet, so the script finds them by listing releases (the `releases/tags/<tag>` endpoint returns 404 for drafts), which also lets a failed run resume its draft. Packaging builds the x64 installer, creates a portable ZIP and signs both. The script assembles six required assets:

- `PieAll_VERSION_x64-setup.exe` and its `.sig`.
- `PieAll_VERSION_x64-portable.zip` and its `.sig`.
- `latest.json`, with a standard `windows-x86_64` Tauri target and the exact owned installer URL.
- `SHA256SUMS.txt`, covering the other five assets.

The offline `release-verify` CLI streams both artifacts through Minisign verification, authenticates their signed versions, enforces the stable version/URL/target contract and checks the inventory/hashes. It never initializes Tauri or launches an installer. Installer verification is limited to 256 MiB, ZIP verification to 512 MiB. The workflow uploads only to the draft, downloads every uploaded asset back, compares it byte-for-byte with local hashes, re-runs signature verification, rechecks monotonic latest and only then publishes. A failed gate leaves the release in draft. Do not manually publish a failed draft to bypass these checks.

The updater feed itself is ordinary HTTPS JSON. Its installer bytes and version are authenticated by the signature's trusted comment. Notes are bounded plain text. Installer URL must be exactly `https://github.com/NitishMPD/PieAll/releases/download/vVERSION/PieAll_VERSION_x64-setup.exe`. The legacy `latest.json.minisig` protocol is not used.

## Installer and portable behavior

NSIS installs for the current user with stable product/publisher identity, registry record, Start menu shortcut and uninstall entry. WebView2 comes from the system; the installer downloads its bootstrapper if needed. No startup-at-login setting is enabled. The standard finish page offers reopening the app. Downgrades are disabled.

`src-tauri/windows/installer-hooks.nsh` replaces the pinned CLI's force-close process guard with a Restart Manager check. Manual install/uninstall refuses a running executable. `/UPDATE` waits up to 60 seconds for the app/worker holding that executable to exit, then refuses rather than terminating anything. Failures to query running processes fail closed. Review these hooks against Tauri's upstream templates whenever the CLI is upgraded. Install/uninstall removes known program resources, not user output folders. Native acceptance is still required, including first installation, later upgrade and a running job.

Only one consolidated `THIRD-PARTY-NOTICES.txt` is installed and included in the portable ZIP. `licenses/` remains an internal retained-text cache; its hundreds of folders are not distributed separately. `python scripts/bundle-notices.py` deduplicates identical full texts while retaining every package attribution. Update the cache/inventory with `python scripts/update-notices.py` after dependency changes, then regenerate the consolidated resource. This conservative inventory also includes build dependencies; it does not claim every listed crate ships as executable code.

The portable ZIP contains PieAll.exe, README.txt and notices. It needs system WebView2. Its update action runs the standard installer and switches to the installed copy; it does not overwrite the portable executable. Do not run both copies together. Portable mode is not a claim that every OS/user-profile trace is confined to its folder.

## Local source checks

Use an x64 Visual Studio developer terminal with Desktop development with C++, Windows SDK, Rust/Cargo on PATH, Node and pnpm installed:

```powershell
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test
pnpm test:release
pnpm format:check
node scripts/release-version.cjs --check
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked --features release-tools
cargo clippy --workspace --all-targets --locked --features release-tools -- -D warnings
cargo test -p pieall --locked --lib
cargo test -p pieall-desktop --locked --bin pieall-desktop shell::tests
cargo test -p pieall-desktop --locked --features release-tools --bin release-verify
```

Build/dev server/app launches remain user-run. For an unsigned executable without an installer, use `pnpm tauri build --no-bundle`. After signing environment setup, `pnpm package:windows` builds just the installer at the checked-in version; it does not perform the complete release staging/publishing flow.

## Manual complete release

Use a fresh checkout of the exact intended committed source. Set the three signing variables above in your local environment without printing their secret values. Authenticate gh to the repository. Choose an unused positive run number that produces a version newer than the latest release; it should correspond to the intended GitHub workflow run to avoid collisions. Then explicitly run:

```powershell
$env:GITHUB_RUN_NUMBER = '42'
node scripts/release-version.cjs
node scripts/github-release.cjs draft
./scripts/package-release.ps1 -NotesFile .release/release-notes.txt
node scripts/github-release.cjs publish
```

The draft/publish commands modify GitHub. Packaging builds binaries and signs artifacts. These commands were documented and authored, not executed by the implementation task. Signed assets are staged under `.release/vVERSION/assets`; the readback lives beside it under `downloaded`. Staging never deletes existing directories; a retry needing fresh staging should use a fresh checkout after inspecting the old artifacts. No private key file belongs in staging.

## Acceptance and recovery

Before treating the first release as ready, run native Rust checks and the complete [verification pass](../verification.md). Test clean-Windows install/uninstall, a manual installer while a job is running, update handoff/reopen, preserved originals/outputs, bad signatures and mismatched signed versions, unavailable feed, interrupted download, retry after failed launch, stale install confirmation and portable-to-installed behavior. Verify no forced process termination or startup registration. Measure installed/ZIP/installer sizes and the full WebView2 process tree; size improvements remain unmeasured.

If CI fails, inspect its first failure and draft assets; fix source/configuration and rerun on a fresh runner. Do not change an already published artifact in place. Publish a higher corrective version for rollback/recovery; semver downgrade and installer downgrade protections are intentional. If an installed copy cannot open, close it and manually install a known trusted corrective build. User outputs are independent of the installation directory. Updater bytes are kept in memory only until install/exit; Tauri owns installer staging and retains what the launched installer still needs. Do not remove its temporary executable during installation. Private-key loss or compromise requires an owner-led signing-key/recovery plan; no silent key rotation is implemented.

Workspace all-target Rust checking and strict Clippy pass on this checkout. No product build, installer, signed release, live update or GitHub publication has been tested here. See verification.md for actual checks instead of inferring success from the workflow source.

References: [Tauri Windows installer](https://v2.tauri.app/distribute/windows-installer/), [Tauri updater](https://v2.tauri.app/plugin/updater/), and the pinned [installer template](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.3/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi) / [process-check macros](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.3/crates/tauri-bundler/src/bundle/windows/nsis/utils.nsh).
