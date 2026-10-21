# macOS Support Implementation Plan
Author: Nitish

**Status:** Proposal only. No macOS support or workflow is implemented by this document.

**Goal:** Port PieAll to macOS while keeping the React interface, Rust tools and source-preserving behavior shared with Windows.

**Architecture:** Keep one Tauri application and one action engine. Isolate native desktop integration behind platform modules; keep session revisions, operation exclusion and action eligibility in the shared shell. macOS uses the system WKWebView rather than WebView2.

**Tech stack:** Existing Tauri 2, React/TypeScript and Rust workspace; AppKit, Accessibility and Core Graphics APIs for macOS integration; GitHub Actions macOS runners for compilation and packaging.

## Constraints and decisions

- This is an architecture and delivery plan, not a ready-to-apply implementation patch. Native API choices must pass the focused experiments below before promising feature parity.
- Start with Apple Silicon (`aarch64-apple-darwin`). Add Intel (`x86_64-apple-darwin`) after the first native acceptance pass. A universal binary can follow if supporting both justifies its size.
- Select and document the oldest supported macOS version after checking the pinned Tauri/Rust dependencies and available test machines. Apply the same deployment target to Rust, native dependencies and the bundle.
- Keep the legacy GDI executable Windows-only. Do not port it.
- Preserve captured source identity, revision checks, native authorization, one operation at a time, bounded workers and source-preserving output. Unsupported actions must be unavailable in both the UI and native dispatcher.
- Preserve keyboard and screen-reader access. Manual file selection and explicit clipboard input work without desktop automation permissions.
- Plan execution is inline unless the user requests delegation. Do not create branches, commit, push, deploy, launch apps or run builds without applicable user authorization. The commands below are instructions, not actions performed by this plan.
- Update `project.md` and `design.md` when implementation changes supported behavior or UX. This standalone proposal does not change their current Windows product contract.

## Findings from the current source

- Both Cargo manifests declare Windows bindings unconditionally.
- `src/platform.rs` owns Windows dialogs, clipboard, Explorer selection, input injection, executable matching and window actions.
- `src/actions.rs` includes direct HWND/PID validation, Explorer and ShellExecute calls. Its transformations and archive/text engines can largely remain shared.
- `src/output.rs` protects destination ancestors using Windows handles. Its non-Windows branch does not supply equivalent protection against destination-directory replacement.
- `src-tauri/src/shell.rs` imports Win32 APIs directly for focus, DPI, window placement, clipping and visual treatment. Its updater accepts a specific Windows `.exe` URL.
- `src-tauri/src/hold.rs` uses a Windows low-level keyboard hook and Windows key codes. Those key codes also reach shortcut parsing and customization.
- `src/shortcuts.json` identifies applications by Windows executables and contains Windows bindings. Mac app identity and bindings need separate data.
- `scripts/release-artifacts.cjs`, `scripts/package-release.ps1`, the offline release verifier and `.github/workflows/release.yml` assume Windows artifacts.

## Milestone 1: A useful macOS app with explicit input

**Files:** Modify `Cargo.toml`, `src-tauri/Cargo.toml`, `src/platform.rs`, `src/actions.rs`, `src/output.rs`, `src-tauri/src/main.rs`, `src-tauri/src/shell.rs`, `src-tauri/src/hold.rs` and `src-tauri/tauri.conf.json`. Create native implementation modules under `src/platform/` and `src-tauri/src/desktop/`, plus `src-tauri/tauri.macos.conf.json`.

- [ ] Move Windows crates into target-specific dependency sections. Gate legacy modules and binaries so a macOS workspace check does not compile Win32 code.
- [ ] Extract existing Windows implementation into platform-owned files. Use compile-time module selection and concrete functions; no plugin framework or service container.
- [ ] Move focus, monitor geometry, native window styling and platform invocation choices out of shared session coordination. Retain native source identity internally; do not cast a macOS object into an HWND-shaped integer or accept a frontend-supplied target.
- [ ] Implement file picking, clipboard text/image access and file reveal using native APIs or a fitting Tauri plugin. Use argument arrays for any subprocess; never interpolate selected paths into shell commands.
- [ ] Make catalog eligibility reflect platform support. Start with explicit file input, drop, clipboard text, image transformations, ZIP operations, hashing and reports. Keep unimplemented window/app actions inaccessible; a stub returning success is unacceptable.
- [ ] Implement Unix destination safety before enabling writes. Use directory-relative operations with retained directory handles and no-follow checks, preserving atomic publication without replacement. Test directory/symlink substitution during publication on APFS; do not treat the current empty non-Windows guard as protection.
- [ ] Keep `--worker` dispatch before WebView initialization. Verify helper execution from inside the `.app` bundle, timeout cleanup and bounded stdout/stderr. Preserve existing cleanup guarantees; process groups become necessary if engines start descendants.
- [ ] Split platform bundle targets: Windows NSIS, macOS `.app` and `.dmg`. Keep updater artifacts disabled for unsigned development packaging. Keep the existing Windows release pipeline intact.
- [ ] Provide native macOS menu/quit behavior and a usable menu-bar route. Closing the dashboard must not strand the app; Quit still refuses an active operation/editor.

