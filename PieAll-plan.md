# PieAll implementation plan
Author: Nitish

Date: 19 October 2026; expanded 21 October 2026.
Status: partly implemented. The current Tauri app has 44 local commands plus contextual app shortcuts. Section 13 records the expanded toolkit scope and the implementation gaps. Native acceptance and performance remain unverified; planned tools are not shipped capabilities.

**Goal:** Build a fast Windows radial action menu that understands the selected item, opens with a configurable backtick shortcut, and lets people finish useful tasks without opening a large dashboard.

**Architecture:** The current app uses Tauri 2 and React/TypeScript over the existing Rust execution core. Rust owns invocation, immutable context, eligibility, operation exclusion and output safety; React owns presentation. Expensive operations run in temporary worker processes. The planned browser extension supplies browser context. A planned separate community hub distributes reviewed declarative packs that invoke the same built-in handlers. Current behavior in project.md supersedes the original invocation/rendering proposal below.

**Tech stack:** Tauri 2, React/TypeScript, Rust, windows-rs, Win32, Serde and narrowly enabled image/archive libraries. SQLite is planned for persistent app data. The initial rendering proposal used Direct2D/DirectWrite, optional DirectComposition and Windows Imaging Component; that proposal remains historical rather than a requirement to replace Tauri. A Chromium Manifest V3 extension in TypeScript and a separate Rust HTTP service with simple web pages remain planned.

**Execution:** Implement sequentially in the existing folder. Do not create branches, commit, push, deploy, start a dev server, run a build, or launch the app under the current working instructions. Checks and targeted tests are allowed. No delegated agents are required.

## 1. Product decisions

- Windows 11 x64 first. Record the tested Windows build in each release. ARM64 and Windows 10 require separate compatibility work.
- The pie is the main surface. The dashboard handles configuration, action discovery, recent jobs, and collections.
- Use warm translucent surfaces, separate rounded wedges, fine light borders, orange selection, and compact floating editors.
- Support keyboard invocation on the current selection and modifier-plus-file-drag invocation. Both use one registry and execution path.
- Preserve the original 100-action catalog as the baseline. Section 13 expands the product with 72 tracked tool workflows, including overlaps and Windows adaptations. Do not advertise 172 distinct actions: workflows, variants and handlers overlap. Release smaller verified groups; some require an optional engine or browser extension. State that distinction in onboarding and marketing.
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

## 4. Original baseline catalog: exactly 100 actions

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
| 66 | Save selected web image | Browser | One-action save of the explicitly targeted image at the best available source resolution to the configured web-download folder; allowed-origin fetch with protected/blob/canvas limitations; see 13.14 |
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
| 79 | Download online video | Browser + Engine: yt-dlp/FFmpeg | Invoke over the active supported video page/post and choose Download video; captured URL, best available video plus audio by default, configured destination, background progress/cancel; explicit pasted URL also supported; see 13.14 |
| 80 | Download online audio | Browser + Engine: yt-dlp/FFmpeg | Same contextual or pasted-URL flow, extracting the best available audio by default; configurable copy/encoding settings, supported access and tool-update limits |

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

The original delivery of this plan was planning-only. Implementation has since started; project.md and design.md exist, each with Author: Nitish directly under its title. Record completed work in the daily task file only after completion. The milestones below retain the original baseline; section 13 supplies the expanded sequence against the current Tauri implementation.

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

## 13. Expanded custom toolkit — 21 October 2026

Build focused tools inside PieAll for image editing, PDFs/documents, conversion, compression, video/audio, text/speech, files, Windows utilities and moodboards. Reuse maintained open-source engines behind PieAll's own controls. A tool opens from the ring when its input is captured, and from the dashboard when the user needs to choose input. It must perform the operation locally; an external-app shortcut does not count as an implemented editor.

This section incorporates the seven supplied screenshots and an extracted catalog of 72 tools, including their individual workflow descriptions. The catalog has 15 video, 5 audio, 8 image, 22 document, 5 text/speech, 2 file and 15 system workflows. All 72 are accounted for below. The 15 system workflows are adapted to Windows; Apple frameworks, phone continuity, Apple Intelligence, Homebrew and macOS settings are not dependencies. The screenshots establish discovery and editor requirements, not permission to copy another application's artwork, brand or performance claims.

T01–T72 are planning identifiers for these workflows. They do not renumber baseline IDs 01–100 or replace existing native command IDs. Multiple workflows can share a handler. The catalog is additional product scope, not a claim that 72 new transformations already exist. Moodboards, archives, original text tools, browser actions, window tools and community packs remain part of the plan even when absent from these 72 workflows.

### 13.1 What exists and what remains

Status below comes from `src/actions.rs`, `src/output.rs`, `src/platform.rs`, `src-tauri/src/shell.rs` and the current React UI. A source implementation is not native acceptance evidence. The existing 44 local commands count individual case, encoding, rotation and flip variants; app shortcut bindings count separately and do not complete tool milestones.

| Area / baseline IDs | Current implementation | Remaining work |
|---|---|---|
| Image editing, 01–15 and 17–20 | Numeric crop; bounded resize and native percent preset; PNG/JPEG/lossless WebP conversion; JPEG/transparent-PNG compression with no-shrink result; rotate/flip; metadata re-encoding; details; clipboard pixels; quantized palette; solid frame/corners; grayscale; contact sheet | Pointer and keyboard crop handles/aspect presets; richer resize controls; estimated/actual output preview; metadata/format inspection; swatches; gradient/image backgrounds, shadows and ratios; watermark; OCR; safe SVG rendering; labelled/orderable contact sheets; two-image comparison |
| Background removal, 16 | No model integration | Explicit optional engine/model install, license/RAM information, mask preview, edge review, alpha/solid output |
| Files/folders, 21–30 | Copy paths, Reveal, Terminal, duplicate files, SHA-256, folder size, list-only duplicate detection | Quoted path choice; folder duplication; batch rename preview/undo; copy/move destination and conflicts; cancellation, inaccessible-item reports and scan navigation |
| Archives, 31–40 | ZIP from selected files; extraction into a new sibling folder; entry list; integrity test | Folder traversal for ZIP; previewed merge extraction; selected-entry extraction; itemized integrity failures; 7z encryption, 7z/RAR reading, conversion and split volumes |
| PDF, 41–50 | Images to raster PDF with fixed A4 pages | Ordered page-size/margin editor; merge/split/extract/reorder/rotate; render/text extraction; structural optimization and inspection; expanded PDF editor features below |
| Text/data, 51–60 | Case/trim/sort/dedupe/count; JSON format/minify; URL-component and text Base64 encoding/decoding | Sentence case/options/preview; sort direction; counts removed; save output; full-URL mode; binary Base64 export; CSV-to-JSON; QR create/read |
| Browser, 61–70 and 82 | No browser bridge or extension | Bookmarks, reading list, clean/Markdown links, viewport capture, permitted web-image access, search/translation, sourced quotes and board URLs |
| Video/audio, 71–80 | No media engine or handlers | FFprobe metadata, conversion/compression/trim/audio/frame/GIF operations, explicit online downloads; expanded workflows T01–T20 |
| Moodboards, 81–90 | No collection database, asset store or board UI | Managed local images, named boards, notes/tags, provenance, similarity, contact sheet/bundle/palette exports, original-source opening |
| Windows/screen, 91–100 | Always on top, center, fit-to-work-area, basic title/HWND report | Screen/region capture, OCR, color picker, next-monitor move, executable location, richer details and resizable-window constraints |
| Cross-cutting product work | Tauri tray/ring/dashboard, contextual app shortcuts, ring layouts and trigger choice, isolated timed workers, collision-safe output, signed updater/release configuration | Versioned SQLite and settings migration; cancel/process-tree cleanup; explicit engine management; typed availability reasons; accessible rich editors; modifier-drag; exclusions; recent jobs/favorites; packs/hub; native acceptance, installer/release and performance evidence |

