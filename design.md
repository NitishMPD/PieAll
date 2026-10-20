# PieAll design
Author: Nitish

## Floating ring and toast — 20 October 2026

This supersedes the floating-ring parts of the section below; the dashboard look is unchanged.

- **Gesture.** Hold ` and the ring appears centered on the cursor, already showing what it found. Sliding toward a slice highlights it by angle, so a flick past the rim still counts. Releasing runs it. Releasing in the center (66-unit dead zone) leaves the ring open with "Click an action · Esc closes". While the key is held, the hub hint reads "Release on an action" in warm amber.
- **Look.** Smoked translucent glass: wedges `rgba(36,29,24,0.66)` with a faint cream edge and a soft drop shadow. The selected wedge is burnt orange `rgba(205,104,48,0.94)` with white text. Gaps and corners are fully transparent, so the desktop shows through. The hub is darker glass with the thumbnail or kind icon, the target name and the hint. No rim disc. It opens in 140 ms with a fade-scale, removed under reduced motion. Forced-colors styling is unchanged. There is still no native blur.
- **Slices.** Up to eight, sector zero at twelve o'clock, clockwise. Icon above a 12.5 px label, with the hint ("50% width & height", "White background", "Opens more") shown only on the selected slice. Submenus replace the ring in place; the hub becomes Back with the submenu's name.
- **Keyboard.** Arrows and Tab move a roving selection from nothing selected; Enter or Space runs it; 1–8 run directly; Backspace or Escape leaves a submenu; Escape closes and returns focus to the source window. Slices are `menuitem`s inside a `menu` named for the target, and submenus carry `aria-haspopup`.
- **Toast.** A 460×88 DIP pill at the bottom center of the cursor's work area, click-through and never focused. Dark glass with a 32 px status mark: green check (saved/copied), red alert (failed), neutral ring (info), spinner (working). A bold title such as "Saved photo-compressed.jpg" sits over a muted detail such as "2.4 MB → 820 KB, 66% smaller". It lingers 3 s, or 6 s for errors. Because the window never takes focus, screen readers may not announce it; the dashboard status line remains the accessible record.

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
- Ring choice is ranked by context, not catalog order. Images: Convert, Crop, Resize, Frame, Compress. Files: Copy paths, Reveal, Create ZIP, Extract ZIP, SHA-256, Duplicate, Terminal. Text: Trim, Format JSON, Count, UPPERCASE, lowercase, Sort. Window: Center, Always on top, Fit, Details. Anything else follows in catalog order. When more than six qualify, five show and the sixth wedge is **More…**, which opens the dashboard list.
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