**Proof:** Unit tests for unsupported-action rejection, output collisions, destination substitution and worker failure. Existing frontend tests and type check pass. On a Mac, opening files, running an image/text/ZIP operation, closing/reopening the dashboard and quitting work with all automation permissions denied. Compilation alone does not complete this milestone's acceptance.

## Milestone 2: Context capture and permission UX

**Files:** macOS platform/desktop modules, `src-tauri/src/shell.rs`, `src-tauri/Info.plist`, `web/main.tsx`, `web/native.ts`, `project.md`, `design.md`, `verification.md`.

- [ ] Capture the source application/process and focused accessibility element before PieAll takes focus. Revalidate source identity and current focus before any action that sends input or edits another application.
- [ ] Read selected text through Accessibility where exposed. Treat secure fields, empty selections and inaccessible applications as unavailable; keep explicit clipboard/file fallbacks.
- [ ] Prototype Finder selection with Finder automation. If Apple Events are used, add the usage description and request Automation access only when invoking that feature. Bound the request in a disposable helper so a blocked app cannot hang PieAll.
- [ ] Add contextual permission states for Accessibility and, where the chosen event-tap API requires it, Input Monitoring. Show why access is needed, a System Settings route and a retry check. Never request every permission at first launch.
- [ ] Start with a regular configurable global shortcut through the existing Tauri plugin. Choose its Mac default after testing OS/app conflicts; avoid assuming Windows Ctrl+backtick or macOS Command+backtick is appropriate.
- [ ] Port hold-to-open only after proving event-tap permission requirements, tap-versus-hold behavior, typed-key preservation, keyboard layouts, Secure Input and recovery when the event tap is disabled. Keep the ordinary shortcut and menu available without the hold hook.
- [ ] If a clipboard copy/paste fallback is needed, restore a snapshot only when the clipboard still contains PieAll's temporary payload. Never overwrite a newer user copy.

**Proof:** Native tests with permissions granted, denied and revoked; Finder and a supported text app; a secure field; a source app that exits; focus moving during capture; stale helper replies; multi-monitor placement; keyboard-only use and VoiceOver. Validate WKWebView storage sharing between dashboard/ring windows before relying on existing customization persistence.

## Milestone 3: Mac app shortcuts and window tools

**Files:** `src/shortcuts.rs`, `src/shortcuts.json`, a separate macOS shortcut data file, macOS platform modules, `src/actions.rs`, `web/Customize.tsx`, ring/menu code and related tests.

- [ ] Identify apps using bundle identifiers plus native process identity. Do not infer app identity from window titles or reuse `.exe` aliases.
- [ ] Add an explicit macOS shortcut catalog for a small verified set: Finder, Safari, Chrome, VS Code and Figma desktop. Bindings are reviewed per app; Ctrl-to-Command replacement is not a migration strategy.
- [ ] Use platform-neutral key names in shared data and separate native key translation. Migrate saved trigger-key preferences so Windows virtual-key numbers are not interpreted as macOS key codes.
- [ ] Preserve stable action identifiers where meaning matches; filter saved layouts by actual platform availability. Display Mac modifier symbols and expose accessible spoken labels.
- [ ] Send input only to the revalidated captured app. Release injected modifiers on every exit path; cancel on intervening focus/input changes and ignore PieAll's own injected events.
- [ ] Implement center/fit/details through supported accessibility attributes where the target allows them. Do not promise arbitrary always-on-top control over other apps; omit it unless a supported mechanism is demonstrated.

**Proof:** Catalog parsing/identity isolation tests and native per-app shortcut checks, including held modifiers, alternate keyboard layouts, app exit and focus changes. Mark dispatch as sent rather than claiming app-side completion.

## Milestone 4: Build and distribute without owning a Mac

You can edit and push source from Windows and have GitHub Actions compile it on an Apple-hosted macOS runner. You do not need to own a Mac, but this build still runs on macOS. Ordinary Windows/Linux cross-compilation is not the supported Tauri macOS packaging path.

