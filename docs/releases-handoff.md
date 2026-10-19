# Windows releases and updater handoff
Author: Nitish

Paused at Nitish's request on 19 October 2026. This is incomplete source implementation, not a working release pipeline. Resume from this note; do not restart the native MVP or replace its code.

## Reference and intended behavior

Reference checkout: `D:\Nitish\Random\Code\Captrain`.

Read `.github/workflows/release.yml`, `scripts/release-version.cjs`, `scripts/release-version.check.cjs`, `src-tauri/tauri.conf.json`, `src-tauri/src/updates.rs`, `src/updates/` and the tray setup in `src-tauri/src/lib.rs` there.

Captrain uses Tauri's signed updater and NSIS packaging. Pushes to main/master create a Windows x64 release; CI adds the GitHub run number to the base patch and synchronizes version files without committing bumps. Assets go to a draft first, are validated, then published. Release apps check after 10 seconds and every six hours. Download and installation require separate explicit actions. Development does not fetch updates. Busy work prevents unsafe exit/install. There is no configured Authenticode certificate.

PieAll is native Win32 Rust, so retain its architecture rather than introducing Tauri. Its existing tray, single instance, close-to-hide dashboard and Ctrl+backtick invocation already provide the basic lifecycle.

## Implemented so far

- `src/updates.rs`: release-only checks, bounded HTTPS through `ureq` with native TLS, signed manifest verification, stable/newer semantic version validation, exact GitHub installer URL validation, streamed download with progress/256 MiB cap, installer signature verification, isolated temporary staging, re-verification and write/delete exclusion during installer launch. Successful launch retains the temporary directory so NSIS can still read the installer.
- `src/ui.rs`: update state owned by the UI thread, worker results/progress through posted messages, dashboard update button/status, tray update command, 10-second/six-hour timer, separate check/download/install actions, install confirmation and guards around active jobs/capture/editor/update exit.
- `Cargo.toml`, `Cargo.lock`, `src/lib.rs`: updater module and pinned dependencies (`ureq`, `minisign-verify`, `semver`; `tempfile` moved to runtime dependencies).
- Two updater unit tests were written for version/URL/signature/bounded-read rejection. They have not been run yet.
- `docs/superpowers/plans/2026-10-19-windows-releases.md`: original implementation outline.

Preserve `main.rs`'s `--capture` and `--worker` dispatch, `source_pid` capture, raw UI state pointer handling, native shell fixes and all existing commands. Another native MVP thread completed those changes and handed aggregate verification to this updater task.

## What remains, in order

