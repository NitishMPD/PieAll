# Verification record
Author: Nitish

## Tool expansion, jobs and engines ? 21 October 2026

Implemented code increases the local registry from 44 to 66 commands (seven PDF and 15 optional media commands), adds engine setup, richer image/PDF settings, folder ZIP creation and native job progress/Close/Cancel. Current implementation and limits are documented at the top of project.md; the remaining-work matrix is section 13.15 of PieAll-plan.md.

Passed source checks:

- `cargo test -p pieall --lib --locked`: 29 tests. Covers source/collision preservation, cancellation publication ordering, ZIP traversal/quotas/empty folders/compression choices, PDF ranges/form refusal/extraction/splitting/inherited rotation/metadata, image page settings/resize modes, engine persistence/hash mismatch, compatible inputs and typed media argument rejection.
- `cargo check --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `pnpm typecheck` and `pnpm exec vitest run web/editor-fields.test.ts web/menus.test.ts web/action-search.test.ts`: 14 tests.
- `node scripts/preview-ui.cjs` and `node scripts/preview-ui.cjs --shortcuts`: source-only React static rendering; the latter checks contextual action rows/pagination and no unrelated app leakage. Corrected the preferences server snapshot and deferred title-bar native-window access to effects/click handlers so these checks run without a browser.
- Changed-file Prettier, `cargo fmt --all --check` and `git diff --check`.
- Third-party notices regenerated: 343 inventoried dependencies, zero missing license-text reviews; 229 consolidated texts. Bundling normalizes trailing whitespace while original retained license files remain available.
- Impeccable source detector reported two advisory easing matches in existing ring/toast CSS. The established motion was retained; this is not visual/native acceptance.

No app, engine executable, dev server or build was launched for this work. Real encoding, live cancellation during a codec, actual executable-version/codec support, packaged worker dispatch, native toast Close/Cancel hit testing/focus/accessibility/DPI, crash/forced-kill cleanup, disk-full behavior and PDF fidelity across varied external documents remain unverified.

User-run acceptance command: `pnpm tauri dev`. Let first-use setup install FFmpeg/FFprobe (or select custom paths in Advanced), exercise each media format with representative video/audio, inspect playback and output size, and test cancellation/Close during encoding and finalization. Exercise PDF page settings/ranges/rotation, a signed/form PDF refusal, duplicate filenames, readonly targets, ZIP folders with links/empty folders and image crop/gradient previews. A complete release build remains user-run: `pnpm package:windows` with the documented signing setup. Browser downloads have source coverage but need native/site acceptance; boards, advanced editors and the rest of the catalog remain planned.

## Productivity default profiles - 21 October 2026

The current catalog has 3,108 app bindings across 57 profiles, plus 44 local tools. Updated 49 quick lists, preserved existing action IDs and saved layouts, added browser Copy URL and Figma alignment/distribution, and kept recognized app actions primary over selected text. This supersedes the selected-text precedence statements in older records below.

Passed: `pnpm exec vitest run web/menus.test.ts web/profile-defaults.test.ts web/action-search.test.ts` (17 tests); `cargo test -p pieall --lib shortcuts::tests` (3 tests); `pnpm run typecheck`; `cargo check -p pieall-desktop --all-targets`. Tests cover every profile's six defaults, browser Copy URL/private browsing, Figma's eight alignment choices and reachable services, editor selection routing, generic text fallback and saved customization. Changed-file Prettier and `cargo fmt --all -- --check` also passed. These checks do not prove target-app key delivery.

User-run acceptance: `pnpm tauri dev`. Check Copy URL in supported browsers, including full URL clipboard contents and focus after Escape. In Figma desktop, check property transfer, both flips, component/auto layout and all alignment/distribution choices on eligible layers. Select code in VS Code/Cursor and confirm app actions remain primary and Text tools reachable; check the intended editor pane, since pane detection is not implemented. Verify saved custom layouts remain intact and Reset adopts the defaults. Exercise mouse, number keys, arrows, Enter, Escape and hold navigation through Align/More. No build, app launch or browser automation was run for this update.

## Context-only shortcut correction ? 21 October 2026

The dashboard now indexes the native captured session, with a shared profile guard for dashboard and ring. Regression cases cover Figma to Photoshop changes, unsupported/missing app context, unrelated entries incorrectly marked eligible, selection-tool precedence, six direct shortcuts and flat Search actions. The former all-app browse fixture below records the earlier implementation and is superseded by this correction.

Passed: `pnpm run typecheck`; `pnpm test` (14 tests); the five native shell unit tests; strict desktop Clippy; Rust formatting; changed-file Prettier; and `node scripts/preview-ui.cjs --shortcuts` (58 contextual rows, six direct app choices, flat search, no unrelated app rows). No browser automation was needed for these source-verifiable changes.

User-run acceptance: `pnpm tauri dev`. Invoke from Figma, Photoshop and an unsupported app; verify only the captured app's commands appear. Use Search actions and `/` to open and focus the flat Apps search; switch apps after entering a query and confirm the query/page reset. Check mouse release, click, number keys and focus restoration while dispatch remains restricted to the captured app. No app, dev server or build was launched for this correction.

## Contextual app library — 21 October 2026

At the original catalog pass, shared keymaps expanded into 3,079 action IDs across 57 profiles, plus the existing 44 local tools. Source validation checks unique profile/executable/action IDs, known key grammar, resolvable IDs and six valid quick choices per profile. It does not prove every default binding against installed app versions.

Passed for this change:

- `cargo test -p pieall --lib shortcut --locked`: 4 passed, covering the bundled catalog/parser, global-invocation collision detection and fail-closed invalid window input (no real keystrokes sent).
- `cargo test -p pieall-desktop --bin pieall-desktop shell::tests --locked`: 5 passed, including session exposure restricted to the captured profile.
- `pnpm test`: 14 passed, covering geometry, recursive menu reachability, ranked direct actions, long labels, whole-chord/token search, profile eligibility and pagination across the full shipped catalog.
- `pnpm run typecheck` and `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `node scripts/preview-ui.cjs --shortcuts`: static rendering asserts 60 rows, app quick choices and pagination; its fixture includes 3,083 rows (full app library plus four window tools). A 1,000-query Node run reported about 0.16 ms mean search time and 0.64 ms index construction on this machine. These are microbenchmarks, not native app latency or UI frame-rate measurements.
- One 1040×740 browser image of the static actual React fixture checked eight Figma choices and bounded results. A root-relative logo preload produced an access-denied message in the headless fixture; its inlined logo rendered. No app, dev server or build was launched.