### 13.2 Video — 15 workflows

Every timeline has numeric start/end controls, keyboard seeking, a preview of the selected segment and explicit output settings. Probe duration, streams and rotation before presenting valid choices. Honor variable frame rates, orientation, audio-only inputs, missing streams and unsupported codecs. Use FFmpeg/FFprobe as optional engines; do not keep codecs loaded in the resident tray process.

| ID | Tool | Required input, controls and result | Existing overlap / status |
|---|---|---|---|
| T01 | Download video | Primary flow: invoke PieAll over an active supported video page/post and choose Download video; use the captured URL and yt-dlp in the background, best available video plus audio by default, with FFmpeg merging when necessary. App settings customize quality/format/codec/audio/destination; per-run overrides are optional. Explicit one/many pasted HTTP(S) links remain available without the extension. See 13.14 for targeting and defaults. | Extends 79–80; missing |
| T02 | Burn in subtitles | Video plus SRT/WebVTT, or a reviewed local transcript; text size/style/position with preview; encode a captioned copy. Parse subtitles as data and restrict referenced fonts/assets. | New; missing |
| T03 | Change speed | 0.25×–4×, presets and new-duration readout; preserve pitch by default, optional mute; output preset. | New; missing |
| T04 | Extract frames | One timestamp, every N seconds or N evenly spaced frames; PNG/JPEG, size and count limits; publish a new frame folder. HEIC is available only after its codec gate. | Extends 78; missing |
| T05 | Record screen | Display/window/area selection; independent system-audio and microphone toggles; pointer/click highlight options; native/1440p/1080p/720p, 30/60 fps when supported; H.264 MP4/MOV; visible record/pause/stop and recoverable partial recording. | Extends screen work 91–92; missing |
| T06 | Compress video | High/Balanced/Small quality; optional 1080p/720p/480p cap, audio removal and supported hardware encoding with CPU fallback; show before/after bytes and honest estimated size. Save nothing by default when output is larger. | Extends 73; missing |
| T07 | Convert video | MP4/MOV/WebM/M4V; compatible codec/quality presets; remux when compatible, otherwise re-encode; disclose which path applies. Availability follows the installed engine's real encoders. | Extends 72; missing |
| T08 | Resize video | Presets/custom dimensions; fit, pad, crop-to-fill and explicit stretch; aspect lock; resulting dimensions and codec-evenness validation. | New shared video transform; missing |
| T09 | Crop and rotate video | Playing preview with crop handles and numeric equivalents; free/16:9/9:16/1:1/4:3/4:5; quarter-turn rotation and horizontal/vertical flip; output dimensions. | New shared video transform; missing |
| T10 | Media info | Streams, duration, dimensions, display rotation, frame rate, codec, bitrate and readable metadata for video/audio; copyable report. Missing values stay unknown. | Same family as 71; missing |
| T11 | Trim video | Timeline and numeric times; fast keyframe-aligned stream copy or precise re-encode; preview and explain actual cut boundaries. | Extends 74; missing |
| T12 | Merge videos | Explicit ordered clips with keyboard move controls; output resolution; normalize incompatible streams before joining; combined duration and per-input failures. | New; missing |
| T13 | Mute video | Remove audio while stream-copying compatible video; a no-audio input gets a clear unchanged result. | Same family as 76; missing |
| T14 | Replace audio | Primary video plus explicitly chosen audio/second video; replacement track loops to fit or plays once then silence; original/new track volume and preview; video duration remains the boundary. | New; missing |
| T15 | Video to GIF | Selected interval, width, frame rate, loop option and palette generation; hard duration/frame/pixel/output caps; approximate size warning before export. | Extends 77; missing |

### 13.3 Audio — 5 workflows

| ID | Tool | Required input, controls and result | Existing overlap / status |
|---|---|---|---|
| T16 | Clean up audio | Audio/video; Light/Medium/Strong denoise, rumble/hiss filtering, optional loudness normalization and silence removal; A/B playback and peak/clipping indication; explain when silence removal changes timing. | New; missing |
| T17 | Record audio | Native microphone/device choice and permission guidance; live level, mono/stereo, M4A/WAV when supported; record/pause/stop; retain recording if transcription fails; explicit Transcribe handoff. | New; missing |
| T18 | Trim audio | Waveform with numeric start/end; playback selection; optional fade-in/out with validated lengths; new audio file. | Extends 74 for audio; missing |
| T19 | Extract audio | Video/audio stream choice; M4A/MP3/WAV/FLAC and supported bitrate; lossless stream copy when compatible, otherwise explicit encoding. | Extends 75; missing |
| T20 | Convert audio | MP3/M4A/WAV/FLAC/AIFF when supported; bitrate, sample rate and channel options; original-rate choice; explicit lossy/lossless labeling. | New shared converter; missing |

### 13.4 Images — 8 workflows

Retain the existing source-preserving pipeline and orientation normalization. Preview the first image prominently, but make every batch item reviewable and report its outcome. Automatic detection never guarantees that every face or sensitive detail was found.