1. Review and complete the updater/UI behavior before declaring it ready. Show release notes through an accessible native control/window; notes are parsed but not displayed yet. Recheck busy/capture/editor state after the reentrant install confirmation and prevent a hotkey from starting a conflicting capture during installation. Check network failure/retry, state transitions and shutdown behavior. Review temporary installer retention/cleanup policy. Add meaningful targeted tests for any new state logic.
2. Add `installer/pieall.nsi`: Windows x64 per-user NSIS installer, stable install location/registry record, Start menu shortcut, uninstall entry, bundled notices/licenses, no deletion of user outputs. `/UPDATE` must wait for the old PieAll process to exit; manual install/uninstall must refuse a running instance instead of killing active jobs. Offer reopening after installation. Do not silently enable startup-at-login; Captrain does not establish that requirement.
3. Add `scripts/release-version.cjs` plus Node built-in tests: base patch + `GITHUB_RUN_NUMBER`, stable/Windows version bounds, synchronize only PieAll's package version in root `Cargo.toml` and `Cargo.lock`, preserve dependency resolutions and handle LF/CRLF. No npm project or second lockfile is needed.
4. Add release packaging/signing and `.github/workflows/release.yml`: push main/master, Windows runner, pinned Rust/tool versions, checks, explicit x64 release build in CI, NSIS installer and portable ZIP, generated release notes, upload to draft, read back/verify every required asset before publishing. Keep release jobs serialized so an older run cannot replace the latest feed after a newer run.
5. Signing tooling considered: pinned `rsign2` 0.6.7 (`rsign` executable), interoperable with `minisign-verify`. `rsign generate`, `rsign sign FILE -s KEY -W`, `rsign verify FILE -P PUBLIC_KEY`. Confirm exact flags from the downloaded crate's README/source before writing CI. Use a dedicated PieAll key, never Captrain's private key. Keep private material outside the checkout and in the GitHub Actions secret. Do not log it or ask for it in chat.
6. Match the current runtime contract: compile `PIEALL_UPDATE_PUBLIC_KEY` into release binaries; it is the raw base64 public key, not the entire public-key file. Missing/invalid key disables checks. Feed is hardcoded to `https://github.com/NitishMPD/PieAll/releases/latest/download/latest.json`. Fetch and verify `latest.json.minisig` before parsing. JSON fields: `version`, `notes`, `url`, `signature` (the full textual installer Minisign signature). Installer URL must be exactly `https://github.com/NitishMPD/PieAll/releases/download/vVERSION/PieAll_VERSION_x64-setup.exe`. Sign both manifest and installer; signing metadata prevents version/URL substitution. Upload installer `.minisig` and signed portable ZIP too. Validate matching public/private keys before publishing.
7. Refresh `THIRD-PARTY-NOTICES.md` and `licenses/` for new resolved dependencies. Keep the existing inventory and license texts; include applicable transitive crates. Add generated-output/key ignores without creating or exposing private keys.
8. Update `project.md`, `design.md`, `README.md` and extend `verification.md` to describe the implemented network/release behavior and remaining runtime gates. Current docs still say no networking/signing/installer. Add `docs/releases.md` with exact repository/key setup, manual release commands, install/update acceptance and recovery steps. Extend the existing `tasks/task_19_oct.md` only after meaningful work is complete.
9. Run the checks below. Record actual results and failures in `verification.md`; preserve the existing native MVP record. Do not claim a tested installer/update until the user runs the acceptance pass.

The large patch intended to add installer/workflow/version scripts was rejected atomically because it targeted the new NSIS file twice in one patch. Those files were **not created**. The empty `scripts/`, `installer/`, `.github/workflows/` directories may exist. Recreate the files from the requirements above.

## Verification state and commands

`cargo check --all-targets --locked` passed after the updater/UI integration. An earlier unused `TempDir::keep()` result warning was corrected with `let _ = ...`. Formatting, strict Clippy and updater tests have not been run on the final integrated source. The native MVP thread previously passed 15 targeted native tests; that is not evidence that the updater passes.

Allowed code checks on resume:

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --lib updates::tests -- --nocapture
cargo test --locked --lib actions::tests -- --nocapture
cargo test --locked --lib output::tests -- --nocapture
cargo test --locked --lib boundary_tests -- --nocapture
cargo test --locked --lib concurrency_tests -- --nocapture
cargo test --locked --bin pieall ui::tests -- --nocapture
node --test scripts/release-version.test.cjs
```

Also parse new PowerShell scripts for syntax without executing packaging/signing. Validate workflow YAML and NSIS configuration statically where possible.

No builds, app launches, dev servers, branches, commits, pushes or deployment were run by this updater task. Do not run them without the standing instructions changing. Give the user exact build/installer commands after those files exist. Native UI, screen-reader behavior, clean-Windows install/uninstall, actual signature rejection, restart, upgrade preservation and GitHub publication remain user-run acceptance gates.

## External prerequisites

`git remote -v` returned no remote when inspected. GitHub repository creation/connection, repository signing variable/secret, and first release publication have not been configured or performed by this task. Confirm that `NitishMPD/PieAll` is the intended repository; if it changes, update both runtime and publishing URLs together. No signing key was generated. No private key is present in this checkout from this task.

Automatic checks work only in a release binary compiled with the matching public key. A development typecheck does not establish live update delivery. Decide/document that the portable ZIP's update action launches the installer rather than replacing the portable executable in place.