The broad frontend formatter also flagged five untouched files (`web/bootstrap.tsx`, `web/geometry.test.ts`, `web/geometry.ts`, `web/Toast.tsx`, `web/tokens.css`); they were not reformatted as part of catalog work. Changed frontend files are checked separately. A transient Windows incremental-cache access warning during an earlier test invocation did not recur in the later checks.

User-run acceptance: start with `pnpm tauri dev` (or build without an installer using `pnpm tauri build --no-bundle`). From Explorer, Chrome/Edge, Word/Excel, Figma, Photoshop, a code editor and a terminal, verify all six quick choices, flat Search actions, keyboard navigation, immediate release after a flick, click while holding, captured-app search/pagination, Escape/focus restoration and Narrator. Exercise mode-dependent commands in their intended mode. Explicitly check Ctrl+backtick actions in VS Code and Excel, then confirm PieAll's own hotkey still works. Change focus or close the source before dispatch and during a multi-chord action; no further keys should reach another app. Check clipboard behavior, elevated targets, US/alternate layouts, dialogs, app-customized keymaps and actual aliases. Measure cold/warm invocation, hold-to-action latency, frame consistency and process-tree memory. None of those native checks was run in this session.

## Hold-to-open, in-place actions — 20 October 2026

Passed after the invocation rework:

