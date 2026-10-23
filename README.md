# PieAll — Tauri desktop

<img src="public/brand/pieall.svg" width="80" height="80" alt="PieAll radial mark" />

A Windows radial action utility with **68 local commands and 3,108 app shortcuts**, a searchable action browser, focused image editors, browser video/audio downloads and isolated local workers. Built from [the product plan](PieAll-plan.md). This is an early implementation; native app acceptance remains unverified.

## Run it yourself

The default application is now Tauri 2 + React/TypeScript with Rust-owned commands. The GDI shell is retained as an explicit legacy fallback. See [the port guide](docs/tauri-port.md) for architecture, signing setup and native acceptance gates.

Use Windows 11 x64, Rust MSVC 1.92+, Visual Studio C++ Build Tools/Windows SDK, Node 22.12+ and pnpm. The implementation task did not build or launch the application. Run yourself:

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

To build the executable without installer signing: `pnpm tauri build --no-bundle`. For a signed per-user NSIS installer, configure the PieAll signing variables described in the port guide, then run `pnpm package:windows`. The installer uses the system WebView2 runtime; it does not bundle a fixed browser. Smaller artifacts remain an unmeasured goal.

The mark is a graphite tile with off-white segments and one orange selected segment; every launch shows a short splash over the desktop: a mini ring swipes through its feature slices and lands on "PieAll". Regenerate the icons with `python scripts/generate-brand.py`.

Select files in Explorer and press **Ctrl + backtick**, drop files onto PieAll, or choose **Open files**. **Use clipboard text** is explicit. Closing the dashboard hides it; tray Exit quits when no action/editor/update is active. File arguments work on first and second launches. The image ring offers Convert, Crop, Resize, Add background, Compress and All actions. Convert opens PNG/JPEG/WebP choices. Arrows/Tab, Enter/Space, number keys and Escape work in the floating ring; the searchable dashboard is the alternative keyboard route. **Customize ring** sets the hold key, edits ring layouts per context and has a **Start PieAll with Windows** option (per-user Run key, no admin rights). The tray's **Use shortcut** toggle pauses the hold key and Ctrl + backtick for the current session.

## Implemented commands

The default Tauri shell now also includes **3,190 contextual app shortcuts across 58 profiles**, for **3,258 entries** with the 68 local commands below. Shared browser/editor keymaps are counted separately for each matching app. Hold backtick in a supported app for six direct quick choices and Search actions, or press `/` from the ring to search a flat list of that app's shortcuts in the dashboard. Other apps' shortcuts are excluded. Selected text in a recognized app keeps app actions primary with Text tools secondary; files/images retain their selection tools. Browsers, including Dia, include Copy URL and private browsing. Figma adds an Align group alongside six editing actions, with search/window access in More. Customize ring also offers Add app: choose an executable, name the app and add its keyboard shortcuts; custom profiles can be edited or removed there. Existing customized rings are preserved; Reset adopts the new defaults. Explorer, everyday browsers, Office, chat/notes/media tools, Figma, Photoshop, Adobe video tools, Blender and specialist editors are covered. Native matching uses the captured window/process and exact executable names; Figma running in a browser currently receives browser shortcuts. App commands use Windows US defaults and depend on the target's editing mode/keymap. Dispatch confirms “shortcut sent,” not app-side completion. See [project behavior](project.md) for ownership and limits.

