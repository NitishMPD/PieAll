# PieAll — Tauri desktop

A Windows radial action utility with **44 implemented commands**, a searchable action browser, focused image editors and isolated local workers. Built from [the product plan](PieAll-plan.md). This is an early implementation, not a verified release of the full 100-action product.

## Run it yourself

The default application is now Tauri 2 + React/TypeScript with Rust-owned commands. The GDI shell is retained as an explicit legacy fallback. See [the port guide](docs/tauri-port.md) for architecture, signing setup and native acceptance gates.

Use Windows 11 x64, Rust MSVC 1.92+, Visual Studio C++ Build Tools/Windows SDK, Node 22.12+ and pnpm. The implementation task did not build or launch the application. Run yourself:

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

To build the executable without installer signing: `pnpm tauri build --no-bundle`. For a signed per-user NSIS installer, configure the PieAll signing variables described in the port guide, then run `pnpm package:windows`. The installer uses the system WebView2 runtime; it does not bundle a fixed browser. Smaller artifacts remain an unmeasured goal.

Select files in Explorer and press **Ctrl + backtick**, drop files onto PieAll, or choose **Open files**. **Use clipboard text** is explicit. Closing the dashboard hides it; tray Exit quits when no action/editor/update is active. File arguments work on first and second launches. The image ring offers Convert, Crop, Resize, Add background, Compress and All actions. Convert opens PNG/JPEG/WebP choices. Arrows/Tab, Enter/Space, number keys and Escape work in the floating ring; the searchable dashboard is the alternative keyboard route.

## Implemented commands

- **14 image tools:** crop; resize within bounds without upscaling; PNG/JPEG/lossless WebP conversion; JPEG compression with size-reduction check; left/right rotation; horizontal/vertical flip; re-encode without source metadata; basic image details; quantized palette; decoded pixels to clipboard; solid-color frame with padding and rounded image corners; grayscale.
- **2 combined-image tools:** contact sheet (selection order, no labels) and images to PDF (A4, JPEG raster pages, white alpha background).
- **7 file/folder tools:** paths to clipboard, reveal, terminal, duplicate files, SHA-256, folder size, duplicate-content report. Folder scans skip links/inaccessible entries, are read-only, and have depth/count/time limits.
- **4 ZIP tools:** create from selected files, extract to a new folder, list, integrity test. Extraction rejects unsafe names/links/case collisions and enforces expansion quotas. Folder ZIP creation and merge extraction are not included.
- **13 text commands:** upper/lower/title case, trim lines, sort lines, deduplicate lines, count, JSON format/minify, URL-component encode/decode, Base64 encode/decode. Several commands are variants of one planned catalog action; 44 commands does not mean 44 of the plan's distinct 100 IDs.
- **4 window tools:** always-on-top toggle, center, fit work area, title/HWND details.

## Boundaries

The Tauri shell passes workspace all-target Rust checking, strict Clippy, TypeScript and frontend/release-tool source checks. The app and installer have not been launched for runtime acceptance. Native focus, Explorer capture, screen readers, transparency, mixed DPI, previews and update installation remain unverified. See [verification.md](verification.md).

The UI uses warm ivory/amber rounded SVG wedges, local stroke icons and translucent fills. It does not implement native acrylic/glass blur. Only the first selected image is previewed; actions still validate and apply to all selected targets. Crop uses numeric fields. Frame has solid color/padding/corners. Metadata stripping re-encodes pixels rather than claiming complete sanitization. There is no browser extension, optional engine integration, persistent collection database, configurable shortcut or public hub.

Workers time out at 120 seconds; there is no job-cancel control. Previously published outputs remain after a timeout. Abrupt termination may leave staging files. Hard-link publication requires filesystem support and fails rather than overwriting. Source-preserving output and ZIP guards remain in the original Rust core.

The new shell uses Tauri's signed updater and NSIS configuration. Checks are release-only with a configured dedicated PieAll public key. Download and install are separate explicit actions. The draft-first GitHub release workflow, versioning and signed-artifact verification are implemented; see [release setup](docs/releases.md). Dedicated production signing keys, optional Authenticode and clean-Windows install/upgrade acceptance remain external setup/verification steps. The previous custom-updater handoff is historical; use the Tauri protocol documented in the port guide.
