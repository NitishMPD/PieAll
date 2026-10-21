# Research: everyday app defaults in PieAll
Author: Nitish

Research date: 21 October 2026. Proposal only; no defaults or runtime code changed.

## Finding and evidence limits

The product goal is to shorten repetitive workflows: turn several clicks or key chords into one invocation, expose buried operations, and keep actions tied to the app where the work happens. Daily frequency helps choose candidates, but the main criterion is work saved. User feedback explicitly prioritizes Figma property transfer, flips, alignment, components and auto layout; browser Copy URL while retaining Incognito; and preserving code-editor context when code is selected. The original frequency-led ranking below has been revised for those requirements.

This review compares all 57 bundled app profiles, grouped below where they share a recommendation. ?Before? is the current working-tree catalog, not a claim about an earlier release. Most initial recommendations use six direct slots, with Search actions and Window in the remaining slots. The revised Figma proposal uses six direct commands, one Align group and one More group; Search actions, Window and secondary commands move into More. This is a proposed interaction change, not a JSON-only reorder. Other initial rankings are provisional until assessed against the workflow-saving criterion.

Official vendor documentation establishes command meanings, default keymaps and intended workflows. Some vendors label lists ?frequently used,? ?most useful,? or ?most popular?; these are vendor recommendations, not representative independent frequency studies. The proposed rankings are my product judgment from those sources. PieAll has no usage data in this review, and no users or target apps were observed. Confidence is higher for documented semantic mismatches than for choosing one useful command over another.

