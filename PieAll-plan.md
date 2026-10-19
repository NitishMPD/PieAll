# PieAll implementation plan
Author: Nitish

Date: 19 October 2026  
Status: proposed implementation plan. No application has been implemented or benchmarked.

**Goal:** Build a fast Windows radial action menu that understands the selected item, opens with a configurable backtick shortcut, and lets people finish useful tasks without opening a large dashboard.

**Architecture:** A resident Rust Windows application owns invocation, context, the pie, and a small dashboard. Expensive operations run in temporary worker processes. A browser extension supplies browser context. A separate community hub distributes reviewed declarative packs that invoke the same built-in handlers.

**Tech stack:** Rust stable, windows-rs, Win32, Direct2D/DirectWrite, optional DirectComposition, Windows Imaging Component, SQLite, Serde, and narrowly enabled image/archive libraries. Chromium Manifest V3 extension in TypeScript. A separate Rust HTTP service and simple web pages for the hub.

**Execution:** Implement sequentially in the existing folder. Do not create branches, commit, push, deploy, start a dev server, run a build, or launch the app under the current working instructions. Checks and targeted tests are allowed. No delegated agents are required.

## 1. Product decisions

- Windows 11 x64 first. Record the tested Windows build in each release. ARM64 and Windows 10 require separate compatibility work.
- The pie is the main surface. The dashboard handles configuration, action discovery, recent jobs, and collections.
- Use warm translucent surfaces, separate rounded wedges, fine light borders, orange selection, and compact floating editors.
- Support keyboard invocation on the current selection and modifier-plus-file-drag invocation. Both use one registry and execution path.
- Target 100 distinct actions for v1.0. Some require an optional engine or browser extension. State that distinction in onboarding and marketing.
- Start with a complete 20-action version, then add actions in tested groups.
- Default to local processing. The hub receives pack submissions, not users' files, clipboard contents, browsing history, or moodboards.
- Local actions and browsing packs require no account. Publishing public packs requires an account.
- Use explicit selection and deterministic matching. No AI model belongs in the menu's invocation path.

## 2. Visual and interaction reference

Carry these design details into PieAll:

1. Separate rounded wedges with small gaps and a thin outer rim.
2. Warm translucent fill and fine light borders.
3. Orange selection with a clear icon and short label.
4. A quiet center identifying the target and supporting cancel/back.
5. A second level for format choices with a visible way back.
6. Focused floating editors instead of a full application workspace.
7. Results beside the source, preserving originals.
8. Small progress UI that does not force the dashboard open.

## 3. Behavior and invariants

### Context priority

1. Files explicitly dropped onto PieAll.
2. Files/folders selected in foreground Explorer or the desktop.
3. Explicit browser image/link/text selection supplied by the extension.
4. Current webpage supplied by the extension.
5. Readable, non-password text selection through UI Automation in supported apps.
6. Active-window actions, with an explicit Use clipboard option.

Capture context before changing foreground focus. Never silently send Ctrl+C or replace the clipboard to discover selection. UI Automation coverage is incomplete; unsupported applications get a clear fallback.

Each invocation creates an immutable snapshot: invocation ID, source HWND/process ID, provider, capture time, target descriptors, and capabilities. Browser snapshots also include connection/profile, browser window/tab/frame identity, origin, and capture time. Discard replies for closed or superseded invocations.

The center names the target: photo.jpg, 4 images, Folder: Research, Current webpage, or Active window. While resolving, target-dependent actions cannot execute. On timeout offer retry, explicit drop, or active-window actions. Never run against a stale selection.

### Shortcut and drag

- Offer configurable backtick bindings, including modifiers and physical-key behavior. Test keyboard layouts, dead keys, IME, repeats, and ordinary typing.
- Prefer RegisterHotKey for supported bindings. Registration failure keeps tray/drag access working and offers another shortcut.
- Do not consume ordinary backtick typing by default. Offer plain-backtick mode explicitly; use a modified backtick binding initially and explain the distinction during setup.
- If hooks are needed for plain-key/drag support, callbacks only record input and post work. No COM, I/O, decoding, or rendering in hooks.
- Honor excluded apps, screen lock, secure desktop restrictions, suspend/resume, and composition state.
- Click invocation captures first, then focuses the pie; deliberate click or keyboard activation executes. Closing restores valid source focus.
- Drag invocation remains non-activating and uses the Windows drag threshold and OLE file drops. Always report copy semantics. Hover never executes.
- Hold-and-release marking gestures can wait; they are not necessary for the initial complete workflow.

### Action contract

Every action has a stable ID, title, accepted targets, cardinality, typed parameters, required capability/engine/permission, handler ID, output policy, and side-effect class. The registry owns eligibility; workers own execution validation; UI owns presentation.

Availability states: ready, needs input, needs engine, needs permission, unsupported. Evaluate cheap metadata predicates to draw the pie. Do not decode images, inspect entire archives, or traverse folders there.

For a batch, default to actions supported by every target. A subset action must say Apply to 3 of 5 and identify skipped targets before execution. Combining actions define explicit ordering. Opening a pie must not rearrange slots halfway through an invocation.

## 4. Launch catalog: exactly 100 actions

These are user-visible actions, not 100 services. Conversions share a converter panel and handler; image transforms share a pipeline; all output passes through one safety boundary.

Availability labels:

