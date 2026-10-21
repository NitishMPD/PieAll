- Branded PieAll with a shared radial mark across dashboard, connection screen, favicon, README and Windows app/installer/tray assets; added a reproducible asset generator and documented the identity.
- Redesigned the floating ring as a glass tray with pillow slices (parallel gaps, rounded corners, denser fills, solid selected slice), removed the pale crescent above the top slice, made releasing ` always close the ring, and fixed the ring refusing to reopen after a slice opened the dashboard editor.
- Releasing ` now closes the ring natively as a backstop, 400 ms after release once selection capture ends, so a stalled web round-trip or failed run can't leave the ring open.
- Releasing ` now runs the highlighted action slice, falling back to the cursor's direction only when nothing is highlighted.
- Fixed the hold-to-open races: a late focus-loss event from the previous ring no longer closes a new one, a closed ring can't be re-shown by a pending focus call, focus loss is ignored while ` is held, and release always hides the window. Also centered the ring's clip region on the client area (top crescent), moved the rest-to-open line into the tray gutter so labels never overlap it, shrank labels to 12 px, and gave app shortcuts clicked mid-hold 5 s for the key to come up.
- Added 3,079 app-scoped shortcuts across 57 profiles, six curated quick actions per app, native process/focus/revision guards, held-key and partial-input handling, selection invalidation and Ctrl+backtick collision handling; documented default-keymap and native acceptance limits.
- Made the large action library searchable by app/category/chord with 60-row pagination and cached indexes; fixed nested menu truncation, bounded long labels and short-window scrolling. Passed TypeScript, strict Clippy, changed-file formatting and 23 relevant unit tests; statically rendered the UI and measured search at about 0.16 ms per query in the Node fixture.

- Fixed shortcut context leakage by using captured-session actions in both views, removed the All apps selector and nested app shortcut trees, added direct Search actions and `/` access to flat contextual search, and reset search on app switches. Passed 14 frontend and 5 native shell tests, typecheck, strict desktop Clippy, formatting and the static contextual fixture.
- Made PieAll open on key-down everywhere (AutoHotPie-speed): quick taps still type ` with the ring vanishing instantly, text capture waits for a confirmed hold, shortcut-only apps confirm immediately, the ring never takes focus while held (hook handles Escape), native-only window placement and release reading, and a 70 ms entrance with no stagger.
- Turned off the Windows 11 accent border on the ring and toast windows; clicking a slice activated the ring and the border showed as a pale blue arc above the top slice.
- Redesigned dashboard navigation and empty state; added Customize ring (hold-key choice, per file type and per app ring layouts).
- Rebuilt Customize ring as a fixed three-pane editor (rail with active app and app search, full ring preview, slice list and add picker); removed filler copy and made the dashboard non-scrolling.
- Added a native Use shortcut tray toggle to pause hold-key and Ctrl+backtick invocation for the current session, preserve manual ring access, and guard hotkey restoration; verified disabled-key passthrough and active-hold release with a focused Rust unit test.
- Replaced the hand-drawn icon set with Lucide plus a keyword resolver that gives app commands relevant icons (782 of 870 shortcut titles matched, generic fallback for the rest).
- The ring no longer flashes the previous target's slices while reading the selection: it shows the empty tray and "Reading…" hub, then the real slices fade in once.
- Added a frameless dashboard with a custom title bar (drag region, minimize/maximize/close to tray).
- Expanded the product plan with all 72 extracted toolkit workflows, Windows adaptations, current implementation gaps, moodboards, open-source engine candidates and phased acceptance gates; synchronized planned scope and UX in project.md/design.md without changing runtime code.
- Specified contextual browser video/audio downloads with yt-dlp, best-available quality and app-configurable defaults; added direct web-image save/copy/board flows, target disambiguation, background-job behavior and acceptance requirements to the plan and product/design docs.

- Updated 49 default app profiles for repetitive workflows; added browser Copy URL, Figma Align/distribute and app-first selected-text routing while preserving saved layouts and action IDs. Updated research/product/design/verification docs; passed 17 frontend and 3 catalog tests, TypeScript/Rust checks and changed-file formatting. Native target-app acceptance remains user-run.
- Fixed false "PieAll is busy" after closing the dashboard with an editor open (closing now abandons the editor), stopped toast text clipping (detail wraps under the title) and redrew the submenu dwell progress as a dark bar with a track inside the popped slice.
- Fixed Windows CI formatting failures with a repository LF checkout rule; confirmed the existing source needs no formatting changes and `pnpm format:check` passes.
- Built the Windows x64 NSIS installer (PieAll 0.1.0, 1.89 MiB) with a temporary local override disabling updater artifacts; production signing configuration unchanged. Frontend type-check/build and Rust release packaging passed. Installer is unsigned and was not launched.
- Removed the ring loader: while the selection is read, the ring shows the slices it settled on last time for the same app, so it is usable on its first frame.

- Added native worker progress, Close/Cancel controls, cancellation-safe publication and Windows process-tree/memory ownership; extended image crop/resize/frame and image-to-PDF settings.
- Added seven bounded PDF commands, 15 optional FFmpeg/FFprobe media commands, native engine selection with SQLite/hash records, contextual quick tools and folder ZIP creation with compression controls; preserved originals and refreshed open-source notices.
- Verified tool expansion with 29 Rust and 14 targeted frontend tests, workspace checking/strict Clippy, TypeScript, static rendering and formatting; documented runtime acceptance limits and remaining planned work.
- Result toasts are quick again: finished actions show a plain notice (no progress bar, "complete" or close button) for 1.4 s, errors 3.2 s; progress shows only while running. The toast is near-opaque so it reads on bright wallpaper.
- Fixed "Open terminal" reporting success with no window: cmd.exe now starts in its own console.
- Smarter file ring: a PDF-only (or media-only) selection gets its tools as direct slices instead of a "PDF tools" submenu, and Copy SHA-256 left the default file ring.
- One-shot actions (Terminal here, Copy path, Duplicate) never show the progress card or spinner, only the short result notice; progress UI is reserved for multi-file or progress-reporting work.
- The pin slice now reads "Unpin" when the captured window is already always-on-top (checked live each time the ring opens).
- Releasing ` on an app shortcut or window slice no longer waits for the selection read: the read is cancelled (clipboard restored) and the action runs at once.
- Fixed Terminal here opening a blank, dead window and "Invalid worker response": launchers now run outside the sandboxed worker (whose job killed the terminal), and cmd.exe gets its own console without inheriting PieAll's stdio.