| ID | Tool | Required input, controls and result | Existing overlap / status |
|---|---|---|---|
| T21 | Blur faces | Local face detection with reviewed model license; editable/addable/removable regions on each image; blur/pixelate/solid cover, strength and before/after preview; explicit no-face and missed-detection states. Solid cover replaces pixels; blur is not a privacy guarantee. | New; missing |
| T22 | Beautify screenshot | Solid, gradient or local-image background; padding, corner radius, shadow; natural/16:9/4:5/1:1/custom canvas; live dimensions and keyboard controls; full-resolution PNG export. | Extends 13 / `frame`; partial: solid/padding/corners only |
| T23 | Remove background | Local cutout preview with editable/refinable mask; transparent/white/custom background; optional crop to subject; PNG output and no-subject failure. | Extends 16; missing |
| T24 | Convert images | PNG/JPEG/WebP initially; quality, alpha flattening and metadata policy. Add TIFF/HEIC reading/writing only after exact Windows codec/license/fixture checks; expose unsupported direction clearly. Safe SVG rasterization remains baseline 18. | Extends 03 / `convert`; partial format coverage |
| T25 | Compress images | Quality presets plus numeric control; keep format where a real encoder can reduce it, or explicit JPEG/WebP; compare size and visual preview; alpha/metadata policy; no guaranteed invisible loss. | Extends 04 / `compress`; partial |
| T26 | Resize images | Percentage, longest side or exact size; proportions, explicit enlargement choice, resulting dimensions; preserve the current no-upscale default. | Extends 02 / `resize`; partial UI |
| T27 | Rotate images | Composable left/right/flip operations, batch preview and final dimensions; normalize orientation first. | Baseline 05–08; separate rotate/flip commands implemented, unified editor missing |
| T28 | Remove metadata | New re-encoded image with normalized orientation; disclose retained color information and removed metadata classes; inspect output fixtures rather than promise complete sanitization. | Baseline 09 / `strip_metadata`; implemented re-encoding, richer review missing |

### 13.5 Documents and PDF — 22 workflows

PDF editing means page organization, text/annotation overlays, forms, signatures and the operations below. Arbitrary in-place editing of existing PDF text/layout is a separate future capability, not implied by the word editor. PDF-to-Word reconstructs editable content with stated fidelity limits. A drawn signature is a visible mark, not certificate signing. Flattening forms does not make a PDF tamper-proof or perform redaction.

| ID | Tool | Required input, controls and result | Existing overlap / status |
|---|---|---|---|
| T29 | Scan document | Local photos or explicit Windows camera/scanner input; detected page quadrilateral with manual corners; perspective/deskew; Color/Grayscale/Black & White; reorder/rotate/delete, paper size and optional OCR layer. No phone-account integration is required. | New; missing |
| T30 | Redact PDF | Page preview; manual regions and reviewed find-all matches; burn solid regions into bounded raster pages and rebuild a fresh document excluding original streams, attachments, metadata and prior revisions. Optionally OCR only the final visible pixels. Verify hidden text cannot be extracted or recovered from the saved object graph. | New; missing; security acceptance gate |
| T31 | Fill and flatten forms | List supported AcroForm text/checkbox/choice fields with page numbers; keyboard editing; add positioned text for static forms; keep fields editable or flatten appearances into a copy; show unsupported XFA/signature fields. | New; missing |
| T32 | Make PDF searchable | Local OCR and selected installed languages; skip pages with existing text or explicitly re-recognize; invisible aligned text layer; progress per page and OCR accuracy review. Re-rendering can enlarge/change pages and must be disclosed. | Extends OCR 17/48; missing |
| T33 | PDF to Word | DOCX/RTF/TXT output; embedded text first, optional OCR for scans; headings/paragraphs/lists, optional images/page breaks; preview reading order and disclose table/multicolumn limitations. | New; missing |
| T34 | Sign PDF | Draw/type/import a local signature; position/resize with numeric and keyboard equivalents; optional explicit local signature saving/removal; export a new PDF; report invalidation of existing digital signatures. | New; missing |
| T35 | Merge PDFs | Ordered PDFs and supported images; thumbnails with keyboard reorder; optional per-source bookmarks, encrypted/invalid input handling; one copy. Mixed inputs reuse images-to-PDF. | Extends 41–42; PDF merge missing |
| T36 | Split PDF | Every page, every N pages or custom range groups including open-ended ranges; page-count validation, preview of output names/counts; publish bounded new files. | Extends 43; missing |
| T37 | Organize PDF | Lazy bounded page thumbnails; reorder/rotate/delete with keyboard move controls and undo inside the editor; preserve untouched page content where supported. | Extends 45–46; missing |
| T38 | Delete pages | Select/remove/invert/keep-all; numeric ranges; preview kept pages; reject deletion of all pages; save a copy. | Shared organizer; missing |
| T39 | Extract pages | Ordered selection/ranges; one PDF or separate files; reject out-of-range/empty input; explicit duplicate-page behavior. | Extends 44; missing |
| T40 | Images to PDF | Orderable images; fit-image/A4/Letter, orientation and margins; alpha background and resulting page count. | Baseline 41 / `images_pdf`; partial: fixed A4/selection order |
| T41 | Compress PDF | Separate lossless structure optimization from image recompression/downsampling; Screen/Balanced/Print presets; retain selectable text when chosen; before/after bytes; save nothing by default if larger. | Extends 49; missing; qpdf alone does not supply image recompression |
| T42 | Add password | Password and confirmation; modern encryption supported by the selected engine; explicit print/copy permission choices with a warning that viewers may ignore restrictions; no secret persistence/logging. | New; missing |
| T43 | Remove password | User-supplied known password; validate and export an unlocked copy; no cracking; per-file errors; never place the password in a visible process command line. | New; missing |
| T44 | PDF to images | Selected/all pages; PNG/JPEG, 72/150/300 dpi; page/pixel/byte quotas and ordered naming; HEIC only with verified codec support. | Extends 47; missing |
| T45 | Convert documents | DOC/DOCX/ODT/RTF/HTML/TXT/Markdown capability matrix by engine and direction; retain supported styles/lists/links/images; disclose layout, comments and tracked-change losses; bounded asset folder/bundle for HTML/Markdown outputs. | New; missing |
| T46 | Word to PDF | DOC/DOCX/RTF/TXT to PDF with optional LibreOffice; page size, font/substitution and pagination review; no macros or external-link updates. | New; missing |
| T47 | PowerPoint to PDF | PPT/PPTX/ODP to PDF with optional LibreOffice; slide/notes export only where supported; preview and missing-font/layout failures. | New; missing |
| T48 | Excel to PDF | XLS/XLSX/ODS to PDF with optional LibreOffice; sheet choice, print area, scaling and pagination; preview cropped/overflowing content; preserve the source workbook. | New; missing |
| T49 | Web page to PDF | Explicit HTTP(S) URL, active-tab browser handoff or local HTML; paginated A4/Letter or bounded long page; network/source disclosure, resource limits, no access to arbitrary local files from page content. | Extends browser work; missing |
| T50 | Markdown to PDF | Local Markdown; Clean/code-friendly styles, paper size, fenced code/tables and local images; sanitize raw HTML, disable scripts and remote resources by default; preview before saving. | New; missing |