- **Core:** base install, using Windows APIs or small bundled Rust dependencies.
- **Engine:** explicit optional local engine/model download.
- **Browser:** Chromium extension, with feature-specific permissions where needed.
- **Core + Browser:** works locally; browser inputs require the extension.
- **Browser + Engine:** requires both.

Transformations preserve originals. Network operations start after user activation. Mutation actions have a preview/conflict step where needed. An action becomes ready only after checking format support, dependencies, and permissions.

### Images: 01–20

| ID | Action | Availability | Behavior and boundary |
|---|---|---|---|
| 01 | Crop image | Core | Floating preview, aspect presets, numeric coordinates, keyboard handles; save a new image |
| 02 | Resize image | Core | Dimensions/percent, aspect lock, optional no-upscale; preview dimensions |
| 03 | Convert image format | Core | PNG/JPEG/WebP picker; explicit alpha flattening color for JPEG |
| 04 | Compress image | Core | Quality and size preview; report when encoding cannot reduce size |
| 05 | Rotate left | Core | 90-degree counterclockwise copy after orientation normalization |
| 06 | Rotate right | Core | 90-degree clockwise copy |
| 07 | Flip horizontally | Core | Mirrored copy |
| 08 | Flip vertically | Core | Vertically flipped copy |
| 09 | Strip image metadata | Core | New image with documented metadata policy; no claim of perfect sanitization |
| 10 | Inspect image details | Core | Detected format, dimensions, size, orientation, readable metadata |
| 11 | Copy image pixels | Core | Decoded bitmap on clipboard, distinct from copying the file |
| 12 | Extract color palette | Core | Dominant swatches and copyable HEX from a downsampled image |
| 13 | Add framed background | Core | Video-style panel: solid/gradient, padding, corners, shadow, ratio |
| 14 | Add watermark | Core | Text/local-image overlay with opacity and placement preview |
| 15 | Grayscale image | Core | Preview and save grayscale copy |
| 16 | Remove background | Engine: ONNX runtime/model | Local inference; show download, model license, RAM estimate, preview |
| 17 | Extract image text | Core | Windows OCR with an installed supported language; explain missing-language setup |
| 18 | SVG to raster | Core | Safe PNG/WebP render; disable scripts and external resources |
| 19 | Create contact sheet | Core | Ordered multi-image grid with thumbnail size and labels |
| 20 | Compare images | Core | Side-by-side/wipe view for exactly two readable images |

### Files and folders: 21–30

| ID | Action | Availability | Behavior and boundary |
|---|---|---|---|
| 21 | Copy paths | Core | One per line, explicit quoted/unquoted choice |
| 22 | Reveal in Explorer | Core | Select target in parent; folders can open directly |
| 23 | Open terminal here | Core | Typed working-directory argument; no interpolated shell command |
| 24 | Rename with preview | Core | Single/batch template, replacement, numbering; validate case-only and duplicate destinations |
| 25 | Duplicate selection | Core | Collision-safe sibling copies via shell file operations |
| 26 | Move to folder | Core | Destination, preview, conflict policy; supported cross-volume shell behavior |
| 27 | Copy to folder | Core | Destination picker and explicit conflicts |
| 28 | Calculate folder size | Core | Cancellable counts; report inaccessible items; do not follow reparse points by default |
| 29 | Calculate SHA-256 | Core | Streaming file checksum and copyable report; cancellable |
| 30 | Find duplicate files | Core | Selected-folder scope, size then hash; list-only results |

### Archives: 31–40

| ID | Action | Availability | Behavior and boundary |
|---|---|---|---|
| 31 | Create ZIP | Core | Selected files/folders, stable relative paths, bounded traversal |
| 32 | Extract to named folder | Core | ZIP; collision-safe sibling destination |
| 33 | Extract here | Core | ZIP; preview entry count/conflict policy before merging |
| 34 | List archive contents | Core | ZIP entries/sizes without extraction; escape unsafe names in UI |
| 35 | Extract selected entries | Core | ZIP picker; validate each selected path |
| 36 | Test archive integrity | Core | ZIP stream/CRC check with itemized failures |
| 37 | Create encrypted 7z | Engine: 7-Zip | Local password field, never persisted/logged; disclose selected tool's process-argument exposure |
| 38 | Extract 7z/RAR | Engine: 7-Zip | Password prompt where needed; RAR read-only |
| 39 | Convert archive to ZIP | Engine: 7-Zip | Safe quota-limited staging, then new ZIP |
| 40 | Split archive into volumes | Engine: 7-Zip | New numbered 7z volumes with explicit size/destination |

### PDF: 41–50

| ID | Action | Availability | Behavior and boundary |
|---|---|---|---|
| 41 | Images to PDF | Core | Ordered images/page-size panel; small raster-only writer, no Office |
| 42 | Merge PDFs | Engine: qpdf | Ordered inputs; report encrypted/invalid inputs |
| 43 | Split PDF | Engine: qpdf | New file per page or specified ranges |
| 44 | Extract PDF pages | Engine: qpdf | Validated ordered page ranges |
| 45 | Reorder PDF pages | Engine: qpdf | Number-list reorder; optional thumbnails require PDFium |
| 46 | Rotate PDF pages | Engine: qpdf | Select pages and rotation; save copy |
| 47 | PDF pages to images | Engine: PDFium | Selected pages, explicit DPI and output limits |
| 48 | Extract PDF text | Engine: PDFium | Embedded text; scanned documents get an honest no-text result |
| 49 | Optimize PDF structure | Engine: qpdf | Lossless stream/object optimization; compare size, allow keeping original |
| 50 | Inspect PDF | Engine: qpdf | Page count, version, encryption, metadata; not a security certification |

