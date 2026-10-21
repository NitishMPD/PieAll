# Native downloads and automatic tools implementation plan
Author: Nitish

Goal: Download the captured browser page with one ring action, with automatic dependencies and existing cancellable progress.

Architecture: The native platform validates the original HWND/PID and uses the bundled Copy URL shortcut. The action lease captures the URL before network work. A native installer owns fixed release sources, digest validation, bounded extraction and atomic engine records. Advanced manual selection remains available.

- [x] Remove extension assets, native messaging IPC and extension UI; retain settings compatibility.
- [x] Copy a fresh browser URL with focus checks and clipboard restoration; expose video/audio in browser rings.
- [x] Install missing FFmpeg/FFprobe, yt-dlp and Deno from fixed official release assets, checking publisher SHA-256, staging before publication and preserving custom engines.
- [x] Integrate setup into action progress/cancellation and a compact Tool setup view.
- [x] Verify with targeted Rust/frontend tests, type checking, Clippy and static rendering; update product/design/verification docs. No builds, launches, commits or subagents.