- **14 image tools:** crop; resize within bounds without upscaling; PNG/JPEG/lossless WebP conversion; JPEG compression with size-reduction check; left/right rotation; horizontal/vertical flip; re-encode without source metadata; basic image details; quantized palette; decoded pixels to clipboard; solid/gradient frame with padding, canvas ratios, rounded corners and shadows; grayscale.
- **2 combined-image tools:** contact sheet (selection order, no labels) and images to PDF (A4/Letter/image-sized pages, orientation/margins/quality, white alpha background).
- **7 file/folder tools:** paths to clipboard, reveal, terminal, duplicate files, SHA-256, folder size, duplicate-content report. Folder scans skip links/inaccessible entries, are read-only, and have depth/count/time limits.
- **4 ZIP tools:** create from selected files and folders (nested and empty directories included) with Store or Deflate 0–9, extract to a new folder, list, integrity test. Creation limits depth, file count and source size; extraction rejects unsafe names/links/case collisions and enforces expansion quotas. Merge extraction is not included.
- **7 PDF commands:** details, page extraction/splitting/rotation, stream compression, document metadata removal and selectable-text export. Rewriting refuses forms/signed PDFs; no OCR or secure redaction.
- **15 media commands:** video conversion/compression/resize/rotate/mute/trim/GIF/frame/audio export; audio conversion/trim/compression/normalization/volume; media inspection. FFmpeg and FFprobe install automatically on first use (or pick your own under Tool engines → Advanced). Actual formats depend on the codecs available; native media acceptance remains user-run.
- **2 browser download commands:** Download video and Download audio, from the browser ring (Chromium and Firefox) or a pasted HTTP(S) link in the **Browser downloads** sidebar. Video defaults to the best available quality merged into MKV; audio keeps its original codec or converts to MP3/M4A/Opus/FLAC/WAV. Quality cap, container, audio format and destination (default: Downloads) are saved defaults. yt-dlp, Deno and FFmpeg set themselves up on first use from pinned GitHub releases with SHA-256 checks. No cookies/login, DRM, live streams, batch links or full playlists. Open the exact video/post page, not a feed.
- **13 text commands:** upper/lower/title case, trim lines, sort lines, deduplicate lines, count, JSON format/minify, URL-component encode/decode, Base64 encode/decode. Several commands are variants of one planned catalog action; 68 commands does not mean 68 of the plan's distinct 100 IDs.
- **4 window tools:** always-on-top toggle, center, fit work area, title/HWND details.

## Boundaries

The Tauri shell passes workspace all-target Rust checking, strict Clippy, TypeScript and frontend/release-tool source checks. The app and installer have not been launched for runtime acceptance. Native focus, Explorer capture, screen readers, transparency, mixed DPI, previews and update installation remain unverified. See [verification.md](verification.md).

The dashboard uses neutral graphite/white tokens (light and dark) with an orange `#FF8A3D` selection; the floating ring is a dark glass tray with pillow-shaped slices and local stroke icons. There is no native acrylic/Mica blur. Only the first selected image is previewed; actions still validate and apply to all selected targets. Crop has drag selection, aspect presets and numeric fields. Frame has solid/gradient backgrounds, canvas ratios, padding, corners and shadows. Metadata stripping re-encodes pixels rather than claiming complete sanitization. Media and download engines install app-locally on first use (Windows x64), with custom paths and pinned hashes under Advanced; engine records use local SQLite. Downloads run through a per-job loopback proxy that only allows public HTTP(S) addresses on standard ports; it is a network guard, not an OS sandbox. A job allows one hour and 2 GiB of engine output, and files never open automatically. There is no browser extension, collection/board database UI or public hub.

Local image/PDF/file workers time out at 120 seconds; media allows one hour. Native job progress, Close and Cancel controls are implemented. Previously published outputs remain after a timeout. Abrupt termination may leave staging files. Hard-link publication requires filesystem support and fails rather than overwriting. Source-preserving output and ZIP guards remain in the original Rust core.

The new shell uses Tauri's signed updater and NSIS configuration. Checks are release-only with a configured dedicated PieAll public key. Download and install are separate explicit actions. The draft-first GitHub release workflow, versioning and signed-artifact verification are implemented; see [release setup](docs/releases.md). Dedicated production signing keys, optional Authenticode and clean-Windows install/upgrade acceptance remain external setup/verification steps. The previous custom-updater handoff is historical; use the Tauri protocol documented in the port guide.
