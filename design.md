# PieAll design
Author: Nitish

## Release-note formatting - 23 October 2026

Updates shows formatted release notes with bold emphasis, compact headings, lists, paragraphs and code, superseding the plain-text notes described below. The existing theme tokens, 180 px scroll area and wrapping of long URLs remain. Links display their destination as selectable text; remote images and HTML are not rendered. Static rendering and type checks cover this change; native appearance remains unverified.

## Custom app profiles - 23 October 2026

Customize ring places Add app above the app search. User-created entries carry a Custom label; selecting one reveals Edit app beside Add app. The form occupies the existing preview/editor area, preserving the app rail. It offers a native executable picker, an app name and labelled Action/Keys rows, Add shortcut, Save app and Cancel. Saved names and keys remain editable; removing an app uses an inline confirmation. Errors are announced inline and keep the draft. Cancel discards unsaved edits. Navigating to another dashboard section preserves the draft until Customize ring is reopened in the same session. Rail app/type navigation is disabled during editing. The form scrolls independently, follows neutral light/dark tokens and existing focus styling, and stacks shortcut fields at narrow widths. Native picker and actual keyboard dispatch still require user-run acceptance.

## Dia browser - 23 October 2026

Dia appears by name in Customize ring's app search and uses the existing browser ring defaults, native app icon with a monogram fallback and keyboard-accessible customization controls. Saved Dia layouts follow the same per-app behavior as other browsers.


## Direct browser captures - 23 October 2026

The default browser ring has eight direct slices: Download video, Download audio, Copy URL, Reopen closed tab, New private window, Capture current, Capture full page and Window. Capture current replaces Paste plain text; Capture full page replaces Search actions. Both capture choices use the existing camera icon, labels and keyboard/hold interaction, with no screenshot submenu. Custom layouts retain their selected actions. Other apps keep Search actions, and slash remains the search shortcut.

Capture current copies the visible page viewport to the clipboard and reports Current page copied to clipboard. Capture full page remains a direct default slice in Chrome as well as Edge/Firefox. Chrome scrolls the page automatically, captures overlapping views and restores the starting position before reporting completion. Keep the page still during capture; Escape or changed keyboard/mouse-button input stops it. Small results report Full page copied to clipboard; large results are saved as PNG under Downloads/PieAll Screenshots, with the saved path retained in the dashboard result. Neither route automatically pastes the image.

Edge and Firefox continue to open their native screenshot preview for Copy/Save. No screenshot action opens DevTools, installs an extension or changes device emulation. Errors use the existing notice and dashboard status, preserve the clipboard when no complete image is published, and do not claim a partial capture succeeded. Sticky content and animations can still affect stitched results. Native appearance, pixel output, document targeting, restoration, focus and clipboard behavior remain unverified.


## Parallel jobs - 21 October 2026

The dashboard shows jobs as compact in-app toasts in its lower-right corner, above the status footer across all pages. The stack overlays the workspace without shrinking the ring or action browser, and scrolls within half the workspace height, capped at 480 px. Each notice names the action and state, shows progress and Cancel while running, and retains an expandable, selectable View result afterward. Successful downloads retain Open location. Close dismisses only that in-app notice for the current dashboard session without cancelling work; its later updates remain hidden. Notices remain until closed or removed from bounded native history. Long names, paths and controls wrap, and identical title/detail text is shown once. Toasts use existing neutral theme tokens, typography, Lucide icons and keyboard focus styling. Cancellation and progress labels include job identity. Full result text loads when its disclosure opens.

Downloads return their form after job acceptance with a short started message, allowing another URL immediately. Background work leaves file selection, ring actions and other tools available. Short foreground/clipboard exchanges still show the existing temporary busy state. A stale report stays in Jobs instead of copying/pasting over a newer selection. Floating progress updates keep the current job notice stable; another job's completion may replace it, while all work remains available in the dashboard. Close hides only that job's notifications. Update checks and downloads also leave ordinary actions available; installation and exit still wait for running jobs. This supersedes the earlier single-job footer behavior for the Tauri shell; historical GDI behavior is unchanged.

The in-app toast stack is checked through source, static-render regressions and type checks. Native layout, focus, screen-reader announcements and simultaneous real downloads still require manual acceptance.