- Added video/audio URL downloads with pinned yt-dlp/optional Deno, persistent quality/container/audio/folder settings, public-address proxy checks, shared bounded engine execution and cancellation-safe publication.
- Added an exportable Chromium extension and explicit Chrome/Edge/Brave native-host setup/removal, bounded origin/time/identity/replay validation and background single-instance job handoff.
- Added Browser downloads/settings UI and honest indeterminate progress; verified 40 Rust tests, one extension behavior test, 12 targeted frontend tests, strict workspace Clippy, TypeScript and static rendering. Refreshed notices and documented untested native/site acceptance and remaining browser/board work.

- Replaced extension/native-host downloads with checked native browser URL copying and customizable Download video/audio ring defaults; retained pasted links and saved quality/folder settings.
- Added verified first-use FFmpeg/FFprobe, yt-dlp and Deno setup with cancellable progress, safe extraction, atomic records and custom-path preservation; replaced engine cards with compact Tool setup and Advanced controls. Verified 41 Rust and 17 frontend tests, TypeScript, strict workspace Clippy and static setup rendering; live installs/browser downloads remain untested.
- Terminal here really opens now: dropped SEE_MASK_NO_CONSOLE, which had attached cmd to PieAll's hidden console instead of giving it a window.

- Fixed automatic tool setup panicking on HTTPS by explicitly selecting the enabled native TLS provider; applied the same fix to the legacy updater and added a setup-client regression test.

- Moved notices from the cursor to a fixed bottom-right work-area corner and kept foreground job progress in the dashboard footer; added monitor-coordinate coverage.
- Fixed large engine downloads timing out after 20 seconds by reusing native-TLS reqwest/Tokio with separate overall/stall limits and cancellable awaits; five installer tests, the notice-placement test and strict workspace Clippy passed.
- Progress toasts: hug the bottom-right corner, no longer stutter (window is not moved/resized on every tick; bar eases between updates), and read Title / status with percent / bar + Cancel.

