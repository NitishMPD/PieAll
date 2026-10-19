# PieAll — native MVP

A Windows radial action utility with **44 implemented commands**, a searchable native action browser, focused image editors and isolated local workers. Built from [the product plan](PieAll-plan.md). This is an early implementation, not a verified release of the full 100-action product.

## Run it yourself

The implementation session did not build or launch the application, following the project instructions. From this folder:

```powershell
cargo build --release --locked
.\target\release\pieall.exe
```

Windows 11 x64 and the Rust MSVC toolchain/Windows SDK are required. The tray icon opens the action browser. Select files in Explorer and press **Ctrl + backtick**, or drop files onto the PieAll window, or choose **Open files…**. File arguments are accepted on the first launch. Use **Use clipboard text** explicitly to enable text tools. Close the dashboard to keep the tray running; choose **Exit** in the tray menu to quit.

The image ring offers Convert, Crop, Resize, Frame, Compress and More. Arrow keys/Tab select, Enter/Space runs, Escape cancels. The native list in the action browser is the accessible alternative to the custom ring. All image outputs go beside their source with unique names. Results show output paths and failures and have a Copy button.

## Implemented commands

- **14 image tools:** crop; resize within bounds without upscaling; PNG/JPEG/lossless WebP conversion; JPEG compression with size-reduction check; left/right rotation; horizontal/vertical flip; re-encode without source metadata; basic image details; quantized palette; decoded pixels to clipboard; solid-color frame with padding and rounded image corners; grayscale.
- **2 combined-image tools:** contact sheet (selection order, no labels) and images to PDF (A4, JPEG raster pages, white alpha background).
- **7 file/folder tools:** paths to clipboard, reveal, terminal, duplicate files, SHA-256, folder size, duplicate-content report. Folder scans skip links/inaccessible entries, are read-only, and have depth/count/time limits.
- **4 ZIP tools:** create from selected files, extract to a new folder, list, integrity test. Extraction rejects unsafe names/links/case collisions and enforces expansion quotas. Folder ZIP creation and merge extraction are not included.
- **13 text commands:** upper/lower/title case, trim lines, sort lines, deduplicate lines, count, JSON format/minify, URL-component encode/decode, Base64 encode/decode. Several commands are variants of one planned catalog action; 44 commands does not mean 44 of the plan's distinct 100 IDs.
- **4 window tools:** always-on-top toggle, center, fit work area, title/HWND details.

## Boundaries

The ring uses warm opaque GDI drawing with rounded separated wedges. It is not the final glass/Direct2D renderer. Image previews approximate results and show only the first selected image; transforms apply to every selected image and report each item. Crop uses numeric coordinates, not draggable handles. Frame currently has solid color/padding/corners, without gradient/shadow/ratio controls. Image detail inspection does not enumerate EXIF. Metadata stripping re-encodes pixels; it is not a claim of comprehensive sanitization.

There is no browser extension, optional engine integration, collection database, settings persistence, public hub, signing or installer. The shortcut is fixed and DPI is system-wide. Desktop/Explorer-tab capture needs runtime verification and fails closed on ambiguity. No job cancel control yet; worker jobs time out at 120 seconds, and any outputs already published remain. A hard stop during a job can leave a hidden staging file/folder. Hard-link publication requires filesystem support; it fails rather than falling back to overwriting. Second launches do not forward file arguments.

Screen-reader behavior, focus restoration, protected apps, Explorer tabs, mixed DPI, previews, memory, latency and release size have not been tested in a live app session. See [verification.md](verification.md).