ZIP extraction uses the existing job notice and dashboard progress bar, showing byte-based percentage and the current entry during longer work. The existing 700 ms delay, dashboard foreground behavior, Close and Cancel controls apply; short extractions can finish before the progress notice appears.

## Browser downloads - 21 October 2026

The sidebar opens a focused Browser downloads page in the existing warm dashboard style. A labelled URL field leads into video/audio mode, relevant format controls, a visible destination and Download/Save as defaults. Native folder selection keeps paths out of editable web parameters. Required tools set up on first use through the same progress notice. Form controls remain keyboard accessible; setup and download errors are inline alerts. Forms briefly disable during job acceptance or settings changes; the Jobs list and toast retain per-job Cancel while work continues. Unknown single-item progress uses an indeterminate bar rather than a guessed percentage; closing its toast does not stop the download.

The browser ring starts with Download video and Download audio, then four familiar browser shortcuts. The normal action copies the current browser address and uses saved defaults. The dashboard explains opening the exact video/post page when viewing a feed and retains a paste-link fallback. No extension or connection setup appears. Results preserve saved paths and source/output details for copying.

Download failures distinguish an actual output-size limit from a failure to inspect temporary output; inspection errors include the underlying cause.

## Implemented editors, engine setup and job notices ? 21 October 2026

Tool setup keeps the warm visual language and uses a compact summary with one Set up tools button, two plain status rows (Video & audio, Browser downloads) and an Advanced disclosure. Ready and Sets up automatically describe user-visible availability. Manual file paths, individual executables and recorded hashes stay inside Advanced. Setup progress, errors, Close and Cancel reuse the existing job surface. The layout stacks controls on narrow windows and retains labelled keyboard controls.

Action editors use native registry choices, hide controls irrelevant to the selected mode and preserve numeric keyboard alternatives. Crop adds a draggable SVG selection and centered aspect presets; resize previews dimensions without upscaling; frame previews show the selected gradient, canvas ratio, corners and shadow. PDF tools use page-range inputs and explain forms/signature, redaction and OCR limits. Media uses focused numeric/select controls; timeline, waveform and PDF thumbnail editors remain planned.

While PieAll is active, job progress stays in the dashboard toast stack with per-job Cancel and no floating duplicate. In the background, notices anchor to the invocation monitor's bottom-right work-area corner, inset 16 DIP above the taskbar; updates never follow the cursor or take focus. Returning to PieAll hides that job's floating notice while retaining its status. Job notices use PieAll's existing inline toast surface with an explicit Close button, a progress bar and Cancel while running. File counts come from the worker; known-duration media also shows the current file's encoding percentage. Progress updates retain the same visual identity rather than replaying entrance animation. Close dismisses the job's notices while work and clipboard behavior continue. Cancel changes to Cancelling and keeps completed outputs. The dashboard footer also exposes progress and Cancel. Native notices remain nonactivating, so keyboard/screen-reader acceptance must include the dashboard alternative. Complete/partial/failed/cancelled are distinct states; longer failure details remain available in dashboard status/results. Native clipping, pointer hit testing, high DPI and focus still need manual acceptance.

## Planned tools and Library surfaces — 21 October 2026

Contextual web tools use saved defaults immediately: Download video on a supported captured video page/post starts a background highest-quality download, while Download options opens the optional override editor. Save image and Copy image act directly on an explicitly targeted web image; Add to board uses the saved board or a board chooser. Show a compact target picker only when media is ambiguous, and a paste-link recovery when native browser URL copying is unavailable. Completion does not steal focus. Jobs expose progress/merging, cancel/retry and open/reveal; Settings → Downloads holds persistent quality/format/audio/destination choices. No runtime implementation is implied by this planned UX.

The expanded toolkit preserves PieAll's existing warm palette, Lucide icons, compact title bar and contextual ring. A planned Tools discovery view adds All, Video, Audio, Images, Documents, Text & Speech, Files, Windows and Boards, with searchable descriptions, meaningful previews, registry-derived counts, favorites and recent tool IDs. Inspiration screenshots guide the category/discovery pattern and focused editor behavior; they do not replace PieAll's identity or supply reusable artwork. No runtime UI changes are part of this plan update.