### 13.6 Text and speech — 5 workflows

Language models belong in explicit tools after activation, never in selection capture or ring rendering. Downloads are separate from inference: label installed model, supported languages, disk/RAM cost and local-only execution. Do not silently fall back to a cloud provider.

| ID | Tool | Required input, controls and result | Existing overlap / status |
|---|---|---|---|
| T51 | Transcribe | Local audio/video/recording, selected language or explicit autodetect, timestamped segment review; TXT, timestamped TXT and SRT export; local speech model, cancellation and retained source recording. | New; missing |
| T52 | Summarize | Explicit text/document input via shared extractors; Short/Medium/Detailed and Paragraph/Key points/One line; source alongside editable/copyable output; local optional model, context limits/chunking, uncertainty and unsupported-document errors. | New; missing |
| T53 | Extract text | Images/screenshots/scanned PDFs; embedded text where present, local OCR otherwise; installed language choice, page/file selector, review/copy/TXT/save-all. | Extends 17/48; missing |
| T54 | Capture text | User-selected screen region, OCR preview, explicit copy or opt-in automatic copy; optional bounded history and delete controls. | Extends 93; missing |
| T55 | Translate | Explicit text/document/SRT; source/target language and swap; side-by-side review; local downloadable language pair; TXT/SRT output preserves cue timestamps. Document translation initially exports text unless layout preservation is actually supported. | Expands browser-only 69 with a distinct local handler; missing |

### 13.7 Files — 2 workflows, alongside the existing file/archive catalog

| ID | Tool | Required input, controls and result | Existing overlap / status |
|---|---|---|---|
| T56 | QR code | Create Link/Text/Wi-Fi codes with size/colors/error correction and capacity/contrast checks; copy pixels, PNG/SVG export; read QR and supported barcodes from bounded images; decoded links are displayed before opening. Wi-Fi secrets stay ephemeral. | Extends 60 with reader and Wi-Fi modes; missing |
| T57 | Batch rename | Files/folders; replace/add text, template/numbering/date/case modes; deterministic order; before/after names and conflicts; journaled two-step rename for overlaps/case-only names; explicit commit and recoverable undo without overwriting intervening files. | Baseline 24; missing |

The general converter is a single entry point to image, video, audio, PDF and document converters. It lists only supported input→output pairs and names the required engine. It does not promise arbitrary file-to-file conversion. The compressor similarly routes images, PDF, video and archives to the matching implementation; archive compression never implies lossy recompression of its contents.

### 13.8 Windows utilities — 15 adapted workflows

Prefer documented Windows APIs and supported Settings pages. Show unavailable fields and hardware capabilities honestly. System changes need a review of the exact targets, rollback where feasible, and an explicit user action; packs cannot grant themselves elevated privileges or invoke arbitrary shell commands.

| ID | Windows tool | Required behavior and platform boundary | Existing overlap / status |
|---|---|---|---|
| T58 | Network speed test | Explicit download/upload/latency test with named endpoint/provider and bounded traffic; sequential/simultaneous modes only if supported; cancel and local opt-in history; disclose data transfer before starting. Select an endpoint/license during implementation; do not reuse an Apple-only command. | New; missing |
| T59 | Storage explorer | Choose drive/home/folder; cancellable size tree and largest-files view; report inaccessible paths and skip links; reviewed cache/download candidates; Recycle Bin through Windows shell with per-item failures. Exclude system/active app data from generic cleanup. | Extends 28–30; scans partial, explorer/cleanup missing |
| T60 | Wi-Fi password and QR | Explicit saved-network selection; documented WLAN API and OS access checks; reveal/copy/share only after user activation; manual SSID/password QR remains a complete fallback; no admin-password collection, persistence, history or diagnostics containing credentials. | New; missing |
| T61 | Battery health | Design/full-charge capacity, charge, cycle count, condition and charger data where exposed by Windows/hardware; supported accessory batteries only; unavailable values remain unavailable. | New; missing |
| T62 | Keep awake | 15 minutes/1 hour/3 hours/until time/indefinite; optional display awake and AC-only; Windows power request released on stop, expiry, exit or crash; visible active indicator. | New; missing |
| T63 | Uninstall app | Inventory installed apps and publishers; invoke the registered supported Windows uninstall flow, then separately review confidently attributable leftover user files for Recycle Bin cleanup. Do not delete Program Files/registry trees by guessed name or promise an undo for vendor uninstallers. | New; missing |
| T64 | Presentation mode | Explicit set of supported desktop/taskbar/privacy/power changes; record each original value; status/stop and restore only owned changes; no forced process termination or hidden automatic screen capture. Unsupported shell toggles remain Settings links. | New; missing |
| T65 | Clean keyboard | Explicit 30/60/120-second pause, visible countdown and pointer-operable Stop; watchdog releases suppression on timeout/crash; secure attention/power controls untouched; coordinate with PieAll's hold hook and prove accessibility/recovery before enabling. | New; missing |
| T66 | Eject external drives | Enumerate eligible removable volumes/devices and show exact targets; per-drive/eject-all; Windows veto/open-file errors; never force removal or include the system drive; wait for confirmed ejection. | New; missing |
| T67 | Startup and background items | Inventory supported Startup folders/Run entries and distinguish services/tasks; publisher/signature details without implying unsigned equals malicious; reversible disable for owned supported types; use Windows management UI for unsupported/elevated types. | New; missing |
| T68 | App launch diagnostics | Windows equivalent of the damaged-app workflow: inspect PE architecture, signature, downloaded-file marker and launch error; offer official repair/reinstall/settings guidance. Any per-file unblock is a separate reviewed action naming its effect; never disable Defender/SmartScreen globally or call an invalid signature safe. | New Windows adaptation; missing |
| T69 | App permission help | Inspect available camera/microphone/privacy state and open the matching Windows Settings page; explain restart/regrant steps. Windows has no general equivalent of macOS permission reset; expose a reset only for a specifically documented supported API, never delete permission registry stores. | New Windows adaptation; missing |
| T70 | Advanced Windows settings | Small allowlist of documented Explorer/taskbar/screenshot/file settings; read actual values, explain scope and restart needs, preserve prior values and offer restore. Unsupported settings route to native UI. | New; missing |
| T71 | Display control | Per-monitor brightness/contrast/volume only when exposed by internal-display APIs or DDC/CI; query limits, show unsupported controls disabled, debounce writes and recover from disconnect; optional explicit restore of saved levels. | New; missing |
| T72 | Mouse controls | Per-device wheel direction/speed/smoothing only where device identity is reliable; preserve precision touchpad behavior; test area, enable/disable and restore; uncertain mixed-device events use native Windows settings instead of global rewriting. | New; missing |