### Text and data: 51–60

| ID | Action | Availability | Behavior and boundary |
|---|---|---|---|
| 51 | Change text case | Core | Upper/lower/title/sentence picker; document locale behavior |
| 52 | Trim whitespace | Core | Preview leading/trailing/repeated-space options |
| 53 | Sort lines | Core | Ascending/descending, optional case-insensitive comparison |
| 54 | Remove duplicate lines | Core | Keep first occurrence, show count removed |
| 55 | Count text | Core | Characters, words, lines with defined counting rules |
| 56 | Format/minify JSON | Core | Strict parse, error location, copy/save; never evaluate code |
| 57 | URL encode/decode | Core | Distinguish component/full-URL modes; reject malformed escapes |
| 58 | Base64 encode/decode | Core | Text-safe preview; binary output requires Save as file |
| 59 | CSV to JSON | Core | Header/delimiter/encoding preview and explicit output shape |
| 60 | Generate QR code | Core | Local SVG/PNG for text/URL, with capacity limits |

### Browser: 61–70

| ID | Action | Availability | Behavior and boundary |
|---|---|---|---|
| 61 | Bookmark page | Browser | Folder choice; request bookmarks permission when enabled |
| 62 | Save page for later | Browser | Local reading-list URL/title; no full-page mirror implied |
| 63 | Copy clean URL | Browser | Remove known tracking parameters, preserve functional parameters, show result |
| 64 | Copy Markdown page link | Browser | Escaped title and URL |
| 65 | Capture visible tab | Browser | Viewport screenshot, explicitly not full-page capture |
| 66 | Save selected web image | Browser | Allowed-origin fetch; explain protected/blob/canvas limitations |
| 67 | Copy selected web image | Browser | Fetch/decode/copy pixels under the same access limits |
| 68 | Search selected text | Browser | Configured provider, correctly encoded query |
| 69 | Translate selection | Browser | Open configured service; disclose transmitted text before first use |
| 70 | Save quote with source | Browser | Local text, page title, URL, capture time |

### Video and audio: 71–80

| ID | Action | Availability | Behavior and boundary |
|---|---|---|---|
| 71 | Inspect media | Engine: FFprobe | Streams, duration, dimensions, codec, bitrate |
| 72 | Convert video | Engine: FFmpeg | Container/codec presets; capability-aware hardware encoding and CPU fallback |
| 73 | Compress video | Engine: FFmpeg | Quality/resolution and estimated size; no exact-size promise |
| 74 | Trim media | Engine: FFmpeg | Start/end panel; distinguish keyframe cuts from precise re-encode |
| 75 | Extract audio | Engine: FFmpeg | Copy compatible stream or encode selected format |
| 76 | Remove video audio | Engine: FFmpeg | Silent copy; preserve video without re-encode where possible |
| 77 | Video clip to GIF | Engine: FFmpeg | Explicit duration/dimension/frame-rate caps |
| 78 | Extract video frame | Engine: FFmpeg | Time picker, bounded preview, output image |
| 79 | Download online video | Browser + Engine: yt-dlp/FFmpeg | Explicit URL/destination, supported access only, progress/cancel; no DRM bypass |
| 80 | Download online audio | Browser + Engine: yt-dlp/FFmpeg | Audio-only from accessible source, same access/tool-update limits |

### Moodboards and collections: 81–90

| ID | Action | Availability | Behavior and boundary |
|---|---|---|---|
| 81 | Add image to board | Core + Browser | Choose/create board, copy permitted bytes to managed storage, retain provenance |
| 82 | Add webpage to board | Browser | Title/URL; explicitly requested captured thumbnail |
| 83 | Create board from selection | Core | Name board, import local images, report failures |
| 84 | Tag collection items | Core | Applies inside PieAll collections; normalize keys and retain display spelling |
| 85 | Add collection note | Core | Local plain-text note retaining source link |
| 86 | Find similar board images | Core | Local thumbnail perceptual hashes, explicitly visual rather than semantic similarity |
| 87 | Export board contact sheet | Core | Image export using the shared contact-sheet handler |
| 88 | Export board bundle | Core | ZIP images/notes/source links/JSON; preview private fields |
| 89 | Export board palette | Core | Combined swatches, HEX/CSS token export |
| 90 | Open original source | Core | Local path or HTTP(S); missing-file message, reject unsafe URI schemes |

### Windows and screen: 91–100

| ID | Action | Availability | Behavior and boundary |
|---|---|---|---|
| 91 | Capture screen region | Core | Region selector, copy/save; exclude PieAll from capture where supported |
| 92 | Capture active window | Core | Invocation's source window, explicit minimized/protected-window failures |
| 93 | OCR screen region | Core | Region capture plus Windows OCR; preview before copy |
| 94 | Pick screen color | Core | Magnifier and HEX/RGB; state SDR/HDR limitations |
| 95 | Toggle always on top | Core | Captured source HWND, safe handle tracking and toggle restoration |
| 96 | Move to next monitor | Core | Preserve usable bounds and adapt DPI; require another monitor |
| 97 | Center window | Core | Current monitor work area |
| 98 | Fit window to work area | Core | Resizable source only, respect application minimum sizes |
| 99 | Open app file location | Core | Reveal readable executable path; respect protected-process failures |
| 100 | Copy window details | Core | App/title/path/HWND where readable; titles may contain private text |

