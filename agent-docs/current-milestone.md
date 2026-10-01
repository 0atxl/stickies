# Current milestone

## Status and next steps

The core note app, All Notes window, and Paper, Playful, Minimal Light/Dark appearances are implemented. The latest committed application-code checkpoint is `fe97c94`. The references and implementation plan are committed as `65e3758`. The authorized stage 1 visual preview is implemented and verified in the working tree; visual approval is pending. The user also authorized the individual panel's Pin, size, and weight controls, with font family limited to global Settings. The user has now authorized completion of the remaining note controls. All Notes More parity, the expanded shared palette and save feedback are implemented in the working tree. Edge-strip work remains unfinished.

Documentation cleanup, the code audit and the approved unused-code cleanup are complete. The UI checkpoint is committed locally. On September 30, 2026, the user supplied final visual references and clarified the next UI revision. The originals are saved in [design-references/2026-09-30](design-references/2026-09-30/README.md), and the work order is in [ui-implementation-plan.md](ui-implementation-plan.md). Storing the references and writing the plan did not change application code.

The updated spec and Settings image are saved in [design-references/2026-10-01](design-references/2026-10-01/README.md). Only the individual-tools clarifications and the fuller Settings draft changed from the original spec. The tools corrections are already implemented; Settings scope/behavior, checklist design and final text sizes remain pending. The reference update was documentation-only. The subsequent request, "complete the note controls", authorizes the remaining controls pass; Settings, edge and checklist implementation remain outside that scope. Separate audit findings remain open in code-audit.md. Complete the outstanding checks before declaring phases 5A and 5B finished. Do not start multi-monitor or sync implementation during this milestone.

The remaining product scope and work order are in [implementation.md](implementation.md).

## Agreed UI revision

Direct user decisions below take precedence over conflicting details in both saved UI/UX versions and earlier UI plans. Stage 1 implements appearance, layout, floating tools and resizing. Individual Pin, size and weight are also implemented. The remaining note controls are implemented. Edge strip, Settings and checklist work follow the implementation plan and need their own authorization.

- Four edge notes in the same ordering as All Notes. Replace the two existing title layouts with the reference's vertical tabs. Use resting dots, New note/Search/All Notes utility controls, integrated search, and one theme-following docked note with a decorative pointer. Custom ordering remains later work.
- Keep All Notes compact at a 720x500 default and resizable. Its footer stays Pin / Pop out / More. More contains the other note tools. Keep the existing 240px list and responsive drawer arrangement until the received Settings draft's host and width are settled.
- Make both docked and separate individual notes resizable, with matching minimum sizes and a subtle hover-enhanced corner grip. Preserve edge anchoring, wrapping, and vertical scrolling for long content.
- Keep the individual toolbar hidden by default. Its subtle opening control becomes clearer on hover and opens on click. The expanded toolbar shows compact size, weight, and color controls based on `tools.png`. Keep Close visible. Routine save status appears only in All Notes; errors use toasts in the affected view.
- All Notes Pin remains a persistent ordering preference. The individual pin icon instead controls that view's Always open behavior. Keep independent view timers; turning Always open off starts a fresh two-minute period. Close and Escape remain available, with existing failed-save retention preserved.
- Keep Virgil, Inter, and JetBrains Mono as global Settings-only note fonts, applied in both editors. Older per-note font overrides load but no longer override Settings. UI chrome uses the UI font. Individual tools show S/M/L and Regular/Bold; these persist per note and update both editors. Preview body sizes are 18/21/24px, with titles adjusted by the same 3px steps. Final sizes still await review.
- Use rounded corners throughout, including Minimal. Retain existing Paper tear and Playful wave drawing. Paper has a tag-colored side strip and Minimal a tag-colored top strip in both note views. Playful's note fill follows its tag color. Sidebar markers are dots in Playful and vertical bars in Paper/Minimal. Expand the shared color palette in a compact grid.
- Minimal Dark All Notes uses the light layout with dark colors. The title underline in the references is example content, not automatic decoration. Handwritten underlines and strike-through should appear scribbled when used; the action for general underline is not defined by that visual request.
- Archived notes retain their normal appearance and remain read-only until restored. Existing clickable checklists need visual/behavior redesign; wait for the user's item specification.
- Settings reference received: one scrollable surface with Appearance, Edge Strip, Behavior, Shortcuts and About sections. The spec explicitly keeps the section list draft/open. Use app UI typography, dividers, switches and outlined selected segments. Settle the host/width, preserve Minimal System/Light/Dark, clarify horizontal position presets and Reposition commit/cancel behavior. The strip-enable switch must not be confused with the existing hover-enable preference. Default new-note color should use the shared palette; four image samples do not limit it. Shortcut badges are examples, not currently working global bindings.

