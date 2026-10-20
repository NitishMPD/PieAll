# PieAll Tauri port
Author: Nitish

20 October 2026. PieAll remains in `G:\code\PieAll`. Captrain at `G:\code\Captrain` is the architecture reference, not the destination or a dependency. The approved direction supersedes the older instruction to retain GDI for the default shell.

## Structure

- Root Cargo workspace and single Cargo.lock. `pieall` remains the Rust library with the original 44-command registry, platform helpers, isolated workers and output guards. The original native executable is available only with the explicit `legacy-ui` feature.
- `src-tauri/` is the default workspace member, package/binary `pieall-desktop`. Main dispatches `--worker` and `--capture` before starting Tauri, so disposable helpers never initialize a WebView.
- `web/` is React/TypeScript with Vite. `native.ts` owns typed IPC contracts; `Ring.tsx` owns rounded SVG sectors and keyboard navigation; geometry/ranking is independently tested. Presentation receives action availability and names, not writable path/window identities.
- Native `shell.rs` owns context revisions, source HWND/PID, capture tickets, operation leases, editor ownership, results, tray, single-instance forwarding, global shortcut and update state. One operation at a time. The frontend cannot override the captured source through parameter dictionaries.
- Tauri capabilities permit local event subscriptions only. App commands additionally check their calling window label. No filesystem, shell, process, HTTP or updater permissions are granted to JavaScript. The CSP restricts remote resources; native command capabilities apply only to local surfaces.

## Context and interaction

Ctrl+backtick records the foreground window before showing the ring, clears prior context and captures selection using the existing disposable 1.2-second helper. Cancelled captures lose their ticket and cannot replace a newer selection. Window commands also carry the captured PID into the worker for revalidation. A failed capture leaves window actions available; Open files, native drop and explicit clipboard text remain the file/text fallbacks. The tray's Show action ring has the same invocation path. The dashboard's Show ring preserves its explicit current selection.

All 44 commands remain sourced from the Rust registry. Eligible image actions rank Convert, Crop, Resize, Add background and Compress; a sixth All actions sector opens the searchable browser. Convert offers the three supported formats only: PNG, JPEG and lossless WebP. Other formats in the visual references are not claimed or shown as working. Parameter editors remain numeric/text controls, with bounded first-image crop/frame/resize previews. Operations run on blocking workers, then use the existing child-process job protocol and 120-second deadline. Original/output publication rules are unchanged.

The ring is positioned at the cursor and clamped to the monitor work area. Its native region is circular so transparent corners do not intercept desktop clicks. Escape/center restores the recorded source when its PID still matches. Native focus, clipping and mixed DPI are source implementations awaiting runtime acceptance. Closing the dashboard hides it; tray Exit refuses a busy session/editor. Second launches forward file arguments to the running Tauri instance. Do not run the legacy and Tauri shells together.

## Packaging and size

The per-user Windows x64 NSIS configuration uses the system WebView2 runtime and downloads its bootstrapper only if needed. No fixed browser runtime, fonts, icon framework or Captrain recorder/cloud modules are bundled. Icons and ring geometry are local code/native assets. Production CSS is local; there are no font/CDN requests. Release Rust uses size optimization, LTO, one codegen unit, symbol stripping and abort-on-panic. The legacy updater's HTTPS stack is an optional feature; the default shell's updater uses Windows native TLS.

These settings express a size goal. The installer, installed app, WebView2 process tree and warm/cold latency have not been measured. Tauri may be larger than the old native executable, and system WebView2 is still a runtime dependency. Browser CSS translucency does not establish native backdrop blur; no Mica/acrylic/compositor blur is claimed.

## Updates and releases

The default shell now uses Tauri's updater, matching Captrain's architecture. The old custom signed-manifest implementation remains only for the legacy shell. Its `latest.json.minisig` schema and `/UPDATE` custom-installer plan are obsolete for the Tauri release path. Do not publish both protocols to the same feed.