### Packages and format contract

The catalog contains 65 Core/Core + Browser actions, 11 Browser-only actions, 22 Engine-only actions, and 2 Browser + Engine actions. These are catalog classifications, not measured size claims. Browser-dependent inputs to mixed actions still require the extension.

Optional packages: Archive (7-Zip), PDF structure (qpdf), PDF render/text (PDFium), Media (FFmpeg/FFprobe), Online media (yt-dlp and required runtime), Background removal (ONNX runtime/model). Show each package's actual compressed/installed size, source/publisher, version, license, and uninstall control. Missing engines never masquerade as working built-ins.

Base image contract: static PNG/JPEG/WebP and safe SVG rasterization. Animated files require an explicit frame choice or unsupported-operation message. HEIC, AVIF, RAW, PSD, Office conversion, and generalized semantic AI are future additions, not implicit promises in this catalog.

## 5. Visual and interaction specification

### Material

Use warm glass style for the pie, editors, progress, and dashboard. Proposed tokens are approximations for visual validation, not sampled exact values:

- Light base: warm ivory #F8F1E5; opaque fallback #F3EBDD.
- Raised light surface: #FFF8EB. Dark surface: warm charcoal #28241F.
- Text: #29251F on light; #F8F1E5 on dark.
- Accent: burnt orange #C94F22. Active fill can use #E65A2B with a verified contrasting foreground.
- Fine light borders plus enough darker separation to survive bright backgrounds. Avoid heavy stacked outlines.
- Segoe UI Variable with Segoe UI fallback; no bundled font initially.
- Coherent small line icons from a documented licensed set or simple native geometry.

Use bounded visible surfaces and official supported Windows backdrop/composition APIs. Do not rely on undocumented blur calls. If blur causes contrast, composition, or latency problems, retain warm tinted surfaces without blur. Battery saver, remote desktop, disabled transparency, and high contrast receive opaque rendering. Measure text contrast against actual composited backgrounds; decorative transparency must not reduce readability.

### Pie

- Six visible actions by default, configurable up to eight. Search exposes the full catalog.
- Start near 320 DIPs outer diameter and 88 DIPs center diameter. Adapt for text scaling; use an accessible list when the geometry cannot fit.
- Rounded wedges, small gaps, fine outside rim, restrained shadow.
- Orange hover/selection; distinct keyboard outline and accessible state, not color alone.
- Horizontal one/two-line labels; full labels and disabled reasons available to assistive technology.
- Center shows target thumbnail/type/name and Back/Cancel according to navigation. A thumbnail is not presented as draggable unless it really is.
- Groups replace the current ring with at most eight choices. Back returns to the previous ring. Avoid sprawling nested rings initially.
- Pin positions per context profile. Edit ordering explicitly rather than allowing usage ranking to shuffle familiar slots.
- Clamp the complete hit area to monitor work area; handle negative coordinates and mixed DPI.
- Escape cancels, Tab/arrows navigate, Enter/Space activates. Offer a standard accessible action-list mode through tray and shortcut settings.

### Floating editors

Match warm rounded panels: concise title, close button, large preview, compact grouped controls, obvious Apply/Save. Crop includes aspect presets and numeric bounds. Background includes solid/gradient, padding, corner radius, shadow, and output ratio.

Every slider has a numeric/keyboard equivalent. Preview from bounded thumbnails; full-resolution work begins only after Apply. Reset/Cancel preserve the original. Panels stay usable on small work areas and with larger text.

Proposed motion: open 120 ms, close 80 ms, hover 70 ms, editor reveal 140 ms. Animate opacity and small displacement; avoid perpetual wobble/large zoom. Stop repaint timers after motion. Respect Windows reduced motion. These are starting design values subject to measurement.

### Minimal dashboard

One modest window, around 860 by 600 DIPs where space allows; adapt to smaller displays and text scaling. Carry the same warm material and orange selected navigation.

1. **Actions:** search, context filter, enabled switches, engine/permission badges, six-slot pie preview. Reorder with drag or keyboard Move up/Move down.
2. **Library:** boards, reading list, recent jobs and failure details. Load only the selected view with bounded thumbnail batches.
3. **Community:** pack search/details/import. Publishing/account management may open the hub in the browser.

A single Settings entry holds shortcut, triggers, exclusions, theme/transparency, startup, engines, browser connection, and privacy. No metrics overview, social feed, permanent account banner, or elaborate onboarding. First setup needs shortcut and a test selection; browser connection is optional and engines install on demand.

Use native accessible inputs, lists, buttons, menus, and dialogs with Direct2D styling around them. Expose custom ring actions through UI Automation. Prefer a native control whenever a custom-painted alternative cannot provide correct focus/automation semantics.

## 6. Architecture and ownership

### Native renderer

Default: windows-rs, a Win32 message loop, Direct2D/DirectWrite, and DirectComposition only where transparent presentation needs it. Create the dashboard/tools lazily in the same process. Ordinary forms use native controls; custom drawing belongs to the ring, previews, and material.

