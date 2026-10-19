# PieAll
Author: Nitish

PieAll is a Windows 11 x64 local radial action utility. The full product vision remains in PieAll-plan.md. This first implementation prioritizes a real native invocation surface and working local image, file, ZIP, text and window tools. Browser integration, engines, collections and public publishing remain future milestones, not enabled actions.

## Runtime and ownership
The UI captures an explicit immutable selection before opening. Explicit dropped/opened files replace the prior context. Clipboard text requires an explicit user gesture. The registry owns all-target eligibility; handlers validate again. Expensive handlers execute in a temporary child process so codecs do not stay resident. Native UI owns presentation and source focus.

Outputs preserve original files, stage beside the destination and publish using atomic no-replace hard-link creation. Symlink/reparse inputs and destination parents are rejected; output names are controlled by the application. ZIP extraction uses a new private staging directory, validates paths, rejects links and applies quotas before publishing a new sibling folder. No existing directory is merged. Input byte/pixel/count limits protect workers. No network requests or analytics are implemented.

## First-slice boundaries
Native Win32 and opaque warm GDI rendering are a provisional implementation; Direct2D/translucency, full custom UI Automation, modifier-drag OLE support, automatic active-tab browser capture, persistent configuration/SQLite, cancellation and production packaging have not passed their planned gates. Explorer selection fails closed when foreground tab identity is ambiguous. Explicit file opening/dropping is the reliable fallback. No claims of measured performance or full 100-action coverage.

## Verification
Only formatting, type checks and targeted tests are permitted in this session. Builds, app launches and UI/runtime verification are user-run. Never infer native accessibility, focus, mixed-DPI correctness or latency from static checks.

## Implemented first-slice catalog
There are 44 implemented commands in the registry. Text formatting/case/encoding variants are separate commands, so this is not 44 distinct IDs from the 100-action product catalog. The exact current behavior, limits, run commands and remaining gates are in README.md and verification.md. Explorer capture runs in a disposable 1.2-second helper; jobs run in separate processes with a 120-second deadline and one job at a time. Results are capped at 512 KiB. Image previews normalize orientation and decode with allocation limits before creating a small thumbnail.