### 13.9 Moodboards and collections remain a full workstream

Build baseline 81–90 in the Library, not as files dumped into a generic folder. A board owns its name, ordered items, notes, normalized tags and optional cover. Each item records its import time and local/browser provenance; managed image bytes survive external-original deletion. Show missing external originals without losing the managed copy. Add selected images to an existing/new board or create one from the current selection through the same native revision validation.

- Use versioned SQLite with foreign keys, transactions and serialized writes. Migrate existing `pieall.prefs` settings through an explicit versioned bridge, preserving customization; do not create a second lasting settings owner or delete the old data before a verified import.
- Stage decoded/validated assets before inserting references. Content-address originals/thumbnails, enforce board/item/storage quotas, deduplicate bytes and reference-count them. Failed DB/file steps leave recoverable orphans, not dangling live references. Deleting a board removes only owned data after reference checks, never external originals.
- Provide an image grid, zoomed preview, notes/tags/search, keyboard selection and reorder, add/remove with recoverable undo, and optional spatial arrangement with persisted positions. The grid is a complete keyboard-accessible view of every canvas item; pointer dragging is never the only route.
- Add plain notes, color swatches and HTTP(S) source links. Browser images/pages require the explicit bridge/access permission; never fetch remote content merely because the Library opens.
- Find similar images using local thumbnail perceptual hashes and a stated visual-distance threshold; distinguish visual similarity from semantic search and let the user review matches.
- Export an ordered contact sheet, a combined palette as HEX/CSS, and a ZIP bundle of images/notes/source links/JSON. Preview included private paths/URLs/notes; make provenance inclusion explicit. Escape HTML/CSS/Markdown and archive names. Reuse the existing image/contact-sheet/ZIP/output owners.
- Open original local sources or HTTP(S) URLs after validation; report missing files and reject executable/unsafe URI schemes. Use bounded lazy thumbnails and no idle library scanning.

### 13.10 Shared tool experience

The dashboard gains a Tools discovery view with All, Video, Audio, Images, Documents, Text & Speech, Files, Windows and Boards. Keep the contextual ring as the primary quick-action surface. A tool can be discovered before selecting input; its page explains accepted formats and offers a native picker. Changing input still goes through Rust, increments the revision and invalidates stale editor/preview replies.

Use a compact searchable catalog with meaningful previews and concise descriptions, category counts from registry data, favorites and recent tools. Recent tool IDs are separate from private recent files/text; content history is opt-in, bounded and deletable. Show actual measured compression savings only after completed jobs, never illustrative marketing numbers. Favorites are keyboard-reorderable.

Every tool page follows Input → Preview and settings → Run → Result. Larger editors use a preview area plus an inspector; simple tools keep a compact panel. Include empty/loading/error/unsupported/no-change/cancelled/partial-success states. Format/engine incompatibility has a specific reason and recovery action. Avoid one generic numeric form for timelines, page ordering or signatures, while retaining labelled numeric equivalents for pointer controls.

Each batch has an explicit ordered input list, limits, per-item results and an output-policy summary. Keep smart collision-safe outputs beside the source by default. Offer an explicit destination for user-imported/downloaded/recorded content and board exports; do not silently replace PieAll's sibling-output rule with a Downloads-only policy. Large multi-output operations stage a new named folder and never merge it into existing user content without a reviewed conflict policy.

All editors support keyboard navigation, visible focus, undo for uncommitted edits, Escape/cancel behavior, reduced motion, high contrast and text scaling. Selecting another navigation item cannot silently discard an active edit or leave an invisible native editor lease. Cancel stops the process tree, releases devices/power/input hooks and cleans uncommitted temporary output. Completed published outputs remain listed after cancellation or a later item failure.

### 13.11 Open-source implementation choices

These are candidates/ownership decisions, not installed or vetted binary releases. At integration time choose exact versions, source URLs, hashes, Windows x64 artifacts, build flags, full licenses/notices, measured compressed/installed size and security-update ownership. Never download a floating latest executable or let a pack supply one. Keep source URLs for real dependencies in engine manifests/notices, separate from product inspiration.