This avoids a bundled Chromium/.NET runtime and provides control over startup and rendering. It requires more Windows-specific work for focus, layout, OLE drag/drop, DPI, styling, and accessibility than a web toolkit. Validate this choice in the first vertical slice. Rust alone does not guarantee low memory or small binaries.

Do not initially add Tauri/Electron, a plugin VM, an ECS, a generic conversion graph, or a custom general-purpose widget framework.

### Planned source ownership

Create modules when their milestone needs them; do not scaffold empty extension points.

| Path | Responsibility |
|---|---|
| Cargo.toml, Cargo.lock | One initial desktop crate, pinned versions/features |
| src/main.rs, src/app.rs | Mode dispatch, tray lifecycle, message loop, shutdown |
| src/platform/input.rs | Shortcut registration, minimal hooks, exclusions |
| src/platform/selection.rs | Explorer/desktop selection and UI Automation capture |
| src/platform/drop_target.rs | OLE drop, copy effect, file-list capture |
| src/platform/window.rs | HWND ownership, focus, monitor/DPI, source-window actions |
| src/context.rs | Immutable snapshots, invocation IDs, precedence |
| src/actions/registry.rs | Definitions, predicates, ordering, availability |
| src/actions/images.rs, files.rs, archives.rs, pdf.rs, text.rs, browser.rs, media.rs, collections.rs, screen.rs | Typed handlers grouped by capability |
| src/jobs.rs, src/worker.rs | State transitions, queue, progress, limits, cancellation |
| src/output.rs | Output reservation, safe paths, journal, commit/rollback |
| src/ui/geometry.rs, radial.rs, tools.rs, dashboard.rs, theme.rs, accessibility.rs | Geometry, rendering, focused forms, native controls, UIA |
| src/storage.rs | SQLite schema/migrations/settings/boards/jobs/pack metadata |
| src/engines.rs | Capability inventory and verified engine lifecycle |
| src/bridge.rs | Native messaging/named-pipe validation and source identity |
| src/packs.rs | Declarative pack validation, installation, revisions |
| assets/actions.json | Metadata checked against compiled handler IDs |
| extension/ | Manifest V3 capture/bridge; TypeScript, pnpm lockfile |
| hub/ | Separate public pack service/pages; no invocation-path dependency |
| tests/ | Boundary fixtures and integration tests |
| docs/filee-reuse.md, THIRD-PARTY-NOTICES.md | Provenance and license notices |

Prefer standard library/native APIs. Candidate dependencies: Serde, rusqlite, narrowly featured image/ZIP libraries, reviewed SVG rasterization, SHA-256, QR encoding, and a small raster-only PDF writer. Check Windows support, license, security status, and transitive features before pinning versions. Use WIC when an existing Windows codec meets the contract. Choose one implementation per operation rather than duplicate image pipelines. Do not enable every codec by default.

### Runtime flow

Invocation: Idle -> Capturing -> Showing -> Collecting parameters -> Queued -> Closed. Cancel closes without queuing. Provider timeout returns an explicit retry/fallback.

Job: Queued -> Running -> Succeeded / Failed / Cancelled. Cancellation before commit prevents publication. If commit already completed, report success rather than claiming an output was cancelled. Worker crashes become failed jobs with a fresh retry ID.

Jobs carry action ID, immutable targets, validated parameters, reserved destinations, engine version, and cancellation token. Send small progress/result records to the UI, not full decoded buffers. Use bounded preview images.

Start with one heavy worker and a bounded queue. Add two small I/O jobs only after measurement. Windows Job Objects control child lifetime/resource limits and terminate the engine process tree after a graceful cancellation deadline. A Job Object is not a security sandbox: file parsing still needs trust-boundary limits. Restrict worker privileges where compatible and document the actual security boundary.

The executable supports tray, worker, and native-host modes. Use opaque job IDs and protected pipes rather than placing secrets or selected text on command lines.

## 7. Performance budgets

These are proposed acceptance targets, not measured claims. Reference machine: Windows 11 x64, 4-core CPU, 8 GB RAM, SSD, integrated graphics, 1080p. Also test 4K and mixed-DPI dual monitors. Record exact hardware, OS, power mode, antivirus state, and artifact revision.

| Measurement | Target | Boundary |
|---|---|---|
| Warm invocation to first pie frame | p95 <= 50 ms | Input to presented frame, excluding intentional gesture delay |
| Local Explorer invocation to ready actions | p95 <= 100 ms | Includes capture; slow/unreachable targets may remain resolving |
| Browser invocation to ready actions | p95 <= 150 ms | Connected extension; report cold wake separately |
| Pointer-to-highlight | <= one 60 Hz frame | No decoding, COM, or network in render callback |
| Cold tray-ready startup | <= 750 ms | Process start to registered shortcut/tray; renderer warm-up reported separately |
| Warm dashboard usability | <= 250 ms | No network/full-thumbnail scan required |
| Idle CPU | <= 0.1% five-minute average | Entire PieAll process tree, no jobs/dashboard |
| Idle private committed memory | <= 30 MiB | Resident tray; report working set separately |
| Connected browser bridge idle | <= 40 MiB | All PieAll processes; separately report incremental extension/browser cost |
| Dashboard private commit | <= 70 MiB | Small seeded library, no active jobs |
| Base compressed distribution | <= 15 MiB | Actual signed installer/portable payload; engines/symbols separate |
| Base installed app | <= 30 MiB | All required app/runtime files/assets/notices on clean Windows |