[NN/g recommends observing common tasks before prioritizing shortcuts](https://www.nngroup.com/articles/ui-copy/). Accordingly, these are candidates for validation, not claims of measured daily popularity. A useful ring command combines recurring task value, enough benefit over the native shortcut, reliable context, and clear consequences. Frequency alone does not justify filling every ring with Copy/Paste/Save.

## Revised selection rule: save repeated work

Prioritize commands that remove repeated pointer travel, avoid reopening several menus, apply one operation to several selected items, or perform a short deterministic sequence. Treat documentation as evidence that the operation exists; the user's workflow supplies the reason to promote it. Do not remove Incognito or another requested action because a general popularity assumption ranks it lower.

Examples of the intended improvement:

- Figma: copy properties from a source, select targets, paste properties once. The source and target selections remain separate user decisions; a blind copy-and-paste macro would not choose targets correctly.
- Figma: align or distribute selected layers without repeated trips to the right sidebar. Include Left, Horizontal center, Right, Top, Vertical center and Bottom; horizontal/vertical distribution are useful additional choices.
- Browser: Copy URL performs address-bar focus and copy in one invocation. Address bar alone leaves the main job unfinished. Keep the exact URL; tracking-parameter cleanup is a different operation.
- VS Code: rename a symbol, edit all matching occurrences, comment a selection, format a selection, or duplicate/move lines without falling into a generic text-conversion ring.

Native single-key commands can still be useful for pointer-led users. The rule is not to exclude them categorically, but to give stronger workflow savings priority when slots compete.

## What the implementation actually does

- `src/shortcuts.json` owns each profile's ordered `quick` IDs. `src/shortcuts.rs` converts that order to priorities; `web/menus.ts` takes the first six app commands for the default ring.
- Saved layouts in `web/prefs.ts` override built-ins per app/context. Any eventual default update should preserve them and keep existing action IDs stable.
- App commands appear only for a recognized app in window context. Selected text/files/images take precedence. Replacing Word's six defaults will not make them appear over selected text; that is a separate interaction issue. Explorer's app shortcuts are similarly displaced when a folder/file selection is captured.
- Detection is by executable, not document type, website, app mode, active panel, meeting state or database state. Figma in a browser receives browser actions. A meeting-only Teams ring will be less useful outside calls, but state-specific rings require capability the current implementation does not have.
- Dispatch sends keys to the captured application; it cannot prove that the intended command ran. Mode, pointer location, focus, keyboard layout, custom mappings and app version matter. Single-letter tools may type text when an edit field is focused.

## Before and proposed defaults

`[new]` means a catalog addition or a new verified binding is required. Other proposals reuse existing IDs; that does not mean their bindings have been runtime-tested.

### File Explorer

- **Before:** New folder; Rename; Properties; Address bar; Search folder; Details view.
- **Proposed:** New folder; Rename; Search folder; Address bar; Preview pane; Parent folder.
- **Reason:** Replace Properties and Details view with previewing and folder navigation. These shortcuts appear only in window context; selected files currently open the file-tool ring. [Vendor reference](https://support.microsoft.com/en-us/accessibility/windows/keyboard-shortcuts-in-windows).

### Google Chrome, Microsoft Edge, Brave, Vivaldi, Opera, Chromium, Arc, Firefox, Waterfox, LibreWolf, Floorp, Zen Browser

- **Before:** New tab; Reopen closed tab; Downloads; Find on page; Bookmark page; New private window.
- **Revised proposed six:** Copy URL [new]; Reopen closed tab; New private window; Paste plain text; Find on page; Bookmark page.
- **Reason:** Copy URL collapses address-bar focus plus copying into one action. Paste plain text removes repeated formatting cleanup when transferring content. Incognito stays, as explicitly requested. New tab and Downloads remain accessible through search/customization. This six-slot ranking is a proposal; Copy URL and keeping Incognito are the pinned requirements.
- **Execution:** A candidate sequence is Ctrl+L, Ctrl+C, Escape. This is a derived workflow using documented browser primitives, not a vendor-provided single shortcut. Test that the full URL is copied and page focus is restored in every supported browser. Leave the copied URL on the clipboard intentionally. Do not treat escaped address-bar editing as proof of exact caret/selection restoration.
- **Source:** [Chrome keyboard shortcuts](https://support.google.com/chrome/answer/157179?hl=en). Firefox and browser forks require their own compatibility checks.

### Microsoft Word

- **Before:** Bold; Find; Replace; Change case; Insert comment; Word count.
- **Proposed:** Find; Replace; Clear character formatting; Insert comment; Track changes; Word count.
- **Reason:** Give document editing and review more space: remove Bold and Change case, add Clear formatting and Track changes. Selection capture currently hides these app shortcuts when text is selected, which limits the value of selection-based commands. [Vendor reference](https://support.microsoft.com/en-us/accessibility/word/keyboard-shortcuts-in-word).

### Microsoft Excel

- **Before:** Edit cell; Format cells; Create table; Toggle filters; AutoSum; Fill down.
- **Proposed:** Edit cell; Format cells; Create table; Toggle filters; AutoSum; Fill down.
- **Reason:** Keep. The current mix covers editing, formatting, structured data and calculations; there is no convincing evidence here for replacing it wholesale. [Vendor reference](https://support.microsoft.com/en-us/accessibility/excel/keyboard-shortcuts-in-excel).

### Microsoft PowerPoint

- **Before:** New slide; Duplicate selection; Start slideshow; Present from current slide; Group objects; Selection pane.
- **Proposed:** New slide; Duplicate slide; Format object; Group objects; Selection pane; Present from current slide.
- **Reason:** Use an explicit Duplicate slide action rather than a focus-dependent Duplicate selection action. Replace Start slideshow from beginning with Format object; retain presenting from the current slide. Slide and object commands still depend on focus. [Vendor reference](https://support.microsoft.com/en-us/accessibility/powerpoint/use-keyboard-shortcuts-to-create-powerpoint-presentations).

### Notepad

- **Before:** Find; Replace; Go to line; Insert date and time; New; Save.
- **Proposed:** Find; Replace; New; Save; Save as; Go to line.
- **Reason:** Replace Insert date/time with Save as. Keep Go to line for log/config editing. This is a general-purpose baseline, not a claim about usage percentages. [Vendor reference](https://support.microsoft.com/en-us/windows/keyboard-shortcuts-in-apps-1399b872-8e99-3219-bb7f-1a43e9a45b7a).

### Paint

- **Before:** Resize and skew; Image properties; Undo; Save as; Copy; Paste.
- **Proposed:** Resize and skew; Crop [new]; Save as; Undo; Redo; Paste.
- **Reason:** Promote Crop over Image properties and Redo over Copy. Crop needs a selection and a verified Windows Paint binding; its presence here is a proposed catalog addition, not an implemented command. [Vendor reference](https://support.microsoft.com/en-us/windows/keyboard-shortcuts-in-apps-1399b872-8e99-3219-bb7f-1a43e9a45b7a).

### Windows Terminal

- **Before:** New tab; Split pane; Command palette; Find; Copy; Paste.
- **Proposed:** New tab; Split pane; Command palette; Find; Copy; Paste.
- **Reason:** Keep. Copy/Paste merit slots here because terminal conventions differ from ordinary editors. Never substitute shell-specific commands such as clear, interrupt or rerun for terminal-level shortcuts. [Vendor reference](https://learn.microsoft.com/en-us/windows/terminal/tips-and-tricks).

### VLC media player

- **Before:** Play or pause; Mute; Subtitle track; Full screen; Snapshot; Open file.
- **Proposed:** Play or pause; Short jump back; Short jump forward; Subtitle track; Full screen; Mute.
- **Reason:** Replace Snapshot and Open file with a paired seek-back/seek-forward workflow. This targets watching media; playlist-heavy users may prefer Next item. [Vendor reference](https://docs.videolan.me/vlc-user/desktop/3.0/en/basic/hotkeys.html).

### Figma

- **Before:** Frame tool; Rectangle tool; Add auto layout; Quick actions; Group selection; Export.
- **Revised primary commands:** Copy properties; Paste properties; Flip horizontal; Flip vertical; Create component; Add auto layout; Align group.
- **Align group:** Left; Horizontal center; Right; Top; Vertical center; Bottom. Add Distribute horizontal spacing and Distribute vertical spacing as two related choices.
- **More candidates:** Copy/paste individual fill/stroke/effect properties, Frame selection, Group/Ungroup, Paste over selection, Duplicate, layer ordering and Export. These should not displace the primary operations merely because their bindings already exist.
- **Root proposal:** Copy properties, Paste properties, Flip horizontal, Flip vertical, Create component and Auto layout are six direct slices. Align and More complete the eight-slice ring. More contains Search actions, Window and secondary commands, with the same keyboard access as other groups. The six-command cap and prohibition on app groups are existing implementation constraints, not reasons to drop the requested workflows.
- **Semantics:** Figma calls the bundled commands Copy properties and Paste properties. They are broader than transferring a single named style; use the precise labels and do not promise style/variable-link preservation beyond the target app's behavior. Copy and Paste remain separate because users must choose the destination objects between them.
- **Catalog gaps:** Property transfer, flips, component creation and auto layout already have bindings. Alignment/distribution entries need to be added and checked against Windows defaults.
- **Sources:** [Copy/paste properties](https://help.figma.com/hc/en-us/articles/4412765442967-Copy-and-paste-properties-between-layers), [Alignment and distribution](https://help.figma.com/hc/en-us/articles/360039956914-Adjust-alignment-rotation-position-and-dimensions), [Auto layout](https://help.figma.com/hc/en-us/articles/360040451373-Guide-to-auto-layout-in-Figma).

### Adobe Photoshop

- **Before:** Brush; Free transform; Layer via copy; Image size; Select and mask; Save a copy.
- **Proposed:** Brush; Free transform; Layer via copy; Crop tool; Deselect; Save a copy.
- **Reason:** For a general image-editing baseline, promote Crop and Deselect over Image size and Select and mask. This is a workflow judgment; retouchers may reasonably retain Select and mask. Layer via copy is not a universal duplicate-layer command when pixels are selected. [Vendor reference](https://helpx.adobe.com/photoshop/using/default-keyboard-shortcuts.html).

### Adobe Illustrator

- **Before:** Selection tool; Pen; Shape builder; Group; Make clipping mask; Fit artboard.
- **Proposed:** Direct selection; Pen; Shape builder; Group; Make clipping mask; Fit artboard.
- **Reason:** Keep most choices. Replace Selection tool with Direct selection to expose anchor/path editing alongside object grouping and clipping. This is a modest hypothesis, with lower confidence than the DataGrip correction. [Vendor reference](https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html).

### Adobe Premiere Pro

- **Before:** Razor; Add edit; Mark in; Mark out; Speed and duration; Export media.
- **Proposed:** Selection tool; Add edit; Ripple edit; Mark in; Mark out; Export media.
- **Reason:** Replace Razor and Speed/duration with Selection tool and Ripple edit. Add edit already handles a cut at the playhead, so the six slots need not include both cutting commands. Timeline/source-monitor focus remains essential. [Vendor reference](https://helpx.adobe.com/premiere/desktop/get-started/keyboard-shortcuts/default-keyboard-shortcuts.html).

### Adobe After Effects

- **Before:** New composition; Precompose; Show position; Show animated properties; Easy ease; Add to render queue.
- **Proposed:** Precompose; Duplicate; Split layer; Show animated properties; Easy ease; Graph editor.
- **Reason:** Promote repeated timeline work over New composition, Show position and Add to render queue. Preview is a reasonable custom alternative, but a one-key preview command is already easy to reach. [Vendor reference](https://helpx.adobe.com/after-effects/desktop/get-started/keyboard-shortcuts/keyboard-shortcuts-reference.html).

### Blender

- **Before:** Move; Rotate; Scale; Toggle edit mode; Add object or mesh; Search.
- **Proposed:** Move; Rotate; Scale; Toggle edit mode; Add object or mesh; Search.
- **Reason:** Keep for a modeling baseline. Blender routes many keys through the editor under the pointer; these cannot be advertised as universal app-wide actions. Sculpting and animation need different choices, not an unsupported global replacement. [Vendor reference](https://docs.blender.org/manual/en/latest/interface/keymap/blender_default.html).

### Visual Studio Code, Visual Studio Code Insiders, VSCodium, Cursor, Windsurf

- **Before:** Command palette; Quick open; Format document; Toggle line comment; Search files; Focus terminal. Selecting code can replace this with generic text actions.
- **Revised editing candidates:** Format selection; Toggle line comment; Rename symbol; Select all occurrences; Quick fix/refactor; Duplicate or move lines. Format document, file navigation, terminal and command discovery remain useful secondary or no-selection actions.
- **Required behavior:** Keep the captured editor identity when code is selected. A selection supplies the target for editor operations; it must not by itself demote the context to generic text. Provide generic text transformations through an explicit secondary route. Apply the same ownership principle to other recognized editors, with per-profile rules rather than guessing code from its characters.
- **Root cause:** Native Session::view classifies any nonempty captured text as text context. web/menus.ts returns the generic text menu immediately for non-window contexts. The native action inventory also needs to retain the supported editor actions; a frontend-only menu change would leave them unavailable. Preserve captured-window identity, eligibility checks and selection safety throughout.
- **Boundary:** Executable identity establishes the editor application, not that its code pane is focused. Rename/refactor commands can behave differently in a terminal, search box or chat input; current support must describe that limitation rather than claim pane detection exists.
- **Source:** [VS Code basic editing](https://code.visualstudio.com/docs/editing/codebasics), [Windows shortcuts](https://code.visualstudio.com/shortcuts/keyboard-shortcuts-windows.pdf). Check derivative-specific keymaps before sharing bindings.

### IntelliJ IDEA, PyCharm, WebStorm, PhpStorm, Rider, CLion, GoLand, RubyMine, Android Studio, RustRover

- **Before:** Search everywhere; Find action; Rename; Reformat code; Run; Terminal.
- **Proposed:** Search everywhere; Recent files; Show intention actions; Rename; Reformat code; Run.
- **Reason:** Replace Find action and Terminal with Recent files and Context actions. JetBrains explicitly recommends Recent files and Context actions; Search everywhere already covers actions as well as files. Validate IDE-specific default keymaps rather than assuming every derivative is identical. [Vendor reference](https://www.jetbrains.com/help/idea/mastering-keyboard-shortcuts.html).

### DataGrip

- **Before:** Search everywhere; Find action; Rename; Reformat code; Run; Terminal.
- **Proposed:** Search everywhere; Execute statement [new]; Open query console [new]; Reformat code; Database Explorer [new]; Find action.
- **Reason:** Separate DataGrip from the generic IDE quick list. Query execution and opening a query console belong here; generic Run and Terminal are a poor fit. Execute statement can modify data, so its label must be explicit and PieAll must leave database permissions, transactions and execution prompts to DataGrip. [Vendor reference](https://www.jetbrains.com/help/datagrip/run-a-query.html).

### Microsoft Outlook (classic)

- **Before:** New email; Reply; Reply all; Forward; Search mailbox; Calendar view.
- **Proposed:** New email; Reply; Forward; Search mailbox; Mark unread; Calendar view.
- **Reason:** Replace Reply all with Mark unread for an inbox-triage baseline. Reply all stays searchable. This proposal concerns classic Outlook only; the profile does not establish support for new Outlook. [Vendor reference](https://support.microsoft.com/en-us/accessibility/outlook/keyboard-shortcuts-for-outlook).

### Microsoft OneNote

- **Before:** New page; Search all notebooks; To do tag; Bold; Insert link; Full page view.
- **Proposed:** New page; Search all notebooks; To do tag; Bold; Insert link; Full page view.
- **Reason:** Keep. The current choices already cover capture, retrieval, checklist tagging, formatting and focused reading. Quick Note and Paste text only are useful candidates for later validation, not automatic replacements. [Vendor reference](https://support.microsoft.com/en-us/accessibility/onenote/keyboard-shortcuts-in-onenote).

### Slack

- **Before:** Jump to conversation; All unread; Threads; Search conversation; Set status; Upload file.
- **Proposed:** Jump to conversation; All unread; Threads; Search workspace; Mentions and reactions; Upload file.
- **Reason:** Replace Set status with Mentions/activity and broaden Search conversation to Search workspace. Slack navigation evolves; verify the actual destination and rename the label to match the supported client before shipping. [Vendor reference](https://slack.com/help/articles/201374536-Slack-keyboard-shortcuts).

### Discord

- **Before:** Quick switcher; Search channel; Toggle mute; Toggle deafen; Toggle inbox; Upload file.
- **Proposed:** Quick switcher; Search channel; Toggle mute; Toggle deafen; Toggle inbox; Upload file.
- **Reason:** Keep for a combined chat/voice baseline. A voice-only audience may prefer Answer incoming call; the evidence does not establish that audience for PieAll. [Vendor reference](https://support.discord.com/hc/en-us/articles/31232432266647-Discord-Commands-Shortcuts-and-Navigation-Guide).

### Spotify

- **Before:** Play or pause; Next track; Previous track; Shuffle; Repeat; Search.
- **Proposed:** Play or pause; Next track; Previous track; Search; Like/dislike song [new]; Open queue [new].
- **Reason:** Keep transport and Search; replace Shuffle and Repeat with Like/dislike song and Open queue for a discovery/listening baseline. Both proposed actions need catalog entries. Verify all bindings against the supported desktop version first: the existing mute/preferences bindings differ from the current support page. [Vendor reference](https://support.spotify.com/article/keyboard-shortcuts/).

### Zoom

- **Before:** Mute or unmute audio; Start or stop video; Open chat; Raise or lower hand; Show participants; Full screen.
- **Proposed:** Mute or unmute audio; Start or stop video; Open chat; Start or stop screen sharing; Raise or lower hand; Show participants.
- **Reason:** Replace Full screen with Share screen/Stop sharing. The documented shortcut opens the sharing chooser when idle and stops an active share; the label should make that dual behavior clear. [Vendor reference](https://support.zoom.com/hc/en/article?id=zm_kb&sysparm_article=KB0067050).

### Microsoft Teams

- **Before:** Search; New chat; Mute or unmute microphone; Camera on or off; Raise or lower hand; Share screen.
- **Proposed:** Search; New chat; Find in current chat [new]; Unread chats [new]; Mute or unmute microphone; Share screen.
- **Reason:** For a mixed workday, replace Camera and Raise hand with Find in current chat and Unread chats. These two commands need new entries. For meeting-heavy users, the original camera/hand choices remain defensible. Do not promote the existing Ctrl+2/Ctrl+4 labels as fixed Chat/Calendar destinations. [Vendor reference](https://support.microsoft.com/en-us/accessibility/teams/keyboard-shortcuts-for-microsoft-teams).

### Obsidian

- **Before:** Quick switcher; Command palette; New note; Toggle reading view; Search all files; Toggle checkbox.
- **Proposed:** Quick switcher; Command palette; New note; Toggle reading view; Search all files; Toggle checkbox.
- **Reason:** Keep. This already covers finding, capturing, reading and tasks. Quick switcher and Command palette are different tools; retaining both is reasonable for Obsidian. [Vendor reference](https://obsidian.md/help/plugins/quick-switcher).

### Adobe Acrobat

- **Before:** Find; Go to page; Fit page; Fit width; Print; Advanced search.
- **Proposed:** Find; Go to page; Fit width; Previous view; Rotate clockwise; Print.
- **Reason:** Replace Fit page and Advanced search with Previous view and Rotate view for ordinary PDF reading. Rotate here is a viewing action, not a promise to rewrite PDF pages. Researchers searching many PDFs may prefer Advanced search. [Vendor reference](https://helpx.adobe.com/acrobat/desktop/get-started/preferences-and-settings/keyboard-shortcuts.html).

### Audacity

- **Before:** Play or stop; Split; Trim audio; Silence audio; Fit selection; Import audio.
- **Proposed:** Play or stop; Split; Trim audio; Undo; Fit selection; Import audio.
- **Reason:** Replace Silence audio with Undo in a basic editing workflow. Keep silence available through search. Check against the supported Audacity version; newer manuals distinguish clip trimming and other edit behavior. [Vendor reference](https://manual.audacityteam.org/man/keyboard_shortcut_reference.html).

### Godot

- **Before:** Play project; Play current scene; Stop project; Quick open scene; Quick open script; Command palette.
- **Proposed:** Play project; Play current scene; Stop project; Quick open scene; Quick open script; Command palette.
- **Reason:** Keep. Project/scene test execution and resource navigation fit the editor loop. Pause or Duplicate node are reasonable personal substitutions, not clearly better defaults for everyone. [Vendor reference](https://docs.godotengine.org/en/4.5/tutorials/editor/default_key_mapping.html).

### Notion

- **Before:** Quick find; New page; Toggle sidebar; To do list; Duplicate block; Comment.
- **Proposed:** Quick find; New page; Duplicate block; Comment; Copy page URL [new]; To do list.
- **Reason:** Replace Toggle sidebar with Copy page URL, which Notion includes in its Most popular shortcut section. Copy page URL needs a new catalog entry. Retain comment for collaborative use and to-do for task capture. [Vendor reference](https://www.notion.com/help/keyboard-shortcuts).

### GIMP

- **Before:** Paintbrush; Crop; New layer; Export as; Scale; Open as layers.
- **Proposed:** Crop; Duplicate layer; New layer; Export as; Scale; Open as layers.
- **Reason:** Replace Paintbrush with Duplicate layer for a general image-editing baseline. Here Scale is the existing scale tool, not an image-resize dialog. Painting-first users should retain the brush. [Vendor reference](https://docs.gimp.org/3.0/en/key-reference-layer.html).

### Krita

- **Before:** Freehand brush; Toggle eraser mode; Brush smaller; Brush larger; Canvas only mode; Mirror canvas.
- **Proposed:** Freehand brush; Toggle eraser mode; Brush smaller; Brush larger; Canvas only mode; Mirror canvas.
- **Reason:** Keep for painting. Brush size, erasing and canvas access are coherent together; no sourced reason emerged to replace them with file or layer management. Validate the exact mirror command/keymap on the supported version. [Vendor reference](https://docs.krita.org/en/user_manual/getting_started/navigation.html).

### Inkscape

- **Before:** Selector; Node tool; Bezier; Object to path; Fill and stroke; Export.
- **Proposed:** Selector; Node tool; Fill and stroke; Align and distribute; Duplicate; Export.
- **Reason:** Replace Bezier and Object to path with Align/distribute and Duplicate for a general layout/vector-editing baseline. Path illustration specialists can retain Bezier; Object to path should remain searchable. [Vendor reference](https://inkscape.org/de/en/doc/tutorials/basic/tutorial-basic.html/).

## Corrections to make before changing rankings

1. **DataGrip ownership:** Give it a small app-specific command set. JetBrains documents query execution with Ctrl+Enter and opening a query console with F4 in Database Explorer. These are context-dependent and must not be confused with the generic IDE Run binding. [DataGrip query workflow](https://www.jetbrains.com/help/datagrip/run-a-query.html).
2. **Teams navigation labels:** The catalog calls Ctrl+2 ?Go to chat? and Ctrl+4 ?Go to calendar.? Microsoft documents these as the second and fourth apps in the app bar. Their destinations depend on the user's ordering. Prefer directly documented chat actions over fixed-position assumptions. Microsoft documents Ctrl+F for finding in the current chat/channel and Ctrl+Alt+U for unread chats. [Teams shortcuts](https://support.microsoft.com/en-us/accessibility/teams/keyboard-shortcuts-for-microsoft-teams).
3. **Spotify binding drift:** The catalog sends Ctrl+Shift+Down for Mute and Ctrl+P for Preferences. The current support page lists M for mute and Ctrl+Comma for preferences. It also lists Alt+Shift+B for Like/dislike song and Alt+Shift+Q for Queue. Verify against the supported Windows desktop version before editing; this mismatch is evidence of catalog drift, not proof of runtime failure in every version. [Spotify shortcuts](https://support.spotify.com/article/keyboard-shortcuts/).
4. **Browser families:** Google documentation is not proof that every Chrome-derived app shares every binding. The same qualification applies to Firefox forks. Audit the six proposed commands per supported client before shipping; the twelve-profile grouping above is a proposed baseline. [Firefox shortcuts](https://support.mozilla.org/en-US/kb/keyboard-shortcuts-perform-firefox-tasks-quickly).
5. **Editing context:** Blender, Premiere, After Effects, Office and design tools route commands by mode, panel, pointer or selection. Do not label them as globally reliable app actions. [Blender default keymap](https://docs.blender.org/manual/en/latest/interface/keymap/blender_default.html), [Figma keyboard access](https://help.figma.com/hc/en-us/articles/360040328653-Use-Figma-products-with-a-keyboard).

## Recommended order of work

First preserve code-editor context when code is selected, then implement and validate browser Copy URL while retaining Incognito. Expand the Figma catalog around property transfer, flips, alignment/distribution, components and auto layout. Correct misleading bindings/labels and separate DataGrip query actions from generic IDE actions.

Reassess the other applications with the same work-saving lens. Useful candidates include Excel Fill down/Format cells/filters; PowerPoint alignment/distribution/duplicate slide; Illustrator and Inkscape alignment, duplication and clipping/path operations; Photoshop property/layer operations and transforms; Premiere ripple edits and paste attributes; After Effects precompose, split, duplicate and easing. Existing catalog coverage and the vendor references above support investigating these workflows, but do not establish their final ranking. Merely replacing Silence with Undo or Selection tool with Direct selection does not prove a productivity improvement.

Reuse existing profile bindings and Customize where they fit. Add short deterministic workflows such as Copy URL at the native action owner, retaining focus checks and visible failure handling. App-preserving selection routing and Figma alignment groups require coherent native/frontend changes beyond quick-list reordering. Keep user layouts and learned positions stable; do not automatically rearrange slots by usage. No usage tracking or recommendation service is needed for these changes.

Before shipping, validate the new commands in their intended Windows app versions and contexts. For selection-based commands, include selected objects and focused text inputs; for browser forks, check each distinct client; for meeting actions, check both call and non-call states. User-run native verification is necessary because static catalog checks cannot establish target-app behavior. No builds, app launches, browser automation or runtime tests were run for this research.

## Research coverage and qualifications

The comparison covers every bundled profile. Sources were researched through official vendor help, manuals, reference sheets and NN/g's shortcut guidance. Browser-fork recommendations extrapolate from the Chrome/Firefox families; IDE recommendations extrapolate from JetBrains' shared guidance and require product-specific validation. Paint, Notepad and some specialist tool rankings have weaker evidence and should be treated as workflow hypotheses.

The direct Inkscape key-reference fetch was blocked; its official basic tutorial supplied the Align/distribute workflow instead. The old GIMP key-reference URL was unavailable; the versioned GIMP layer reference and official quick-reference sheet supplied alternatives. The unversioned Godot stable shortcut page returned 404; a versioned 4.5 reference was available. Premiere's full-page fetch timed out; its official indexed shortcut reference was available. These sources do not establish a fresh runtime audit of those applications.