| Responsibility | Preferred owner / candidate | Integration boundary |
|---|---|---|
| Image pixels and ZIP | Existing `image` and `zip` crates plus `src/output.rs` | Extend the existing bounded worker pipeline; no competing converter or output safety layer |
| SVG and extra image formats | `resvg`/`usvg` candidates for safe SVG; evaluate TIFF support in `image`; select HEIC decoder/encoder separately | Disable scripts/external resources, cap dimensions/allocation; review HEIC codec/patent/distribution requirements before enabling it |
| PDF structure/encryption | qpdf, Apache-2.0 upstream license | Typed merge/page/range/rotation/inspection/encryption commands; passwords via a supported protected channel, not argv; preserve object/reference correctness |
| PDF rendering/forms/text | PDFium, upstream BSD-style license plus bundled dependency notices | Disposable worker, JavaScript/actions disabled, bounded page rendering; form appearance and text extraction fixtures; qpdf is not a renderer or redaction engine |
| PDF image compression/redaction | Reuse PDFium rasterization, existing image encoders and a PDF writer; evaluate `lopdf` for supported object edits | Separate structural optimization from lossy image rebuilding; remove original sensitive objects during redaction; do not add Ghostscript without reviewing its AGPL/commercial distribution implications |
| Video/audio codecs | FFmpeg/FFprobe; exact artifact may be LGPL or GPL depending on configuration | Fixed executable paths, typed arguments and detected encoders/filters; no shell interpolation; never label every FFmpeg build LGPL |
| Online media | yt-dlp plus required JS runtime/FFmpeg for the selected release | Exact executable and dependencies/notices, restricted explicit URLs, no shell, bounded downloads, no DRM bypass; review Windows binary's bundled licenses separately from upstream source license |
| Speech transcription | `whisper.cpp` candidate, MIT source license; independently licensed model weights | Explicit local model package, language/timestamp/accuracy fixtures, inference cancellation and RAM caps |
| Local summarization | `llama.cpp` candidate, MIT source license; independently selected licensed model | Optional download; document context/RAM limits and license; grounded input review, no cloud fallback |
| Local translation | CTranslate2 candidate plus a redistributable language-pair model | Runtime and model licenses reviewed separately; supported language-pair inventory; SRT structure/timing preserved |
| OCR | Windows OCR for installed supported languages; Tesseract candidate for optional language packs | Select languages explicitly; bounded raster inputs, confidence/review UI; do not promise automatic support for every language |
| Background/face detection | ONNX Runtime candidate plus separately licensed segmentation/detection models | Verify Windows artifact, model source/hash/license/input normalization; mask/region review and measured RAM; no resident inference |
| Office conversion | Optional LibreOffice, MPL-2.0 plus third-party notices | Isolated per-job profile, supported headless filters, macros/external links disabled, process-tree cancellation; never hijack the user's existing Office session |
| Editable document conversion | LibreOffice where fitting; Pandoc candidate for Markdown/HTML/DOCX/ODT directions | Explicit format-direction matrix and fidelity tests; review Pandoc GPL distribution obligations before shipping; no arbitrary filters, scripts or remote includes |
| HTML/Markdown PDF | Sanitized local HTML rendered through a dedicated WebView2 print path or approved optional Chromium renderer | No existing browser-profile reuse, no arbitrary local-file access, remote resources opt-in, bounded long-page output; current shell window is not the renderer |
| Archives | Existing ZIP first; optional 7-Zip with its LGPL/BSD/unRAR restrictions reviewed | Existing extraction guards and expansion quotas remain mandatory; no raw extract-to-user-folder subprocess |
| Storage and QR | SQLite via a narrow Rust binding; evaluate `qrcode` and `rxing`/ZXing-compatible reader | Versioned migration/recovery, bounded decoding, no automatic URL navigation; QR writer must not require a service |
| Native recording/system tools | Documented Windows Graphics Capture, WASAPI, camera/scanner, WLAN, power, shell, battery and monitor APIs | Use native capability/permission checks; keep device handles and reversible system changes under native ownership |

Engine management has a visible Installed/Available/Update/Remove state and an explicit install action showing size, publisher, license and enabled tools. Verify downloads and safe extraction before atomically switching versions; retain the previous working version for rollback. Uninstall refuses active jobs, removes only owned binaries/models and preserves user results. Refresh eligibility without blocking ring opening. Models and executable engines are never community-pack payloads.

Do not trust whichever executable appears first on PATH or in the current directory. If allowing the user's installed engine, use a native picker, display its identity/version and permission implications, pin the approved canonical path/hash and revalidate before execution. Shared-library lookup and writable install directories are part of that review.

### 13.12 Implementation sequence and ownership

Execute in this checkout without branches, commits, builds, app launches or deployment unless separately instructed. Extend existing owners before introducing modules; split focused native handler modules from `src/actions.rs` as they become real. The following paths describe planned ownership, not files already present. Each stage must leave working tools and source checks, rather than enabled buttons with no handler.

1. **Registry, jobs and engine foundation.** Extend `src/actions.rs`, `src/output.rs`, `src-tauri/src/shell.rs` and `web/native.ts`; introduce `src/jobs.rs`, `src/engines.rs`, `assets/engines.json` and `web/Engines.tsx` when implementing their actual responsibilities. Add typed input cardinality/fields, engine/capability IDs and user-facing availability reasons. Replace parent-only worker termination with Windows Job Object process-tree ownership, cancellation and cleanup records. Test stale revisions, duplicate requests, missing/changed engines, bad hashes, failed install/rollback, hostile extraction, child-process cancellation and no publication after cancellation wins. Preserve the one-operation lease and no-replace output invariants.
2. **Tools discovery and image workspace.** Add `web/Tools.tsx`, `web/tools.css` and focused image editor components; integrate with `web/main.tsx`, `web/Icon.tsx`, `web/menus.ts` and the existing native registry. Reuse source previews and existing transforms for T22/T24–T28, baseline crop/watermark/compare/contact-sheet improvements. Test numeric/pointer equivalence, orientation, alpha, invalid crop, aspect ratios, batch incompatibility, overflow allocations, no-shrink output and collision safety. T21/T23 remain honestly needs-model until their inference package passes stage 8.
3. **PDF structure and document workspace.** Introduce `src/actions/pdf.rs`, `web/PdfEditor.tsx` and shared page-range parsing. Deliver T35–T40/T42–T44 plus inspect/lossless optimization first, using qpdf and bounded PDFium previews. Then implement T30/T31/T34/T41 behind separate acceptance gates. Test encrypted/malformed/huge PDFs, ordered/descending/open-ended ranges, duplicate/out-of-range pages, rotations, annotations/forms, digital-signature warnings, password privacy and redaction recovery attempts. Do not claim secure redaction from a screenshot or text search alone.
4. **Media workspace and recording.** Introduce `src/actions/media.rs`, `src/platform/recording.rs` and `web/MediaEditor.tsx`. Probe once per captured revision; deliver T03/T04/T06–T16/T18–T20 through shared typed FFmpeg transforms, then T02 after subtitle support. Implement T05/T17 with native capture and device cleanup; T01 waits for the download policy and exact yt-dlp/runtime package. Test streamless inputs, variable frame rate, keyframe vs precise trim, codec fallback, A/V sync, mismatched merges, safe subtitle paths, output caps, device revocation, disk-full and cancellation at publication.
5. **SQLite, Library and boards.** Introduce `src/storage.rs`, `src/collections.rs`, `web/Library.tsx` and collection IPC owned by the shell. Deliver baseline 81/83–90 with the data rules in 13.9; retain 82 as needs-browser until stage 9. Test schema migration/backup recovery, preference import failure, duplicate asset references, external-original deletion, failed DB insert after asset staging, cleanup retries, concurrent imports, quota exhaustion, escaped exports and deleting a shared asset from one board only.
6. **File/archive breadth and Windows utilities.** Implement T56/T57 and the remaining baseline file/ZIP operations first through the shared output/journal owners. Add `src/actions/system.rs` and `web/SystemTools.tsx` for T58–T72 in verified groups: read-only diagnostics; bounded reversible power/display/settings controls; then reviewed cleanup/uninstall/ejection/input features. Test reparse points, rename cycles/case/collisions/undo conflicts, denied access, Recycle Bin failure, device disconnect, power expiry/crash, keyboard watchdog, system-volume exclusion and rollback after partial setting changes. Do not ship input suppression before its recovery proof.
7. **Document conversion and OCR.** Introduce `src/actions/documents.rs`, shared extraction/OCR services in workers and `web/DocumentEditor.tsx`. Deliver T29/T32/T33/T45–T50/T53/T54 with explicit engines and supported format directions. Test document macros/external links, hostile embedded resources, font substitution, Office-session isolation, sheet pagination, slide notes, scanned reading order, language absence, HTML file/network boundaries, Markdown escaping and long-page caps.
8. **Optional local models.** Add `src/actions/inference.rs` and explicit model controls for T21/T23/T51/T52/T55. Share extraction and media conversion; keep each model package independent. Test corrupted/hash-mismatched models, unsupported languages, missing faces/subjects, masks, inference RAM/time limits, cancellation, long-input chunking, SRT timing and no network during inference. Benchmark actual model quality/cost before marking ready.
9. **Browser and online workflows.** Complete the baseline extension/bridge with profile/tab/frame identity validation and explicit permissions. Deliver baseline 61–70/82 and web inputs to boards, T49 and T01. A directly pasted download URL does not require extension installation. Test stale tabs/profiles, permission denial, restricted pages, redirects, local/private-network targets, maximum download size, partial recovery and explicit source attribution. Review any authenticated download flow separately; do not copy browser cookies by default.
10. **Packs, hub and release evidence.** Keep baseline milestones 6–7: declarative bounded packs, permissions/version diffs, public submission/moderation, clean-Windows installer/update/uninstall, licensing and measured resource budgets. Extend pack validation to new handlers/capabilities without allowing engines, scripts, secret access or system-change escalation. Deploying the hub remains outside this implementation instruction.