If the first slice misses a target, profile and fix the responsible layer before adding breadth. Never meet targets by trimming working sets, hiding child-process memory, or quoting only compressed executable size.

- No idle screenshots, folder scans, clipboard polling, browser scraping, or network polling.
- Use events, bounded caches, and tiny input callbacks. Cache icons, glyph layouts, and ring geometry.
- Warm rendering after tray readiness or on demand; report first invocation separately.
- Run COM/UIA providers off rendering with deadlines. A timeout does not cancel stuck COM: use a recyclable capture helper if testing requires isolation.
- Stop hidden/stationary animation loops. No perpetual 60/120 Hz timer.
- Downsample previews in workers; cap decoded pixels before allocating full images.
- Load engines/models in workers and release memory when they exit.
- Bound history, paginate libraries, cap thumbnail caches by bytes.
- Measure release size optimization, LTO choice, codegen units, and symbol stripping. Keep separate crash symbols. Do not assume the smallest binary gives fastest interaction.
- Keep recoverable errors/unwinding for the resident process unless a different crash policy is designed and tested.

Use native ETW/WPA or equivalent traces for input/presentation latency, process counters for total CPU/private memory, and actual artifacts for size. Store a repeatable report format in docs/performance.md. Unit-test timings cannot establish perceived latency.

## 8. File safety, privacy, and trust

### Output boundary

- Preserve transform inputs. Use descriptive sibling names and collision suffixes.
- Reserve output names atomically. Prior existence checks alone are race-prone.
- Stage on the destination volume and publish with a no-replace operation. Validate source/destination identity, including hard links.
- Check final destination handles/reparse points at execution/commit, not just preview.
- Test Windows device names, trailing spaces/dots, alternate streams, UNC, Unicode, long paths, and case-sensitive directories.
- Journal batch items and report exact partial completion. No all-or-nothing claim across volumes.
- Rename/move undo is journal-backed best effort. Use shell conflict/Recycle Bin behavior when relevant.
- Archive extraction rejects absolute/traversal paths, unsafe links, alternate streams, and writes through reparse points. Cap entry count, expanded size, compression ratio, and runtime.
- Install engines from a trusted fixed manifest using TLS, expected hashes/signatures, staged extraction, and rollback. Packs cannot supply executable download URLs.

### Browser and IPC

- Chrome/Edge first; Firefox needs its own pass.
- Native host registration is per-user and restricted to approved extension IDs. Bound/validate message sizes, schema, version, invocation ID, and allowed handlers.
- Connect the native host only as needed for responsiveness; measure its resident cost. Reconnect after service-worker suspension.
- Map capture to the foreground browser/profile/window. A cached last-active tab is insufficient; fail closed on ambiguous profile mapping.
- activeTab does not automatically grant capture access when a desktop global shortcut invokes PieAll. Support extension/context-menu gestures first, or request narrow optional host permissions explicitly.
- Restricted pages, cross-origin frames, expiring blobs, and protected images get honest failures. Do not scrape the address bar or bypass access controls.
- Named pipes use current-user ACLs and peer checks. Do not accept arbitrary executable paths in worker/host protocols.

### Packs

Packs are versioned JSON: identity, author, description, supported contexts, built-in action references, typed presets, ordering, and bounded pipelines. Maximum eight steps, no loops, scripts, shell strings, executable payloads, or arbitrary binary downloads. Validate output/input types between steps and disclose side effects.

Installation previews engines, file writes, network destinations, and permissions. Pack composition cannot expand handler privileges. Validate parameters again during execution.

Pin installed revisions. Updates show behavior/permission diffs before acceptance. Curated means reviewed behavior/metadata, not intrinsically safe content. Decode preview images with limits. Revocation produces a clear notice, not an automatic replacement execution.

### Data

SQLite owns settings, overrides, jobs, board items, reading list, packs, and schema versions. Use foreign keys, transactions, serialized writes, and a migration recovery plan; avoid a second competing settings store.

Keep managed board assets in local app data with content-addressed names, quotas, and reference tracking. Stage bytes before adding DB references. Make cleanup idempotent; never remove external originals when deleting a board. Thumbnails are lazy/bounded.

Use DPAPI/Credential Manager for hub session secrets. Redact paths, selected text, URLs, passwords, and tokens from default diagnostics. Analytics off by default. Document/control update, hub, and engine network requests.

## 9. Public community hub

A curated catalog is an intermediate milestone. The original idea requires people to publish packs.

Use a separate small Rust HTTP service, single-deployment SQLite metadata, simple static/server-rendered pages, and object storage for approved previews. No distributed search/queue needed initially. Select actual hosting/authentication at deployment time; this plan creates no accounts or paid services.

Minimum flows:

1. Anonymous browse/search/filter, pack details, manifest, author, immutable versions, permissions, compatibility, engine requirements.
2. Download/import a pack. Add deep links only after URI/file-handler validation.
3. Maintained OIDC sign-in with issuer/audience validation, state/nonce, PKCE, secure session expiry.
4. Author submits/validates/previews a pack into moderation and revises/unpublishes their own submissions.
5. Moderator approves/rejects with reasons, suspends packs, handles reports, and leaves an audit record. Enforce all ownership/role checks server-side.
6. Bound/rate-limit submissions, validate uploaded image bytes, sanitize rendered content, prevent stored XSS/CSRF.
7. Publish revocation notices for installed versions.