Tool pages follow Input → Preview and settings → Run → Result. Rich editors pair a preview with an inspector: timelines/waveforms for media, page thumbnails for PDF organization, canvas/mask controls for image edits, and a grid with optional spatial arrangement for boards. Every pointer operation has labelled numeric or keyboard controls. Category discovery works without a prior file selection; native input picking still owns captured targets. Active edits cannot disappear through navigation without cancellation or a reviewed discard.

Show the actual input order, output policy, format/engine availability and per-item results. Preserve sibling outputs by default; downloads, recordings and exports have explicit destination controls. Distinguish estimated size from completed compression savings. Cover empty, unavailable, no-change, cancelled and partial-success states as well as success/failure. The Library retains notes, tags, provenance, missing-source feedback, bounded thumbnails and private-field export review. Keyboard access, visible focus, high contrast, text scaling, reduced motion and undo for uncommitted edits apply throughout. Detailed tool controls and delivery gates are in section 13 of PieAll-plan.md.

## Custom title bar — 21 October 2026

The dashboard is frameless. Its 44px header is the title bar: brand at left, Show ring, then Minimize, Maximize/Restore and Close flush at the right edge (46px wide, full height, Close turns red on hover). Empty header space drags the window and double-click toggles maximize. Close hides PieAll to the tray, as the native button did; its label says so. Window controls are real buttons with accessible names.

## Icons — 21 October 2026

Action and interface icons come from Lucide (`lucide-react`, named imports only, so unused icons are tree-shaken). `web/Icon.tsx` holds one name-to-icon map for the UI, file tools and app commands. App commands get their icon from `iconFor(title)`, which matches title keywords in priority order (Save, Undo, Bold, Zoom, Layers, Tabs and so on). Anything it cannot place shows the generic Command key, as does any unknown icon name. Stroke width is 1.7 everywhere. App tiles use locally extracted Windows app icons when available, with stable colored monograms as the fallback. Artwork stays contained in the existing tile and is decorative beside the app name.

## Tray shortcut control — 21 October 2026

The native tray menu orders Open PieAll, Show action ring, Use shortcut, and Exit PieAll. Use shortcut uses a native checkmark, checked when keyboard invocation is enabled, and supports the native menu's keyboard interaction. Show action ring has no hard-coded key hint and remains usable when shortcuts are paused. A failed toggle restores the checkmark to the actual state.

## Dashboard structure and customization — 21 October 2026

The dashboard fills the window and never scrolls as a whole; the sidebar, lists and panels scroll on their own. Header and selection bar are compact, with no taglines. The sidebar groups Actions and Personalize, with Updates at the bottom. The empty state shows three numbered steps and two buttons.

Customize ring is a fixed three-pane view. Left: hold-key chip with Change, the active app pinned first, the four file types, and a searchable list of all apps with native app icons (with stable colored monograms when Windows cannot resolve an icon); a dot marks customized rings. Center: the full ring exactly as it will open (same wedge shapes, icons, labels, More slice for overflow past eight), with a dashed ghost slice to add into and the app or type in the hub. Clicking a slice selects it. Right: tabs Slices (drag to reorder, plus move and remove buttons, selection synced with the ring) and Add action (searchable list). Below 860px the right pane moves under the ring.


## Contextual shortcut browsing — 21 October 2026

The existing dashboard palette and neutral glass floating ring remain. For a recognized app in window or selected-text context, the ring keeps six curated app actions primary, with Search actions and Window tools available. Selected text adds a secondary Text tools submenu. Figma shows Copy/Paste properties, horizontal/vertical flips, Create component and Auto layout, then Align and More. Align contains all six directions plus horizontal/vertical distribution; More contains Search actions and Window. Search actions opens the flat dashboard Apps list and focuses its input; `/` from the ring also opens search. Search remains flat; Align is a focused workflow group. Selected files/images keep their relevant selection tools. All submenus reuse the existing keyboard and hold interactions, and overflow moves into More. Saved app layouts remain authoritative, with Align available in Customize ring. App labels wrap to at most two 16-character lines, with ellipsis for longer names; full names and key hints remain accessible in SVG titles. Menu resolution and wedge paths remain memoized.

The dashboard only shows shortcuts for the captured app. There is no All apps selector or disabled catalog of unrelated apps. Apps explains the captured context and default keymap; an unsupported app gets guidance to invoke PieAll from a supported app. Search matches command/category words and whole chord tokens. Input updates immediately; deferred results expose aria-busy. At most sixty rows render per page with accessible Previous/Next controls. Changing app resets the query and page. Selection tools remain available through the existing categories. Short-window scrolling, keyboard support, reduced motion and focus outlines remain intact.