- Made the app icon color-neutral (graphite tile, off-white and gray segments) and added a Captrain-style startup splash to the dashboard.
- Fixed release workflow failing at "Create or resume release draft": drafts are not returned by the tags endpoint, so the script now finds them via the releases list; updated README and release docs.
- Neutralized the dashboard theme (tokens and hard-coded warm colors), made the selected slice orange, and tightened the empty-state steps, icons and copy.
- Moved the startup splash into its own transparent, click-through, always-on-top window (like Captrain) so it plays over the desktop on every launch, even with the dashboard hidden.
- Replaced the logo-trace splash with a mini ring that swipes through the feature slices (center names each) and lands on "PieAll"; dropped the light sweep.
- Restyled the splash ring in the logo colors (graphite slices, copper selection), stripped the tray, strokes and shadows, and fixed icons rendering filled because the slice fill rule also hit icon paths.
- Fixed Windows downloader proxy connection resets by restoring blocking mode on accepted sockets; added a delayed-header regression and verified YouTube metadata extraction through the proxy.
- Ran Prettier on web/dashboard.css, web/Splash.tsx and web/browser-tools.css so the release workflow's format:check passes.
- Ran every release-workflow check locally; fixed cargo fmt in tests/download_diagnostic.rs. All checks pass.
- Added a 'Start PieAll with Windows' checkbox in Customize, backed by the per-user Run registry key (new autostart module and two dashboard-only commands).
- Added dark mode to the dashboard window: colors now come from tokens in web/tokens.css, with a prefers-color-scheme: dark override.

- Added Open location to successful download toasts, revealing the native job output in Explorer; kept completion notices visible for eight seconds with Close, including over the active dashboard. Seven native shell tests, TypeScript and changed-file formatting passed; native UI/Explorer interaction remains unverified.
- Fixed output scans failing when downloader fragments disappear during renames, and separated inspection errors from genuine 2 GiB limit errors; both targeted regression tests passed.
- Fixed release draft creation failing on a stale GitHub release list by validating the creation response directly; added stale-list, resume and ownership regression tests to local/CI release checks. All 10 release tests passed.
- Updated README for 68 commands, browser downloads, automatic engine setup, folder ZIPs, Start with Windows, the tray shortcut toggle and the current graphite/orange look.
- Fixed the release build failing with "Missing comment in public key": package-release.ps1 now passes PIEALL_TAURI_PUBLIC_KEY to tauri build as a config override. Added .taurignore so doc/task/web edits no longer restart `pnpm tauri dev`.
- Tripled ZIP input/expanded-size limits to 1536 MiB, per-file expansion to 768 MiB and expansion ratio to 3000:1; updated product documentation and passed six focused ZIP/archive tests.
- Fixed missing unzip progress by reporting expanded bytes and current entries through the existing job UI, preserving batch counts and checking cancellation every 64 KiB; passed six focused tests including chunked and empty archive extraction.
- Fixed false held-key errors blocking browser downloads and app shortcuts by passing confirmed hold-key releases through to Windows; kept repeat suppression and input safety checks. Two hook regression tests, the invalid-source shortcut test and changed-file formatting passed; native download acceptance remains unverified.
- Fixed fast-release selection capture racing Copy URL/download actions: reserve capture before worker scheduling and reject cancelled reads before Ctrl+C. Nine desktop tests, two shortcut/cancellation tests and changed-file formatting passed; browser timing remains unverified.
- Fixed the download toast: it said "Saved 2 files", lacked Open location and closed after 1.4 s, because the report put a bare file name on its "Saved" line and repeated "Saved" in its format line. Now one absolute-path "Saved" line; the toast names the file, offers Open location and stays 8 s.
- Fixed Explorer falling back to Documents for download filenames with spaces by separating the selection switch from the saved path; covered custom folders, commas, Unicode and directory targets with a passing regression test. Workspace formatting passed; native Explorer interaction remains unverified.
- Enabled concurrent Tauri actions/downloads with independent job progress, cancellation, retained results and output locations; preserved short input coordination, serialized shared tool setup and allowed updater checks/downloads alongside work. Updated project/design docs; 29 UI tests, 11 native shell tests, TypeScript, strict Clippy and formatting passed. Native overlapping-download/focus acceptance remains user-run.
