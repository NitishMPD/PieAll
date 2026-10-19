# Windows releases implementation plan
Author: Nitish

Goal: replicate Captrain's tray/update/download/GitHub release lifecycle within PieAll's existing Win32 Rust architecture.

Keep the native tray, single-instance mutex, action workers and close-to-hide behavior. Add one native update control and tray command; HTTPS work stays on a worker, UI state stays on the message thread. Release builds alone check after 10 seconds/every six hours. Explicit download verifies signatures; explicit installation refuses active actions/capture/editing. Development does not fetch updates.

1. Add `src/updates.rs`: bounded HTTPS, signed manifest, stable newer-version and asset validation, streamed signed installer download, isolated temporary staging and installer launch. Add targeted validation/signature tests.
2. Extend `src/ui.rs`: update state/results/progress, timer, dashboard and tray controls, busy-safe exit/install. Preserve current uncommitted UI changes.
3. Add `installer/pieall.nsi`, release/version scripts and `.github/workflows/release.yml`: per-user x64 installer, portable ZIP, synchronized Cargo versions, signed assets, draft validation before publish. Add script checks.
4. Update project/design/README and release setup instructions; record completed work in the daily task file. Run formatting, all-target type checks, focused tests and script static checks. No builds, launches, branches, commits, pushes or deployments.

External prerequisites: configure the GitHub repository, repository public-key variable and signing-key secret. Never reuse Captrain's private key or copy it into this checkout. Installer/runtime verification is user-run.