Native WebView2 focus/input, screen readers, alternate layouts, mixed DPI and actual frame/latency performance remain user-run acceptance gates. Static source checks do not establish native smoothness.

## Brand identity — 21 October 2026

PieAll keeps its name and “A little less work.” tagline. The mark is a six-segment radial menu on a neutral graphite `#1C1C1E` tile with 19/80 corner radius. Five off-white `#F4F4F6` segments surround an open hub; the upper-right segment is orange `#FF8A3D` (orange), representing a selected action. The geometry echoes the separated action wedges without adding decoration to the floating ring.

The dashboard header uses the mark at 40 px beside the existing wordmark; the connection screen uses it at 48 px. The wordmark keeps Segoe UI, weight 650 and -0.03em tracking. Decorative marks use empty alt text beside the visible product name. Existing keyboard interaction, warm dashboard tokens and neutral glass ring/toast styling are retained.

`public/brand/pieall.svg`, the Windows multi-size ICO (16–256 px), 512 px PNG and 32 px tray RGBA share geometry and colors generated by `python scripts/generate-brand.py` (Pillow required). The dark tile and cream segments give the tray mark contrast on light and dark taskbars. The same SVG supplies the document favicon and README mark. Native shell rendering and taskbar appearance still require Windows acceptance.

## Floating ring and toast — 20 October 2026

This supersedes the floating-ring parts of the section below; the dashboard look is unchanged (still warm, now secondary).

- **Gesture.** Press ` and the ring appears centered on the cursor on key-down; a quick tap (under 150 ms) types a backtick instead and the ring vanishes without a fade. Sliding toward a slice selects it by angle, so a flick past the rim still counts; the selected wedge pops 5 units outward. Releasing runs the highlighted slice. Releasing anywhere else, including the center (82-unit dead zone, matching the hub) or a submenu slice, closes the ring. Resting 550 ms on a submenu slice opens it, with a 3 px dark progress bar on a faint track just inside the white wedge's outer edge (r 180), clear of its label. Inside a submenu, resting 550 ms in the center goes back.
- **Look: neutral glass tray with pillow slices, no warm tones (21 October 2026).** A tray disc (r 200, `rgba(16,16,18,0.78)`, top-lit 1.2 px rim) holds the slices, so gaps read as tray rather than desktop. Slices are pillow-shaped annular sectors (r 96–190) with parallel 9-unit gaps and rounded corners (22 outer, 14 inner): raised panes `rgba(52,52,57,0.9)` with a 1 px edge lit from above (white 55% top, 14% mid, 28% bottom), a faint sheen and a tight drop shadow. The selected slice is solid light `rgba(248,248,250,0.96)` with `#141416` ink. A recessed well ring (r 89) surrounds the hub. The hub (r 82) is dense glass `rgba(38,38,42,0.94)` with an inner top highlight, holding the thumbnail or kind icon, the target name and a one-line caption (the selected slice's hint, "Rest to open", "Center to go back", or "Esc" in click mode). Ink is `#F4F4F6`, 12 px labels. There's no native blur, so tray and panes stay dense enough to read over desktop icons and bright wallpaper. The native window drops its caption/frame styles and centers its elliptic region on the client area (not the outer window), so no frame band shows as a crescent above the top slice. Windows 11's accent border on active windows is turned off for the ring and toast (DWMWA_BORDER_COLOR = none), since clicking a slice activates the ring and that border showed as a pale blue arc.
- **Motion (speed first, 21 October 2026).** The ring is usable on its first frame. There is no loader: while the selection is being read, the ring shows the slices this app settled on last time (kept in memory per app), so in a repeat app the real result usually matches and nothing moves. A first visit shows the app's window menu. Hover works at once; releasing on an app shortcut or window slice during the read runs it at once (the read is cancelled and the clipboard restored first); any other release waits for the real menu and picks by cursor direction. Entrance: 70 ms fade with a 90 ms scale from 0.97, no stagger. Exit: 60 ms fade; the native window hides only after it, so a re-shown ring never flashes a stale frame. Changing level swaps the slices with a 120 ms fade and scale from 0.95. Reduced motion removes all of it.
- **Slices.** Up to eight, sector zero at twelve o'clock, clockwise: a 22 px icon over a 12.5 px semibold label. Conversion to the selection's own format is not offered.
- **Keyboard.** Arrows and Tab move a roving selection; Enter or Space runs it; 1–8 run directly; Backspace or Escape leaves a submenu; Escape closes and returns focus to the source window. Slices are `menuitem`s in a `menu` named for the target; submenus carry `aria-haspopup`.
- **Toast.** Quiet and out of the way: a 40 px near-opaque dark pill (95%, so it stays readable on bright wallpaper) in the bottom-right corner of the work area on the ring's monitor, never focused. A progress toast is 320 px wide and reads top to bottom: title / what is happening ("Downloading · 42%", with "3 of 8" for batches) / a 4 px bar that eases between ticks (a sweep when the amount is unknown) with Cancel beside it. Progress ticks only repaint the toast; the window is never moved or resized mid-job. A small status glyph (green check, coral alert, neutral info, spinner), a semibold title with a muted detail beneath it that wraps to two lines rather than cutting mid-word: "Saved prot-compressed.jpg" / "66% smaller". It springs in over 180 ms, stays 1.4 s (3.2 s for errors) and fades out over 120 ms. There's no startup toast; one appears only if the hold shortcut can't be installed. The progress bar, count and Cancel appear only for progress work (several files, or one file whose tool reports progress: converting, compressing media, PDF work) still running past 700 ms. One-shot actions such as Terminal here or Copy path never show it, only the result notice; once it finishes the same pill turns into a plain result notice and leaves on the normal timer. The close button shows only on running and error notices. Screen readers may not announce it; the dashboard status line remains the accessible record.