Checks are disabled in debug builds and without `PIEALL_TAURI_PUBLIC_KEY` compiled into the desktop crate. Configured releases check after 10 seconds and every six hours, skipping busy/editor sessions. The dashboard offers manual check, accessible release notes, explicit download and explicit install confirmation. Only dashboard IPC may initiate updates. The installer launch holds the same native lease used to exclude jobs, capture, editors and exit.

Feed: `https://github.com/NitishMPD/PieAll/releases/latest/download/latest.json`. Use Tauri's Windows x64 NSIS manifest format and exact installer URL `https://github.com/NitishMPD/PieAll/releases/download/vVERSION/PieAll_VERSION_x64-setup.exe`. Stable versions only. Release notes are capped at 128 KiB. HTTPS download is limited to 256 MiB and ten minutes; response length is checked. Bytes stay in bounded memory until install/exit, unlike the legacy disk-staging design. Tauri verifies the installer signature and signed-version metadata on download and again on installation. `requireSignedVersion` is enabled. Failed install consumes the cached bytes; download again to retry. The draft-first CI workflow and packaging/signature verifier are implemented; see releases.md. Dedicated production keys, Authenticode and live release/installer acceptance are not configured or verified.

Generate a dedicated PieAll key outside the repository using `pnpm tauri signer generate`; keep its private key in a local secret store/GitHub Actions secret, never in chat or source. Use the complete Tauri public-key file content as the value of `PIEALL_TAURI_PUBLIC_KEY` before compiling, and configure `TAURI_SIGNING_PRIVATE_KEY`/`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` for packaging. The raw-base64 key used by the legacy updater is a different format. Never reuse Captrain's key. Tauri CLI 2.12.3 and updater 2.14 require version-bound updater signatures; verify the generated `.sig` with the plugin before first publication. No key was generated or read by this task.

## User-run commands

Install Visual Studio 2022 Build Tools with Desktop development with C++ and the Windows SDK. Use Rust MSVC 1.92+ with Cargo on PATH, Node 22.12+ and pnpm 11.10.0. Run source checks from an x64 developer terminal:

```powershell
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test
pnpm format:check
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p pieall --locked --lib
cargo test -p pieall-desktop --locked --bin pieall-desktop shell::tests
```

When ready to launch/build yourself:

```powershell
pnpm tauri dev
pnpm tauri build --no-bundle
```

The packaged executable is `target\release\pieall-desktop.exe`. With PieAll signing environment variables configured, run `pnpm package:windows`; NSIS artifacts are under `target\release\bundle\nsis\`. Builds, dev servers, app launches, branches, commits, pushes and deployments were not run by this port task.

`node scripts/preview-ui.cjs` renders actual React source to `verification/tauri-layout.html` with an illustrative selection, without a server, application launch or frontend bundle. It is a layout fixture, not an IPC/runtime test. `python scripts/update-notices.py` refreshes the Windows Cargo/production JS license inventory without building.

## Acceptance gates

Repeat every native acceptance scenario in verification.md against the Tauri executable. Add transparent-corner click-through, ring focus loss/return, conversion submenu/back/number keys, screen-reader SVG/button semantics, forced-colors/reduced-motion settings, minimum dashboard width, parameter editors, image orientation/previews, reentrant invocations, second-launch file forwarding and tray reopening during a job. Validate clean install/uninstall, preservation of outputs, signed-version rejection, interrupted updates, user cancellation before installation and restart on a clean Windows 11 x64 machine. Measure complete process-tree memory and artifact size. Source rendering does not verify any of these native behaviors.

## Release tooling completion

See [releases.md](releases.md) for the completed versioning, signed installer/portable packaging, installer process guard and draft/readback/publish workflow. One consolidated THIRD-PARTY-NOTICES.txt is distributed; licenses/ remains an internal source cache. Install confirmation is bound to its reviewed version, and malformed public keys disable update checks. Rust all-target checking, strict Clippy and 17 targeted native tests now pass; the earlier missing-linker limitation is historical. The old custom-release handoff is historical.