- `cargo fmt --all`, `cargo check --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `cargo test --workspace --locked`: 18 passed (14 library, including smart naming, compress/scale outputs and transparency-preserving compression; 4 shell, including toast summaries).
- `pnpm run typecheck`, `pnpm run test` (6 passed: geometry, sector-by-angle, menu resolution) and `pnpm run format:check`.

Not verified, user-run: the keyboard hook's tap/hold timing and replay order in real apps, foreground activation from the hook, Explorer COM capture on the desktop and Windows 11 tabs, Ctrl+C/Ctrl+V text round-trips and clipboard restoration in common apps, toast placement and click-through, ring translucency over varied backgrounds, and mixed DPI.

## Release handoff completion — 20 October 2026

The current release tooling supersedes the earlier blocked port check below. The toolchain now completes native source checks; no missing-linker blocker remains for this checkout's checks. An updater async result needed an explicit Result type, which was corrected. The initial combined frontend run discovered the Node test files and failed discovery; Vitest is now scoped to web, with release tools run separately through Node's test runner.

Passed on the final release source:

- `cargo fmt --all -- --check`.
- `cargo check --workspace --all-targets --locked --features release-tools`.
- `cargo clippy --workspace --all-targets --locked --features release-tools -- -D warnings`.
- `cargo test -p pieall --locked --lib actions::tests`: 4 passed.
- `cargo test -p pieall --locked --lib output::tests`: 2 passed.
- `cargo test -p pieall --locked --lib boundary_tests`: 5 passed.
- `cargo test -p pieall --locked --lib concurrency_tests`: 1 passed.
- `cargo test -p pieall-desktop --locked --bin pieall-desktop shell::tests`: 3 passed, including public-key validity and reviewed-version confirmation.
- `cargo test -p pieall-desktop --locked --features release-tools --bin release-verify`: 2 passed, including valid authentication, byte/comment tampering, version mismatch, malformed keys/signatures, missing signed version and size rejection. Test vectors contain only a public key/signature from a deliberately non-secret fixture.
- `pnpm typecheck`, `pnpm test` (2 frontend tests), `pnpm test:release` (7 Node tests), `pnpm format:check` and release-script `node --check` parsing.
- `node scripts/release-version.cjs --check`: source versions remain 0.1.0; no release bump was applied to this working tree.
- PowerShell parser accepted package-release.ps1 without executing packaging/signing. Python AST parsing passed; workflow YAML, triggers and shared concurrency were parsed/checked. Installer guard was reviewed statically against pinned Tauri CLI 2.12.3 templates; no NSIS installer compilation/runtime test is claimed.
- `python scripts/bundle-notices.py`: 303 packages consolidated into 196 distinct full texts in one 655502-byte resource. The original retained-text folders are not separately bundled.
- `git diff --check` passed.

That is 17 targeted native tests plus 9 JavaScript tests. Unit-test compilation/execution and source checks are not product builds or app launches. No product build, dev server, app/installer launch, production key generation, signing, GitHub write, branch, commit, push, publication or deployment was performed. The workflow was authored, not run.

Unverified acceptance gates remain in docs/releases.md: clean Windows installation/uninstallation; manual-install refusal and bounded updater waiting; native UI/focus/DPI/accessibility; real release key matching; installer/portable signing, upload/readback/publication; restart and output preservation; live retry/recovery; measured artifact size and WebView2 process-tree cost. Dedicated repository signing configuration and the product owner's distribution-license choice remain external setup/decisions.

## Tauri port — 20 October 2026

Current default shell: Tauri 2 + React/TypeScript, package pieall-desktop, with the original Rust action library. The records below this section describe the earlier native implementation and do not certify the new shell.

Passed for the port source:

- `pnpm typecheck`.
- `pnpm test`: two geometry/context-ranking tests.
- `pnpm format:check`.
- `cargo fmt --all -- --check`.
- `cargo metadata --no-deps --locked --format-version 1`: workspace/lockfile metadata resolves.
- Production JavaScript/Windows Cargo dependency inventory refreshed: 303 dependencies, zero missing license texts in the generated inventory. Some upstream archives omit texts; commit-pinned upstream copies/standard MPL text are attributed in their license directories.
- Actual React components rendered to a static illustrative selection at 1040px, with no browser console errors. An initial preview transport was truncated/incorrectly decoded and was corrected before the valid layout inspection. This establishes source layout only; no app/dev server/frontend production bundle was launched. Minimum-width behavior is covered by source CSS, not a verified native screenshot.
- Tauri CLI configuration inspection and signing/build help were read. Its signer supports app-version binding, and build signs version metadata automatically; no key or signed artifact was generated.

Blocked: `cargo check --workspace --all-targets` failed because MSVC `link.exe` is not installed/available. Rust itself is installed (1.97.1), but not on this terminal's default PATH; checks used its explicit user Cargo directory. Tauri environment inspection also found no Visual Studio/MSVC C++ toolchain. No port Rust type-check, strict Clippy or native test success is claimed. The two new shell-state tests are written but not executed. Earlier native-MVP test results are historical, not proof that the port compiles.

Use an x64 Visual Studio developer terminal with Rust on PATH, then run the source-check commands in docs/tauri-port.md. User-run development command: `pnpm tauri dev`. User-run release executable build: `pnpm tauri build --no-bundle`. Signed NSIS packaging after dedicated key setup: `pnpm package:windows`. These were not executed by the agent.

Unverified: helper dispatch in the packaged executable; all 44 commands through Tauri IPC; file drops and forwarding; tray/shortcut conflicts; selection cancellation/reentry; native focus/transparent corners; screen readers and WebView2 keyboard semantics; preview alignment and memory; mixed DPI/text scaling; clean install/uninstall/restart; signed-version/installer rejection and network retry; artifact size and complete process-tree performance. Follow the expanded acceptance gates in docs/tauri-port.md as well as the native scenarios below. No branch, commit, push, deployment or publication was performed.

## Historical native verification

Date: 19 October 2026. Environment: Windows, Rust 1.92.0 MSVC host. This record describes code checks, not a tested app release.

## Passed automated checks

- `cargo fmt --all -- --check`
- `cargo check --all-targets --locked`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --locked --lib actions::tests -- --nocapture`: registry identity, Unicode text round trips/malformed input, source-preserving image transform and crop rejection, ZIP round trip.
- `cargo test --locked --lib output::tests -- --nocapture`: archive traversal/device/stream rejection and output collision/source preservation.
- `cargo test --locked --lib boundary_tests -- --nocapture`: malicious ZIP rejection, PDF object offsets and contact-sheet dimensions, read-only duplicate/folder scan, mixed-target eligibility, invalid image parameters leave no output.
- `cargo test --locked --lib concurrency_tests -- --nocapture`: eight simultaneous output publications preserve all outputs and the original, with no abandoned temporary files.
- `cargo test --locked --bin pieall ui::tests -- --nocapture`: radial center/outside/wedge hits, image-action ordering, bounded rounded wedge geometry.