## Current Tauri design — 20 October 2026

The default shell follows Nitish's radial-menu references: rounded separated wedges, warm translucent fills, a quiet center target, local stroke icons and a distinct selected sector. This section and docs/tauri-port.md supersede the historical GDI record below.

Ivory #F5EFE5 page, cream #FCF7EE controls, warm ink #342B23, secondary #756453, divider #DFD3C2, amber wedge #F4D6AF, active wedge #C66A35 and accent #9B4F29. Floating wedges use alpha peach fills and a subtle rim, without native acrylic/Mica or backdrop blur. Segoe UI Variable Text/Segoe UI needs no downloaded fonts; local SVG icons share a 1.7 stroke. web/tokens.css owns tokens. Controls include focus outlines, restrained hover/press states, themed selection/scrollbars, reduced-motion handling and forced-colors sector styles.

The floating ring is 440 DIP with a circular native hit region. SVG sectors use a 440-unit viewBox, 196 outer radius and 82 inner radius, rounded on both edges. Sector zero is centered at twelve o'clock, clockwise, at most six visible; excess actions become All actions after five contextual choices. Convert drills into PNG/JPEG/WebP only, with center/Backspace to return. JPEG uses white behind alpha; its full editor allows another background. Unsupported reference formats are not presented as working features.

The center shows thumbnail/icon, bounded target name/count and an Escape hint. Hover selects; click executes. SVG groups expose button names/roles. Floating arrows/Tab use roving focus, Enter/Space executes, 1–6 activates and Escape backs out/closes. Dashboard Tab remains document navigation. Native focus loss dismisses the floating ring; explicit dismissal restores source focus when its PID matches. The embedded center opens file selection.

Dashboard default 1040×740 DIP, minimum 640×540: brand/Show ring header, category/count sidebar, target/Open files/explicit clipboard row, contextual ring and searchable action list. Narrow layouts visually collapse category text while retaining accessible names. Search supports / outside inputs. Unsupported rows are disabled with eligibility hints. Loading/busy, empty selection, failed capture, connection errors, no matches and action errors have explicit states. A persistent live status line reports progress; selectable results and Copy feedback appear below the ring.

Parameterized actions replace the list with an inline editor. Labelled numeric/text controls remain keyboard-accessible; first-field focus, Enter submit and Escape/Cancel return are provided. Previews approximate crop bounds/invalid rectangles, resize dimensions without upscaling and frame padding/color/corners. Native validation applies to every selected input. Decode failure leaves the editor usable. Updates show accessible plain-text notes, progress, check/download states and separate inline install confirmation. Confirmation names the reviewed version and is rejected if that version changes before installation.