Do not include likes, comments, followers, payments, enterprise private packs, or automatic pack execution at launch.

## 10. Implementation milestones

This is a planning-only delivery. At implementation start, create project.md and design.md from the appropriate sections, each with Author: Nitish directly under its title. Record completed work in the daily task file only after completion. No application scaffolding or implementation records are needed for this standalone plan.

### Milestone 1 — Native interaction and accessibility proof

Files: Cargo.toml; src/main.rs, app.rs, context.rs; src/platform/input.rs, selection.rs, drop_target.rs, window.rs; src/ui/geometry.rs, radial.rs, theme.rs, accessibility.rs; tests/radial_geometry.rs, context_priority.rs.

- [ ] Pin minimal dependencies and record geometry-port provenance/license.
- [ ] Test center/outside, wedge midpoints, exact boundaries, wrap at 12 o'clock, empty rings, and negative coordinates.
- [ ] Test drop precedence and rejection of stale capture replies.
- [ ] Implement tray, single instance, configurable shortcut, Explorer/desktop capture, non-activating OLE drop.
- [ ] Render six wedges with an accessible list equivalent. Copy paths and Reveal are real working actions; development-only visual placeholders cannot execute.
- [ ] Through a later user-run app session, verify Explorer tabs/folders, cancellation/focus, mixed DPI, and Narrator.
- [ ] Measure warm/first invocation, idle CPU/memory, prototype artifact size.

Gate: keyboard and drag flows execute real actions, warm material stays legible, accessibility works, and native rendering plausibly meets budgets. Fix this slice before adding breadth.

### Milestone 2 — Safe jobs and first 20 actions

Files: src/actions/registry.rs, images.rs, files.rs, archives.rs; src/jobs.rs, worker.rs, output.rs; src/ui/tools.rs; assets/actions.json; tests/action_eligibility.rs, output_safety.rs, worker_lifecycle.rs.

Initial IDs: 01–15, 21–23, 31–32. Exactly 20 actions.

- [ ] Define typed metadata/parameters, all-target eligibility, and availability reasons.
- [ ] Test source collisions, simultaneous reservations, hard links, reserved names, malicious ZIP entries.
- [ ] Implement bounded worker execution, progress/cancellation, per-item results, staged no-replace output.
- [ ] Implement shared image transforms with orientation, alpha, malformed/huge-image, metadata-policy fixtures.
- [ ] Implement crop and background/padding/corner/shadow panels.
- [ ] Complete initial shell/terminal/ZIP handlers through shared safety boundaries.
- [ ] Verify cancellation on both sides of commit, worker crash, denied permissions, disk-full behavior, and temporary cleanup.

Gate: convert, crop, and frame an image without dashboard/source loss; all initial actions have clear failures.

### Milestone 3 — Dashboard, local breadth, and personal boards

Files: src/ui/dashboard.rs; src/storage.rs; src/actions/images.rs, files.rs, archives.rs, pdf.rs, text.rs, collections.rs, screen.rs; tests/storage_migration.rs, board_import.rs, batch_rename.rs, archive_limits.rs.

Add IDs 17–20, 24–30, 33–36, 41, 51–60, 81, 83–100. Browser-only ID 82 waits for milestone 4.

- [ ] Implement versioned SQLite storage, migration backup/recovery, bounded history/settings.
- [ ] Build Actions/Library/Community shell, search, keyboard reorder, preview, Settings.
- [ ] Add local handlers, Unicode/encoding/capacity limits, screen/window failures.
- [ ] Implement journaled rename plans including overlapping/case-only destinations and partial rollback.
- [ ] Implement managed board imports, notes/tags, similarity, exports, reading-list storage, provenance.
- [ ] Test filesystem/DB failure boundaries, missing originals, escaped exports, bounded thumbnails.
- [ ] Verify keyboard, text scale, contrast, reduced motion, and zero idle library churn.

Gate: every Core action works on its declared local contexts; configuration requires no manual file editing.

### Milestone 4 — Browser integration

Files: extension/package.json, pnpm-lock.yaml, manifest.json, background.ts, content.ts, native.ts; src/bridge.rs; src/actions/browser.rs; tests/bridge_validation.rs.

Add IDs 61–70, 82 and browser inputs to 81.

- [ ] Use pnpm and one extension lockfile; pin dependencies.
- [ ] Implement per-user native host and bounded versioned protocol with invocation/browser identity.
- [ ] Test oversized/malformed requests, wrong identity, stale replies, multiple profiles, disconnects.
- [ ] Start with explicit extension gestures, then desktop-shortcut capture with documented optional permissions.
- [ ] Implement browser actions with restricted-page and image-access failures.
- [ ] Verify service-worker wake/reconnect and incremental browser/host resource cost.

Gate: no ambiguous profile/tab can silently become an execution target.

### Milestone 5 — Engines and full 100-action coverage

Files: src/engines.rs; src/actions/images.rs, archives.rs, pdf.rs, media.rs; assets/engines.json; tests/engine_install.rs, engine_arguments.rs, pdf_ranges.rs, media_cancel.rs.

Add IDs 16, 37–40, 42–50, 71–80. All 100 IDs are now covered.