For each group, run the smallest targeted unit/integration tests and source type/lint checks. Record exact passed commands in verification.md and completion bullets in the daily task file only after work lands. A checked plan row must name the handler, callable UI path, tested formats/dependency state and remaining native acceptance boundary. The original unchecked milestones are historical requirements, not an accurate progress dashboard; this current inventory is the starting point for updating them as each stage passes.

### 13.13 Release and verification boundaries

Do not promise full-suite completion in a single release or silently turn the original 100-action goal into a measured coverage claim. Count distinct tools from the registry and separately report supported variants, format pairs and installed engines. The 72 workflow identifiers are coverage requirements, not an implementation count.

Before a tool becomes ready, prove its real handler and frontend route, eligibility reason, malformed input handling, permissions/engine path, original preservation or reviewed system mutation, cancellation/cleanup, output naming, keyboard support and applicable license notices. A rich editor is incomplete if it is only a visual mock or opens an external application instead of carrying out the operation.

Agent-run checks remain TypeScript, formatting, lint and targeted tests. Do not run builds, development servers, apps or the full suite under the current instructions. User-run app command after implementation: `pnpm tauri dev`. User-run executable build: `pnpm tauri build --no-bundle`. Signed package command after signing setup: `pnpm package:windows`. Native keyboard/focus, Narrator, mixed DPI/text scaling, codec/form fidelity, hardware controls, recording consent/device release, real latency/memory and clean-Windows acceptance cannot be established from source alone.

### 13.14 Contextual web downloads and image actions — 21 October 2026

The default video workflow is: a YouTube video is playing in the browser → invoke PieAll over that page → choose Download video → the download starts in the background using saved defaults. No URL copying, dashboard navigation or format dialog is required when the browser bridge identifies one supported target and the engine is installed. The same flow applies to X posts and other yt-dlp-supported platforms. Playback itself is not required if a supported video page or explicitly targeted post/link identifies the media. “Any video” means supported accessible media; unavailable, access-restricted, DRM-protected or unsupported sources get a specific failure, never a screen recording silently substituted as a download.

**Target capture and action placement**

- Extend the planned browser bridge with invocation-scoped page URL/title and an explicit image/link/video/post target, including browser profile/window/tab/frame identity. Capture before PieAll changes focus. A browser executable name, window title or previously active tab is insufficient to choose the URL.
- For a single-video watch page, Download video acts on that video. On feeds such as X, a media element, post or link explicitly targeted by the invocation identifies its canonical post/media URL. Prefer that target over the feed URL. Never guess from whichever video happens to be playing elsewhere, download every feed item, or select media in another tab/profile.
- If several videos belong to the captured post, show a small numbered picker with thumbnails and durations; optionally offer Download all in this post. If no unique target can be captured, ask for a target or pasted link instead of downloading a guessed URL. Missing bridge permission/connection offers its setup route and an explicit paste-link fallback.
- Eligible contextual actions appear directly in the relevant ring, with Download video as the primary video action and Download audio as an alternative. Images expose Save image, Copy image and Add to board; links/pages expose their relevant save/copy/bookmark/reading-list actions. Reuse existing ring customization and overflow rules. Ordinary browser shortcuts remain reachable without displacing the explicitly selected media tools.
- Run against the reviewed immutable snapshot; reject a stale response before starting. After a download starts, changing tabs or closing the ring does not retarget or cancel it. Do not scrape tabs or poll playback while idle.

**Download defaults and customization**