Actual React source was statically rendered at 1040px with an illustrative selection. Native transparency, hit regions, WebView2 keyboard behavior, screen-reader exposure, focus restoration, mixed DPI, text scaling and small work areas remain unverified. The dashboard must remain the keyboard/screen-reader alternative. There are no modifier-drag invocation, pointer crop handles, configurable shortcuts or recent-use ranking yet.

## Historical GDI design (legacy-ui only)

The following applies only to the explicit native fallback. An exact pre-port copy is retained in docs/native-baseline-design.md.

How the native Windows UI (`src/ui.rs`) looks and behaves today. It is a GDI first pass, not the Direct2D target in the plan.

## Visual language
- Warm ivory and orange. Base `#F3EBDD`, wedge `#F7D2AE`, selected wedge `#C94F22` with `#FFF8EE` text, ink `#29251F`, rim `#D9C6A8`, wedge edge `#FFF8EC`.
- Type is Segoe UI: 14 px labels, 14 px bold for the target name, 11 px for hints.
- Dashboard, editor and result windows use the same ivory base and ink text, with standard native controls.

## Action ring
- Circular window, 320 DIP across, clipped with a window region. Center circle is 104 DIP across. Up to six separate wedges with 6 DIP gaps, 9 DIP rounded corners (GDI polygon with quadratic corner curves, not antialiased) and a thin outer rim.
- Wedge 0 is centered at 12 o'clock and wedges run clockwise. Hit-testing is unit-tested.
- The center names the target ("photo.jpg", "4 items", "Folder: Research", "Clipboard text (N chars)", "Active window") and shows a small hint ("Esc closes", "Drop files here", "+N in dashboard"). Clicking the center closes the ring.
- Ring choice is ranked by context, not catalog order. Images: Convert, Crop, Resize, Frame, Compress. Files: when the selection fits only one file type (all PDFs, all media), that type's tools are the first slices instead of a "PDF tools" / "Media tools" detour; then Copy paths, Create ZIP, Extract ZIP, Duplicate, Terminal. SHA-256 is not a default slice (still addable through ring customization). Text: Trim, Format JSON, Count, UPPERCASE, lowercase, Sort. Window: Center, Pin on top (reads "Unpin" when the window is already on top, checked live on every open), Fit, Details. Anything else follows in catalog order. When more than six qualify, five show and the sixth wedge is **More…**, which opens the dashboard list.
- Keyboard: Escape closes and returns focus to the window the ring was opened over (if it still exists); arrows and Tab move the selection; Enter or Space runs it; 1 to 6 run a wedge directly. The first wedge is selected when the ring opens. Mouse hover selects and a click runs; the center click also returns focus.
- The ring opens at the cursor, clamped to the monitor work area, and closes when it loses focus.

