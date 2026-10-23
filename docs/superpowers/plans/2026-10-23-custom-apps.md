# Custom App Profiles Implementation Plan

**Goal:** Add and edit executable-matched app profiles and keyboard shortcuts from Customize ring.

**Architecture:** Keep bundled profiles immutable. Store custom profiles in the existing native SQLite settings database; resolve both through the same profile and shortcut APIs using owned snapshots. Native dispatch retains captured process, focus, input and revision checks.

**Constraints:** No builds, app launches, branches, commits, deployment or subagents. Preserve Dia and existing edits. Custom apps cannot replace bundled executable identities or invoke shell commands.

- [x] Add validated custom profile storage, schema migration and catalog lookup; test persistence, invalid keys, duplicate identities and edits.
- [x] Wire dashboard-only picker/save/remove commands and refresh captured profiles. Update callers for owned snapshots; run Rust checks.
- [x] Add accessible Add app and Edit app forms to Customize ring with stable shortcut identities; test form rendering and menu defaults.
- [x] Update project/design docs and daily task file after checks pass.

**Verification:** 24 targeted web tests and 21 targeted native tests passed, along with TypeScript, all-target Rust checks, strict workspace Clippy and formatting. One static visual check covered wide/narrow custom-app forms; no app launch, native picker or real shortcut dispatch was performed.