These are 15 targeted tests. Worker subprocess integration, clipboard, shell actions and rendering were not executed. The PDF test validates offsets and structural tokens, not a third-party PDF reader.

## User-run acceptance pass

Build: `cargo build --release --locked`
Launch: `.\target\release\pieall.exe`

1. Select a PNG/JPEG in Explorer; invoke Ctrl+backtick. Confirm target name, six wedges, no accidental execution on hover, Escape focus restoration, More opens all actions.
2. Open Crop, Resize and Frame. Confirm correct orientation, preview alignment, numeric keyboard editing, invalid-input messages, and source-preserving results beside the source.
3. Try a multi-selection with an unsupported file; confirm image tools are ineligible. Change Explorer tabs rapidly and cancel/reinvoke during capture; no stale reply may execute. Test desktop separately; explicit Open files/drop remains the fallback.
4. Test all 44 commands from the action list with representative fixtures, including a damaged image/ZIP, denied permissions, unsupported filesystem hard links, disk-full and simultaneous external filename creation.
5. Explicitly load clipboard text, transform it, then Copy the result. Confirm no clipboard read on invocation and no automatic replacement except the requested copy actions. Test bitmap clipboard alpha in Paint and other target apps.
6. Verify window commands target the captured source app; close it before execution and confirm rejection. Test protected/minimized windows.
7. Check Tab/arrows/Enter/Escape, screen-reader labels through the standard list, high contrast, text scaling, small work areas, negative monitor coordinates, mixed DPI, shortcut conflicts, sleep/resume, Explorer restart, tray exit while a job runs.
8. Measure actual warm/cold invocation, process-tree memory/CPU, preview memory, artifact size and first startup. None of the plan's budgets has been measured or proven.

## Remaining gates

Direct2D glass renderer; custom-ring UI Automation; modifier/OLE drag; configurable shortcuts and exclusions; persistent storage/history; user cancellation and cleanup/recovery journal; optional engines; browser/boards/packs/hub; release license choice, installer, signing and clean-Windows validation. No push/deployment occurred.