## Invocation and context
- Ctrl+` (physical backtick key) opens the ring. Plain backtick is never intercepted. If the shortcut is taken, the dashboard says so and the tray icon still works.
- The foreground window is recorded before any PieAll window takes focus. The selection is captured on a worker thread from that recorded window handle (never from whatever is foreground later) while the ring shows "Reading selection…". Actions can't run until it resolves. A 1.5 s deadline ends the wait: the ring then falls back as if capture found nothing, and a late worker reply is discarded.
- A newer or cancelled invocation discards late capture results. Paths from an earlier invocation are never reused.
- If capture finds nothing or fails, the ring shows only window actions when a source window exists. Otherwise the dashboard opens with a message and the fallbacks: drop files, **Open files…**, **Use clipboard text**.
- Clipboard text is read only when the user clicks **Use clipboard text**.
- Tray icon: left click opens the dashboard; right click offers Open PieAll, Show action ring and Exit. Closing the dashboard hides it. A second launch asks the running instance to show its dashboard.

## Dashboard
- A search box, a standard list box of every action (with its group, and "(needs …)" when it doesn't fit the current target), and buttons **Open files…**, **Use clipboard text**, **Show ring** and **Run**. A status line sits at the bottom.
- Tab moves between controls. Enter runs the selected action. Double-click runs. Escape hides the window. Files can be dropped onto the window.

## Parameter editor and results
- Actions with fields open a small topmost editor near the cursor: a label and a text box per field, **Run** (Enter) and **Cancel** (Escape). Numeric fields (size, position, padding, corners, quality) only accept digits; the text boxes are the keyboard equivalent of any drag handle.
- Image actions also show a 150 DIP preview box above the fields, with the first selected image as a thumbnail (max 256 px, decoded on a helper thread with size and memory limits; files over 64 MB or 50 MP show "No preview"). It redraws on every edit. Crop fades the image and shows the crop rectangle at full strength with an orange outline, and reports an invalid rectangle. Frame shows the background color, padding and image corner radius to scale. Resize shows the resulting size; other image actions show the plain thumbnail. Previews are approximations of the backend result.
- Only one job runs at a time. Starting another while one is active shows a status message (or a notice in the editor title) and does not start a process.
- Every action is coordinated on a worker thread and executes in a disposable process. The tray tooltip and dashboard status show progress. When it finishes, a result window opens with the text in a read-only box, **Copy** to the clipboard and **Close**.

## Accessibility
- All controls are standard Win32 controls with Tab order and keyboard activation. The ring has full keyboard support. Every text box and the action list sits directly after a visible label in tab order, which is how Windows pairs a label with a control; this has not been checked with Narrator or another screen reader.
- The ring itself exposes no UI Automation element, so a screen reader can't read it. The dashboard list reaches the same actions and is the accessible route.
- Text scaling isn't honored beyond the system DPI.

## Limits of this build
- GDI only: opaque ring, no per-pixel alpha or glass, no antialiased wedge edges, no animation, no Direct2D/DirectWrite or DirectComposition.
- No action icons; wedges show text labels only. The preview is a static bitmap with no accessible content beyond its caption.
- No UI Automation provider for the ring.
- No modifier-plus-drag invocation and no second ring level for format choices. Dropping files onto the ring or dashboard is the drag path.
- The shortcut is fixed at Ctrl+`; there is no settings UI.
- Windows size from the system DPI at startup (system-DPI aware, not per-monitor).
- Ring actions are ranked by a fixed context table, not by recent use.
- Starting PieAll with file arguments while it is already running doesn't forward them to the running instance.
- Dropping files from an elevated Explorer onto a normal PieAll is blocked by Windows.
- Not built or run by this task. Formatting, all-target type checks, strict Clippy and three UI geometry/ranking tests passed before concurrent updater integration. Runtime appearance and accessibility remain unverified.



## Startup splash — 21 October 2026

Every launch shows a 2.5 s splash in its own small transparent window, centered on screen above everything, with no background, whether or not the dashboard is open. It is a minimal mini ring in the logo colors (flat graphite `#1c1c1e` slices with dimmed off-white `#f4f4f6` icons, no tray, strokes or shadows): a copper `#ff8a3d` selection swipes quickly through six feature slices (Images, Video, Audio, PDF, Downloads, Shortcuts), each popping out slightly while the graphite center names it, then the center lands on "PieAll" and the ring fades and the window closes. The window ignores the mouse, never takes focus, skips the taskbar, and is `aria-hidden`; under reduced motion it shows the ring with "PieAll" and no swipe. The dashboard, floating ring and toast have no splash of their own.


## Neutral dashboard — 21 October 2026

The dashboard now matches the neutral ring and icon: page `#F5F5F6`, controls `#FFFFFF`, ink and primary buttons graphite `#1C1C1E`, muted `#6C6C70`, lines `#E1E1E4`. Orange `#FF8A3D` (`--select`) is reserved for the selected slice in the ring preview and customizer, echoing the icon. The empty state is three compact icon + short-label steps (Select, Hold, Release on a slice) with Choose files and Customize ring beneath; the duplicate caption is hidden until something is selected, and the selection bar reads "Nothing selected" instead of "Choose your files".

## Dark mode

The dashboard follows the Windows app theme (`prefers-color-scheme`). All dashboard colors come from tokens in `web/tokens.css`; the dark set overrides them there. The floating ring and splash keep their own always-dark styling. Runtime appearance is unverified.

## Download completion action - 21 October 2026

A successful download toast adds an Open location button below its result text and a Close notification button. It keeps the existing neutral toast styling and keyboard focus treatment, remains nonactivating, and stays eight seconds instead of the usual 1.4 seconds. It also appears when the dashboard is active. Selecting Open location reveals the saved file in Explorer; failures appear inline in the toast. Native keyboard, focus and Explorer behavior still require manual acceptance.