**Files:** Create `.github/workflows/macos-build.yml`; later extend `scripts/release-artifacts.cjs`, `scripts/release-artifacts.test.cjs`, `scripts/github-release.cjs`, `src-tauri/src/release_verify.rs`, `src-tauri/src/shell.rs` and `docs/releases.md` for signed releases.

- [ ] Add a manual `workflow_dispatch` job with `contents: read`. Use a supported macOS runner, record `uname -m`, Xcode/SDK and toolchain versions, then pin a runner/toolchain combination after the first successful build.
- [ ] Reuse pnpm 11.10.0 and a Node version meeting `>=22.12.0`; install the native macOS Rust toolchain at Rust 1.92 or later. Install the Apple Silicon Rust target explicitly.
- [ ] Install locked dependencies; run source checks; compile and package the checked-out commit. GitHub builds committed source, not unsaved or uncommitted Windows edits.
- [ ] Upload `.app`/`.dmg` outputs as workflow artifacts. Do not publish a GitHub release in this first workflow. Zip a raw `.app` with macOS metadata/symlink preservation if distributing it separately; the DMG already carries the bundle.

Commands for that macOS job, **after milestones 1's compilation changes exist**:

```bash
pnpm install --frozen-lockfile
rustup target add aarch64-apple-darwin
pnpm typecheck
pnpm test
cargo check --workspace --all-targets --locked --target aarch64-apple-darwin
cargo test -p pieall --locked --lib --target "$(rustc -vV | sed -n 's/^host: //p')"
pnpm tauri build --target aarch64-apple-darwin --bundles app,dmg \
  --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

Expected bundle directory: `target/aarch64-apple-darwin/release/bundle/`. Cross-architecture compilation on a Mac is possible, but execute tests on the runner's native architecture. For Intel builds use `x86_64-apple-darwin`; for a universal app install both targets and use Tauri's `universal-apple-darwin` target. Native dependencies must support both slices.

- [ ] For public direct distribution, enroll in the Apple Developer Program and use a Developer ID Application certificate. Sign the bundle and nested code, apply required hardened-runtime entitlements, notarize with Apple and staple the delivered artifact where applicable. Store credentials in CI secrets; never in source or chat. This account is not required merely to compile a development build.
- [ ] Keep Apple signing separate from Tauri updater signing. Updater trust uses its own signature/key; Apple signing/notarization supplies the macOS distribution trust path. Unsigned or unnotarized test artifacts can encounter Gatekeeper restrictions.
- [ ] Add Darwin architecture entries and verified updater archives/signatures to release metadata. Replace the Windows-only runtime URL check with an exact allowlist derived from OS, architecture, repository and release version. Verify all platform assets before publishing one feed; do not let separate jobs overwrite each other's `latest.json`.
- [ ] Extend the offline verifier and release tests for Darwin archives, wrong-platform artifacts, wrong signatures, mismatched versions and unexpected asset URLs. Leave macOS automatic updates disabled until this path is implemented and exercised.

**Proof:** Download an artifact from a manual CI run, install and open it on a test Mac. For public releases verify signing/notarization and Gatekeeper behavior, then install a signed update and test interrupted download/retry. A green hosted build does not verify desktop automation or permissions.

## Practical access to macOS

- GitHub-hosted macOS: recommended for automated compilation and artifact delivery. Availability and minutes/cost depend on the repository and GitHub plan; check current pricing before budgeting.
- A rented Mac with a remote desktop, or a borrowed tester's Mac: use for permissions, Finder capture, keyboard hooks, menu-bar behavior, VoiceOver and installation acceptance. Hosted headless CI is insufficient for these checks.
- AWS EC2 Mac or a hosted Mac service can supply Apple hardware when sustained interactive access is needed. Check each service's minimum billing period and remote-access support.
- A Windows VM or a Windows cross-compiler does not replace the macOS SDK, Apple signing tools and interactive acceptance environment required by this delivery path.

## Recommended release boundary

Ship the explicit-input toolbox first on Apple Silicon, after native acceptance. Add Finder/text context and reviewed Mac shortcuts next. Hold-key invocation, broad app coverage and extra window controls remain gated by native API experiments and permission testing. Do not advertise the Windows shortcut count as macOS support.

This plan is based on repository source inspection and platform guidance; no Mac build, cost check or native experiment was performed. Before implementation, verify current runner labels, Tauri signing requirements and the chosen minimum OS against the official documentation.

## References

- Tauri prerequisites: https://v2.tauri.app/start/prerequisites/
- Tauri macOS distribution: https://v2.tauri.app/distribute/macos/
- Tauri macOS signing: https://v2.tauri.app/distribute/sign/macos/
- Tauri updater: https://v2.tauri.app/plugin/updater/
- GitHub-hosted runners: https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners
- Apple notarization: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