- [ ] Select exact engine builds, versions, architecture support, source URLs, hashes, licenses, sizes.
- [ ] Test download interruption, wrong hashes, malicious extraction, concurrent install, failed upgrade/rollback.
- [ ] Implement explicit install/uninstall, fixed executable paths, capability refresh, needs-engine UI.
- [ ] Integrate 7-Zip, qpdf/PDFium, FFmpeg/FFprobe, yt-dlp, approved ONNX model with typed arguments and process limits.
- [ ] Verify exact FFmpeg build capabilities/license and yt-dlp runtime requirements. Pack updates cannot replace executables.
- [ ] Test malformed PDFs/media, encrypted archives, encoder fallback, inference limits, process-tree cancellation.
- [ ] Audit every ID for a real handler, context, user-facing failure, and installation/permission path.

Gate: optional packages honestly enable promised actions while idle memory remains free of loaded codecs/models.

### Milestone 6 — Packs and public posting

Files: src/packs.rs; assets/pack.schema.json; src/ui/dashboard.rs; hub/Cargo.toml; hub/src/main.rs, routes.rs, auth.rs, moderation.rs, storage.rs; hub/templates/; tests/pack_validation.rs; hub/tests/authorization.rs.

- [ ] Define schema, typed presets, bounded pipelines, permissions, immutable version identity.
- [ ] Test unknown handlers, type mismatch, cycles/overlong pipelines, executable URLs, permission escalation.
- [ ] Implement local import/export, install preview, pinned versions, update diffs, removal preserving outputs.
- [ ] Implement anonymous catalog/details/download and authenticated author submission/revision.
- [ ] Add ownership checks, moderation, reports/audit, limits, safe previews, suspension/revocation.
- [ ] Test cross-author editing, unauthorized approval, repeated submissions, XSS, revoked versions.
- [ ] Seed Photographer, Designer, Researcher, Developer packs using existing handlers.

Gate: another person can submit, a moderator can approve, and a user can install an exact reviewed version. Hosting/deployment requires a later instruction; do not deploy during implementation without it.

### Milestone 7 — Release verification

Files: docs/performance.md, verification.md, privacy.md; THIRD-PARTY-NOTICES.md; installer/release configuration; catalog/engine manifests.

- [ ] Review all adapted code and retain applicable notices.
- [ ] Complete clean-Windows packaging, signing plan, per-user registrations/startup, uninstall cleanup.
- [ ] Offer explicit retain/remove choices for local data; never delete external originals.
- [ ] Measure actual signed distribution/install sizes and optional packages separately.
- [ ] Verify all 100 actions plus failures, Narrator, keyboard-only use, mixed DPI, text scale, excluded apps, power transitions.
- [ ] Repeat performance protocol after bridge/library/engine integration.
- [ ] Publish tested OS/browser/format boundaries and unsupported cases.

Gate: evidence for performance, safety, accessibility, action coverage, licensing, installer behavior. Do not claim 100 immediately ready offline actions unless the shipped package really supplies that.

## 11. Checks and user-run commands

These commands apply after their source/manifests exist, not to today's planning-only directory:

- Rust format: cargo fmt --all -- --check. Expected clean formatting.
- Rust type check: cargo check --all-targets --locked. Expected success after Cargo.lock exists.
- Rust lint: cargo clippy --all-targets --locked -- -D warnings. Expected no warnings.
- Targeted tests: cargo test --locked --test output_safety, or the specific test target for the milestone. Expected named cases pass.
- Extension type check: pnpm --dir extension exec tsc --noEmit. Expected no errors after configuration exists.
- Hub authorization: cargo test --locked --manifest-path hub/Cargo.toml --test authorization. Expected ownership/role cases pass after its lockfile exists.

Do not run builds, app launches, dev servers, or a broad/full suite under current instructions. Give the user the exact command when needed. Planned examples: release build, cargo build --release --locked; launch, .\target\release\pieall.exe; full desktop suite, cargo test --locked. Installer/signing commands must match the chosen configuration. Static checks cannot prove real latency or screen-reader behavior.

## 12. Sequence, effort, and choices to validate

Milestones 1–2 produce the smallest convincing product: a real pie, a focused workflow, 20 dependable actions. Packs become useful after handler/permission boundaries stabilize.

Rough allowance for one experienced full-time Windows/Rust engineer: 3–5 weeks for native interaction/accessibility and 20 actions; another 6–10 for local breadth/dashboard/boards; 3–5 for browser/engines; 4–7 for hub/security/packaging/release proof. Total 16–27 engineering weeks, with substantial uncertainty. This is a scope estimate, not a commitment; replace it with measured velocity after milestone 2.

Resolve through evidence rather than blocking planning on preference questions:

1. Supported Windows composition achieving the target appearance within budgets. Opaque warm styling remains a complete path.
2. Reliable active Explorer tab/desktop selection including directories. Initially guarantee Explorer and explicit OLE drops, not every third-party file manager.
3. Plain-backtick typing/layout behavior; modified shortcut remains available.
4. Exact codec/PDF/model/CLI artifacts meeting size, security, capability, and license requirements.
5. Global-shortcut browser permissions and profile mapping; explicit extension invocation remains supported.
6. UIA and crop-editor keyboard accessibility. Accessible list/numeric controls are requirements.
7. Hub hosting/authentication and PieAll's own public distribution license. The owner decides its license; third-party notices remain required regardless.

When a gate fails, fix its responsible layer before increasing scope.
