# Stickies roadmap

[current-milestone.md](current-milestone.md) defines the work allowed now. This file records the agreed product scope and remaining phases. Do not implement later phases until they become the active milestone.

## Product scope

Stickies is a small, native Linux scratchpad. It uses Rust, GTK4/libadwaita, Wayland Layer Shell, and SQLite. Local editing works offline without an account. There is no telemetry or application-owned cloud backend.

- Keep plain-text notes, clickable checklists, and the existing simple list controls.
- Copy text is sufficient. Dedicated Markdown editing, Markdown rendering, and file export are not required.
- Multi-monitor behavior follows the cursor: the monitor containing the cursor has the edge strip. Do not add a monitor picker or independently maintained decks per monitor.
- Google Drive sync is required, with a simple setup and few controls. Sync is optional to enable; local notes must keep working when disconnected. The exact integration is not selected yet.
- Application-level note encryption is not required. Authentication credentials for Drive still need appropriate storage.
- Retain broader compositor validation and packaging/release work.
- Virgil is the agreed handwritten font, Inter the clean font, and JetBrains Mono the monospace font. Reference images do not override these choices.

## Work order

Documentation cleanup, the audit, and confirmed unused-code cleanup are complete. The UI checkpoint is committed as `fe97c94`. Remaining audit findings are listed in [code-audit.md](code-audit.md).

1. Implement the approved UI revision in [ui-implementation-plan.md](ui-implementation-plan.md), starting with an authorized compact visual preview. The saved references and later user clarifications define its target. Both docked and separate notes are to be resizable. Settings, checklist behavior, and final text sizes remain pending.
2. Resolve the remaining approved fixes and platform checks before closing phases 5A and 5B.
3. Continue with cursor-following multi-monitor behavior, Google Drive sync, compositor validation, and packaging/release.

Commit and push require explicit authorization.

## Current foundation

Phases 0 through 4 are implemented: native edge interaction, note state, SQLite persistence, and All Notes management. Phase 5A implementation exists for CI, single-instance handling, configuration, desktop metadata, and autostart entry management, but final validation remains open. Parts of phase 5B are also implemented. Neither phase should be declared complete before the final UI and outstanding desktop checks pass.

## UI and phase 5B: desktop behavior

Finish the compact All Notes and individual-note revision, the four-note vertical edge strip with integrated search, pointer targeting, reduced motion, and final user review. Verify all four edges, marker movement, approved resizing behavior, separate-window movement, fullscreen/DND behavior, and independent inactivity timers. Finish the application icon and user-facing startup controls as needed. Verify an installed launch after login.

Use native keyboard navigation and readable focus states. Add application shortcuts only after the core UI is stable. Keep pinned-first ordering, with the edge following All Notes' ordering choice. The individual pin icon controls Always open rather than ordering. Manual drag reordering is not part of the active milestone.

Complexity: medium. Desktop interaction and final visual approval are the main uncertainties; most note behavior already exists.

## Phase 6: cursor-following monitors

Maintain one edge strip on the monitor containing the cursor. Preserve the configured edge and notes when the cursor crosses monitors. Verify laptop-only, external-only, dual-display, hotplug, and mixed-scale setups.

Before implementation, settle what happens to an already-open docked note during a monitor change. Preserve edits and avoid duplicating the strip. Determine how each supported compositor exposes the required cursor/monitor information; do not assume the existing first-monitor geometry code provides this behavior.

Complexity: medium to high. The product rule is simple; compositor access and display changes need testing.

## Phase 8: Google Drive sync

Build the smallest Google Drive integration that meets the user's needs. Choose direct integration or an existing local Drive integration only after checking feasibility during this phase. No generic multi-provider sync framework, collaboration, or dedicated Stickies server.

Define the minimum behavior before coding: connect/disconnect, initial upload/download, offline edits, updates, deletions, and simultaneous edits on two devices. Handle conflicts without silently discarding a note; a simple retained conflict copy may be enough. Do not treat copying a live SQLite database between devices as a complete sync design.

Drive sync is a required remaining feature. Optional enablement does not mean optional implementation. Keep authentication and sync transport separate from ordinary offline editing. No note-encryption feature is planned.

Complexity: medium to high until the integration is chosen. Sign-in, offline recovery, and preserving conflicting edits are the essential work, even with a small UI.

## Phase 10: compositor validation

Keep Hyprland as the reference compositor. Validate Hyprland on other distributions, KDE Plasma Wayland and Sway, then other Layer Shell compositors such as COSMIC, Niri, River, and Wayfire.

Test edge placement, focus, floating windows, fullscreen behavior, scaling, cursor-following monitors, and launch at login. Claim support only after testing. DND currently integrates with Omarchy shell only.

GNOME and X11 are outside this Linux scope. Windows and macOS would be separate future projects, not reasons to add portability abstractions now.

Complexity: high testing effort across environments.

## Phase 11: packaging and release

Start with Arch/AUR and GitHub release artifacts. Retain AppImage, Flatpak, Debian and RPM packaging as distribution-expansion work; stage those formats instead of requiring every format for the first package.

Provide the binary, desktop entry, application icon, needed metadata and bundled-font notices. Verify installation, startup, upgrade, and removal without deleting user notes. Automate checks, builds, checksums, and release artifacts. Review the project name before public release.

Complexity: medium for the first package, higher across all formats.

## Implementation constraints

Keep the existing stack and module responsibilities. Prefer concrete fixes over architectural rewrites. Do not introduce a generic event bus, unused storage backends, or future configuration controls.

Retain database migrations and compatibility parsing needed to load existing data. Remove obsolete runtime branches only after confirming their callers. Never delete user notes as cleanup.

The source license is MPL-2.0. Keep bundled asset notices. Basic accessibility, readable contrast, keyboard focus, and reduced motion remain requirements.

A local scripting interface or external MCP adapter is outside the release work above and requires its own approved milestone. No AI assistant, rich document editor, team workspaces, or attachments are planned.