## Existing behavior to preserve

- Pinned active notes precede unpinned notes. Overflow stays available in All Notes. Opening a docked note hides the titles. Closing or Escape restores them. Hover does not replace an open note.
- Open away from edge creates or focuses a separate window without removing or reordering the note. Several different notes can remain open. No attachment/detachment animations.
- The current All Notes drawer is 260px and overlays below 980px. Individual notes default to 352x376 and resize to a minimum of 300x280. The tool panel floats beside the note without changing those dimensions.
- All Notes has a directly editable note, shared live title/body with individual windows, search/sort, and Pin / Pop out / More actions. Copy text copies the live body. Archive, restore, and deletion Undo are implemented.
- All / Active / Archived are visible filters. Archived notes remain read-only until restored. There is no pull-down archive gesture or collection menu.
- Docked and separate notes close after two minutes without input by default. Individual Pin means Always open and disables that view's timeout. The All Notes editor has no inactivity timeout. Ordering Pin is in All Notes; infinity and individual ordering Pin have been removed.
- Appearance changes apply immediately. Keep theme, global font, relevant paper patterns, reduced motion, and Minimal System/Light/Dark controls. Respect per-note pattern, shape, size, and weight choices.
- Virgil is the decided handwritten font. Inter and JetBrains Mono are the other choices. A reference image is a visual guide, not permission to replace agreed fonts.
- Paper uses soft ivory, a shallow torn bottom edge and soft shadow. Playful uses colored notes and dark outlines. Minimal uses opaque light/dark notes and a thin top accent. Stage 1 gives Minimal rounded corners and matching tag accents in both editing views.
- Medium body text is 21px. Medium Paper/Playful titles are 23px and Minimal titles 20px, with provisional Small/Large adjustments of minus/plus 3px. These are implementation values, not final approval of text sizes. Routine save status is now limited to All Notes. Each view shows deduplicated save-error toasts and dismisses its stale error toast after successful retry.
- Live checklists use clickable native checkboxes and persist as plain-text markers. Backspace at an item's start removes its checkbox. Archived previews currently show markers. Preserve this working foundation until the revised item behavior is approved. Existing bullets and numbered lists remain; broader Markdown and export are not planned.

## Code responsibilities

- `main.rs`: edge/window coordination and weak session lookup by note ID.
- `note_session.rs`: shared title/body buffers, note appearance and pin state, pending saves, and save errors. Failed body saves retain dirty text and block operations that would discard it.
- `editor.rs`: individual-note presentation, editing controls, and per-view inactivity timers.
- `manager.rs`: All Notes selection, filtering, actions, and embedded editor.
- `app.rs`: deck state and ordering.
- `note_controls.rs`: shared size/weight rows and compact 16-color grid, without its own session or persistence owner.
- `appearance.rs`, `assets/all-notes.css`, `settings.rs`, `ui_assets.rs`: drawing, styles, Appearance controls, bundled fonts and icons.
- `storage.rs`: SQL and migrations. Schema 3 expands the color constraint transactionally while preserving note IDs, metadata and the separate per-note appearance table.
- `platform.rs`: event-driven Omarchy DND state watching, with no desktop configuration changes.

## Remaining acceptance checks

Use [manual-testing.md](manual-testing.md) for the current checkpoint and [ui-implementation-plan.md](ui-implementation-plan.md) for the revision's acceptance checks. Update the manual checklist as the revision lands.

- Final user review of All Notes, individual notes, themes, toolbar controls and text sizes. Settle the received Settings draft and obtain the pending checklist item design before their implementation passes.
- Finish the single vertical edge-strip design and integrated search. Verify four-note ordering, pointer targeting, title readability, all four edges, keyboard focus, and reduced motion.
- Verify the two-minute timeout and Always open with the final UI.
- Verify fullscreen hides the strip while open notes stay visible. DND currently reads Omarchy shell state; other notification servers are not supported yet.
- Verify installed launch after login. A generated autostart entry alone does not prove this works.
- Recheck window movement/floating. An All Notes floating rule exists in the reference user's desktop configuration; portable installation must not assume every user has it.
- Investigate the graphics invalid-rectangle warning if it recurs. It was not reproducible in the later checks and is not declared fixed.
- Remeasure the final UI: at most 200 MiB PSS, ordinary interaction below 2% CPU, normal animation below 3%, exceptional one-second spikes at most 5%, idle returning to 0%. Use interval measurements with labeled activity. A prior comparison baseline was about 74 MiB PSS; it is not a final UI pass.