## Concurrent integration note
At 17:56 IST another active project thread began updater integration in the shared checkout. Its dependency/module edits temporarily made aggregate checks fail (`updates.rs` not present yet and Cargo.lock awaiting refresh). The checks listed above passed for the native MVP before that integration. The updater-owning thread must repeat aggregate checks after its files land; this record does not claim that its in-progress changes were validated by the native MVP pass.

At 17:58 IST, after updater files and the refreshed lockfile landed, all-target type checking and strict Clippy passed again. All 15 native-MVP targeted tests were repeated successfully against that dependency set. Updater integration was still editing UI formatting during the final format check, so its owner retains responsibility for the final whole-checkout formatting and updater/release checks. Native code was not built or launched.


## Native download and setup source checks - 21 October 2026

- `cargo test -p pieall --lib --locked`: 41 passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `pnpm exec vitest run web/editor-fields.test.ts web/menus.test.ts web/action-search.test.ts`: 17 passed.
- `pnpm run typecheck`, changed-file Prettier, Rust formatting and `git diff --check`: passed.
- Static React fixtures: `node scripts/preview-ui.cjs --tools` and `node scripts/preview-ui.cjs --shortcuts` passed. The compact setup fixture was rendered and visually inspected at 1000px; this is not native WebView acceptance. The initial collaborative browser attempt could not open file: URLs, so the static HTML preview was used. Remote shell text transport affected the middle-dot glyph in that preview; source UTF-8 text remains intact.
- Queried the fixed publisher release metadata to confirm the Windows FFmpeg GPL asset exists; no engine binary was installed or executed.

## Native downloads and automatic setup acceptance - 21 October 2026

Current code checks cover native URL source rejection, browser ring defaults/customization, release origin/digest validation, archive escape, cancelled setup, matching custom engines, transaction rollback, typed download options and safe publication. Tests do not exercise real browser input or engines. User-run acceptance after an executable is available:

1. Start with no configured engines. In a Chromium or Firefox window open an exact public video/post, hold the PieAll key and choose Download video. Verify it copies the address, restores the clipboard, installs tools with progress and saves the highest available video/audio streams. There is no extension to install. Repeat with Download audio.
2. Change focus or physical input during copying; no keys may reach a different app and no stale clipboard URL may be downloaded. Change the clipboard externally; PieAll must not overwrite it. After copying, changing tabs must not change the queued URL. A feed requires opening the exact post/video first.
3. Exercise Tool setup on a clean profile, failed network, cancelled transfer/extraction, malformed/digest-mismatched release fixtures and a custom media pair. Failure must keep existing records/files; retry is a new setup attempt. Setup network waits poll cancellation every 100 ms; metadata allows 30 seconds, archives 15 minutes overall with a 30-second read-stall timeout. No installed executable is run by setup itself.
4. In Browser downloads save quality/container/audio defaults and choose a destination. Reopen to verify persistence. Try pasted URLs, compatible/incompatible containers, output collisions, private IP/local DNS, denied folders, connection failure and oversized media. Inspect actual codecs/quality with FFprobe.
5. Confirm indeterminate progress without a known total and real byte progress when provided. Close suppresses later job notices; Cancel keeps completed outputs and cleans private staging. Worker termination retains the five-second grace period; setup drops cancellable network operations. Downloads never paste reports into the browser or open files automatically.
6. Check keyboard traversal, screen-reader names/status, compact Tool setup at narrow widths, toast hit testing, mixed DPI, packaged worker dispatch and Windows Downloads paths. Confirm no native messaging registration, plugin/config/PATH discovery or cookie import is needed.

No build, app launch or live engine installation was run by the agent. User-run development acceptance: `pnpm tauri dev`. Production build if needed: `pnpm run package:windows` with the required signing configuration.

## Quiet progress and installer timeout correction - 21 October 2026

The native notice corner has unit coverage for ordinary, negative-coordinate and small monitor work areas. Installer tests cover native TLS construction and cancellation of a pending network future. Actual native focus/minimize transitions, taskbar placement at mixed DPI and a full slow-network engine installation remain user-run acceptance. Verify foreground PieAll shows only footer progress/Cancel; switching away permits a fixed corner notice, moving the cursor never moves it, and returning to PieAll hides it without cancelling or permanently dismissing the job.