- Use maintained yt-dlp as the downloader and FFmpeg for compatible stream merging/post-processing. Default to yt-dlp's best available video and best available audio, with a best combined-stream fallback where needed. Separate high-resolution video/audio streams must be considered; selecting only a precombined stream must not silently cap the quality.
- Best available has no default resolution cap and no forced H.264/MP4 transcode. Choose a container compatible with the selected streams; preserving source quality takes precedence over a familiar extension. Remux without re-encoding where possible. If FFmpeg is required but missing, offer its explicit installation or a clearly labelled lower-quality combined-stream choice; never silently downgrade.
- App Settings → Downloads owns persistent defaults: Best available or a resolution cap, preferred container, compatibility/codec preference, original audio or an explicit output format/bitrate, subtitle/metadata choices, web-download destination and a policy for handling playlists. Defaults are best available, original audio without lossy re-encoding, no embedded subtitle/metadata extras, single targeted item, and the user's Downloads folder. A playlist URL does not download the entire playlist unless selected explicitly.
- Add optional Download options in the app/ring to override one job, inspect available formats and choose video-only/audio-only or a supported combination. Ordinary Download video uses saved defaults immediately. User preferences that cap quality or request a transcode remain visible in the job summary; distinguish Highest quality from Most compatible presets.
- Download audio defaults to the highest available audio stream without an unnecessary lossy re-encode; a saved MP3/M4A/etc. conversion preference is explicit. Show the actual source/output codec and any quality-changing conversion.
- Use controlled names derived from media title/identity and collision-safe publication; sanitize titles as filenames. Stage video/audio/merge files privately and publish only successful outputs to the configured folder. Provide bounded resource/disk budgets and engine-specific job deadlines appropriate for long downloads; do not apply the current local-transform 120-second timeout unchanged to a large download. Downloads use the shared native job owner and exclusion rules; do not spawn a second unmanaged downloader beside it.
- The ring closes normally. A non-activating status/toast confirms start/completion/failure; the app's Jobs view shows title/source, actual resolution/codec, bytes/progress when available, processing/merge state, Cancel, Retry, Open file and Reveal. Unknown size/progress stays indeterminate. Do not steal focus, autoplay or open downloaded files automatically.
- Install/update yt-dlp, its required runtime and FFmpeg through the explicit engine manager. Site changes may require a verified engine update and a retry. Authentication-dependent support needs a separate explicit account/session grant; never silently read browser cookies. No DRM bypass or global security-setting changes.

**Save/copy images and related web actions**

- Save image targets the explicitly selected/context-targeted image. Resolve the best page-provided original/full-resolution candidate when deterministically available and permitted; otherwise use the selected resource's actual resolution. Handle picture/srcset and lazy-loaded images without inventing an original URL or claiming preview pixels are full resolution.
- One action saves to the configured web-download folder using the shared safe writer and collision naming. Save image as provides an optional native destination/format editor. Preserve the original supported image bytes/format by default after validation; format conversion, metadata stripping and recompression are separate explicit choices. Do not upscale to manufacture a higher-resolution result.
- Copy image fetches and bounded-decodes the same target into clipboard pixels. Add to board imports permitted bytes into the managed collection store and records source page/image URLs and capture time; it does not rely on a fragile remote-only thumbnail. A default board can make this direct; otherwise use an accessible board chooser/create flow.
- Cross-origin access requires the bridge's declared permission. Authenticated/blob/canvas resources may be inaccessible: use an explicit supported page-context export when available or explain the restriction. A screenshot fallback is a distinct user-selected action. Animated images retain original bytes when saved; pixel copying/editing requires an explicit frame choice when applicable. SVG content is saved/rendered with its existing active-content/external-resource restrictions.
- Copy image URL, Copy page/link URL, Copy clean URL, Copy Markdown link, Bookmark, Save for later and Save quote with source share the captured browser identity and existing baseline handlers. Download a linked file only for an explicit link target and supported scheme; the URL must not be executed as a command or opened automatically.

**Implementation and proof**

Implement this as the first vertical browser/media workflow in stage 9, reusing stage 1 engine/job ownership and stage 4 media probing. Planned owners: `extension/background.ts`, `extension/content.ts`, `extension/native.ts`, `src/bridge.rs`, `src/actions/browser.rs`, `src/actions/media.rs`, `src/engines.rs`, `src/jobs.rs`, `src/storage.rs`, `web/menus.ts`, `web/Tools.tsx`, `web/MediaEditor.tsx` and the settings/jobs UI. Settings remain under the versioned native store rather than an independent frontend download-preference store.

Targeted tests must prove active-tab/profile/window matching; stale snapshot rejection; single video vs playlist selection; a targeted X post inside a feed; multi-video ambiguity; best separate streams plus audio selection; missing-FFmpeg no-silent-downgrade behavior; resolution/codec preference overrides; URL/argument/path validation; malicious media titles; redirects and private/local-network handling; size/disk/time limits; cancellation during download/merge/publication; partial-file cleanup and collision-safe retry. Image fixtures cover srcset/lazy loading, unavailable originals, permission denial, corrupt/huge images, animated input, SVG restrictions, clipboard decode and board provenance. Live platform/browser/download acceptance remains user-run and must record tested sites and engine versions rather than assert universal support.


### 13.15 Implementation checkpoint ? 21 October 2026

The registry contains 66 local command IDs. This checkpoint records implemented code; native UI and real-codec acceptance remain open. It does not mark all 72 workflows or the original 100 product IDs complete.

| Area | Implemented now | Still required by the plan |
| --- | --- | --- |
| Jobs | Worker file counts, duration-based media encoding progress, Close/Cancel, partial outcomes, no-replace publication, Windows process-tree/memory ownership | Durable history, retry/open/reveal actions, crash recovery/cleanup, richer phase/ETA/size reporting |
| Images | Existing transforms plus crop drag/presets, resize modes, gradient/ratio/shadow framing, quality controls | Layers, masks, background removal, advanced adjustments, annotation and broader codecs |
| PDF | Images-to-PDF page controls; inspect, extract, split, rotate, stream compression, basic document metadata removal, selectable text export | Merge/reorder/page thumbnails, annotations, password UI, form editing, OCR, secure redaction and broader document conversion |
| Media | 15 optional FFmpeg/FFprobe handlers for conversion, compression, trim, resize, rotate, mute, GIF/frame/audio extraction, volume, single-pass normalization and inspection | Timeline/waveform editors, stream/track selection, recording, subtitles, concatenation, further catalog workflows and real-codec acceptance |
| Engines/settings | Native-selected FFmpeg/FFprobe paths and pinned hashes in schema-1 SQLite; missing-engine guidance | Verified install/update/removal of managed binaries, dependency/license/version UX, persistent conversion/download defaults and further engines |
| Archives | File/folder ZIP with empty directories, Store/Deflate level, bounded traversal/streaming | Other archive formats and remaining file-manager workflows |
| Browser/downloads | Planned behavior and acceptance requirements in 13.14 | Browser extension/native host, exact tab/frame/post capture, yt-dlp/runtime management, highest-quality background download, settings and web-image tools |
| Boards/Library | Product and interaction specification | Storage/assets, collection/board UI, notes/tags/provenance, canvas/grid, imports and exports |
| Remaining catalog | Extracted 72-workflow requirements retained above | Speech/model tools, recording, advanced Windows utilities and other unimplemented rows |

The current setup deliberately uses trusted user-installed FFmpeg/FFprobe instead of an unfinished automatic installer. Media jobs use their own one-hour deadline rather than the earlier 120-second image/file limit. The SQLite database currently owns engine records only; ring preferences retain their existing store until the planned migration. No browser or board implementation is implied by the engine/media work.
