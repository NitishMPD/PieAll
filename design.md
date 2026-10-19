# PieAll design
Author: Nitish

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

