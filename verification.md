# Verification record
Author: Nitish

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