## Verification status

The current audit checks passed formatting, strict Clippy, 20 non-display tests, release build and diff checks. The existing GTK shared-editor regression passed in isolation; its previous fixed-deadline timing failure remains noted. Disposable checks confirmed menu retention, a missing edge refresh after blank-note creation, and settings controls remaining on rejected values. See [code-audit.md](code-audit.md) for findings and the proposed fixes. These checks do not replace final visual and compositor approval.

## Stage 1 verification, October 1, 2026

The [saved preview](design-previews/2026-10-01-stage1/README.md) uses disposable notes and real GTK rendering. All four appearances were reviewed at the compact 720x500 All Notes size, 352x376 default individual size, 300x280 minimum, and 600x500 larger size. Opening the floating tool panel preserves the note dimensions. Native separate-note resizing and pointer dragging on all four docked edges passed. Separate notes opened floating without adding a desktop rule. Escape closed separate and docked notes with Always open and tools enabled.

Formatting, strict Clippy, the 20 non-display tests, the existing shared-editor GTK regression, release build, and diff checks passed. These checks validate this pass, not final UI approval or the complete phase 5 performance/compositor checklist. Text sizes remain provisional. The subsequent [individual tools revision](design-previews/2026-10-01-tools/README.md) implements size/weight, Settings-only font family and individual Pin, with verified focused-panel Escape and failed-save retention. The remaining controls were subsequently completed in the pass below. Edge redesign, Settings and checklist work remain pending.

## Note controls completion, October 1, 2026

The user authorized the remaining note controls. All Notes More now includes Checklist, bullets/numbering, S/M/L, Regular/Bold, tag colors and the existing pattern override. Its footer remains Pin / Pop out / More. Copy, Archive/Restore and Delete stay in More, in a compact row; archived editing controls are disabled. Both editors use the same concrete controls and shared session.

The palette has 16 choices in a four-column grid. Existing Amber, Blue, Lilac and Sage values/shades remain intact. Schema 3 rebuilds the constrained notes/style tables in one transaction, preserving content, IDs, pin/order state, timestamps, archive/delete state and style TOML. The tests also cover rollback on migration failure.

Individual notes have no routine save indicator or error banner. Save failures use plain-text toasts in visible views, deduplicated by note/message, and stale error toasts clear after successful retry or a selection change. The shared session retains failed edits and its Close/switch guards. All Notes keeps its compact save indicator, with reduced-motion handling. Individual Pin continues to mean Always open; ordering Pin stays in All Notes.

Formatting, strict Clippy, 22 non-display tests, the shared-editor GTK regression and release build pass. Disposable GTK checks confirm shared controls/fonts/colors, persistence, error toasts and archive/restore behavior in all four appearances. The [controls preview](design-previews/2026-10-01-note-controls/README.md) records verification and remaining limits. The locked desktop prevented a new live Escape/focus and pointer-resize check; those remain pending. Numeric sizes remain provisional; Settings/checklist design and edge implementation are pending. No commit or push was authorized for this pass.

## Next scope, not started

Review the controls and size previews. The next planned implementation pass is the four-note vertical edge strip and integrated search, after a concrete scope is authorized. Settle the Settings host/behavior and obtain the checklist item specification before their passes. The wider compositor, login and performance checks remain open.

## Reference polish pass

The user authorized polishing the implemented UI against the saved references. All Notes now uses a stronger heading, full-width collection filters, rounded search and filter controls, and more space in list rows. Shared tools have a clearer Aa heading, lighter swatch borders and rounded panels. Playful sheets use a slightly heavier outline and softer offset shadow. This pass changes presentation only. Settings, edge redesign, checklist behavior and final text sizes remain pending.

Validation: 22 tests passed, strict Clippy and release build passed. Disposable GTK checks passed for shared styles and all 16 colors, archive/restore, failed-save retention and error toasts. Default and minimum note dimensions passed. Native pointer and Escape checks were not repeated. See design-previews/2026-10-01-polish for window renders.
