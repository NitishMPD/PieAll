# Tasks — 25 October 2026
Author: Nitish

- Expanded Download video into Download media with grouped post images/GIFs/videos, managed gallery-dl setup, public-page recovery, bounded cancellable downloads, partial-result notes, updated docs/notices and passing Rust/web checks.
- Built and launched the updated local desktop app with pnpm tauri dev; the development server is running on port 1420.
- Fixed the progress-notification lock/window-dispatch deadlock, passed 14 desktop tests, and restarted the local app; Windows reports the new process as responsive.
- Fixed gallery-dl image-alternative output being mistaken for invalid URLs; six media regression tests pass and the updated local app restarted successfully.
- Fixed the release-blocking Clippy iterator and cloned-slice warnings; the exact strict workspace CI lint command, formatting checks and six media regression tests pass locally.
