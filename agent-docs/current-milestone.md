# Current milestone

## Status and next steps

The core note app, All Notes window, and Paper, Playful, Minimal Light/Dark appearances are implemented. The latest application-code checkpoint is `fe97c94`, including individual-note/checklist corrections and unused-code cleanup. Visual approval is pending, and the edge-strip redesign remains unfinished.

Documentation cleanup, the code audit and the approved unused-code cleanup are complete. The UI checkpoint is committed locally. On September 30, 2026, the user supplied final visual references and clarified the next UI revision. The originals are saved in [design-references/2026-09-30](design-references/2026-09-30/README.md), and the work order is in [ui-implementation-plan.md](ui-implementation-plan.md). Storing the references and writing the plan did not change application code.

Settings, checklist design/behavior, and final text sizes remain pending. Both docked and separate notes are to be resizable. On October 1 the user authorized stage 1, the compact visual preview, after committing the saved references and plan. Separate audit findings remain open in code-audit.md. Complete the outstanding checks before declaring phases 5A and 5B finished. Do not start multi-monitor or sync implementation during this milestone.

The remaining product scope and work order are in [implementation.md](implementation.md).

## Agreed UI revision, pending implementation

The September 30 decisions below supersede conflicting details in the saved original UI/UX document and earlier UI plans. The application still runs the committed checkpoint until these changes are implemented and verified.

- Four edge notes in the same ordering as All Notes. Replace the two existing title layouts with the reference's vertical tabs. Use resting dots, New note/Search/All Notes utility controls, integrated search, and one theme-following docked note with a decorative pointer. Custom ordering remains later work.
- Keep All Notes compact at a 720x500 default and resizable. Its footer stays Pin / Pop out / More. More contains the other note tools. Keep the existing 240px list and responsive drawer arrangement until the settings design arrives.
- Make both docked and separate individual notes resizable, with matching minimum sizes and a subtle hover-enhanced corner grip. Preserve edge anchoring, wrapping, and vertical scrolling for long content.
- Keep the individual toolbar hidden by default. Its subtle opening control becomes clearer on hover and opens on click. The expanded toolbar shows compact size, weight, and color controls based on `tools.png`. Keep Close visible. Routine save status appears only in All Notes; errors use toasts in the affected view.
- All Notes Pin remains a persistent ordering preference. The individual pin icon instead controls that view's Always open behavior. Keep independent view timers; turning Always open off starts a fresh two-minute period. Close and Escape remain available, with existing failed-save retention preserved.
- Keep Virgil, Inter, and JetBrains Mono as note-content choices, applied to the same note in both editors. UI chrome uses the UI font. Final title/body sizes and S/M/L values await an actual preview; the current sizes are provisional.
- Use rounded corners throughout, including Minimal. Retain existing Paper tear and Playful wave drawing. Paper has a tag-colored side strip and Minimal a tag-colored top strip in both note views. Playful's note fill follows its tag color. Sidebar markers are dots in Playful and vertical bars in Paper/Minimal. Expand the shared color palette in a compact grid.
- Minimal Dark All Notes uses the light layout with dark colors. The title underline in the references is example content, not automatic decoration. Handwritten underlines and strike-through should appear scribbled when used; the action for general underline is not defined by that visual request.
- Archived notes retain their normal appearance and remain read-only until restored. Existing clickable checklists need visual/behavior redesign; wait for the user's item specification. Settings also awaits the promised design.

## Existing behavior to preserve

- Pinned active notes precede unpinned notes. Overflow stays available in All Notes. Opening a docked note hides the titles. Closing or Escape restores them. Hover does not replace an open note.
- Open away from edge creates or focuses a separate window without removing or reordering the note. Several different notes can remain open. No attachment/detachment animations.
- The current All Notes drawer is 260px and overlays below 980px. Individual windows currently have fixed dimensions; resizing belongs to the revision above.
- All Notes has a directly editable note, shared live title/body with individual windows, search/sort, and Pin / Pop out / More actions. Copy text copies the live body. Archive, restore, and deletion Undo are implemented.
- All / Active / Archived are visible filters. Archived notes remain read-only until restored. There is no pull-down archive gesture or collection menu.
- Docked and separate notes close after two minutes without input by default. Always open disables that view's timeout. The All Notes editor has no inactivity timeout. The existing individual toolbar has separate ordering Pin and Always open controls; the revision above changes their presentation and roles.
- Appearance changes apply immediately. Keep theme, font, relevant paper patterns, reduced motion, and Minimal System/Light/Dark controls, with existing per-note overrides respected.
- Virgil is the decided handwritten font. Inter and JetBrains Mono are the other choices. A reference image is a visual guide, not permission to replace agreed fonts.
- Paper uses soft ivory, a shallow torn bottom edge and soft shadow. Playful uses colored notes and dark outlines. Minimal uses opaque light/dark notes and a thin top accent. The current square Minimal corners are to be replaced by rounded corners in the revision.
- Current body text is 21px. Individual Paper/Playful titles are 23px and Minimal titles 20px. These are implementation values, not final approval of text sizes. The current individual header includes save status, to be removed by the revision.
- Live checklists use clickable native checkboxes and persist as plain-text markers. Backspace at an item's start removes its checkbox. Archived previews currently show markers. Preserve this working foundation until the revised item behavior is approved. Existing bullets and numbered lists remain; broader Markdown and export are not planned.

## Code responsibilities

- `main.rs`: edge/window coordination and weak session lookup by note ID.
- `note_session.rs`: shared title/body buffers, note appearance and pin state, pending saves, and save errors. Failed body saves retain dirty text and block operations that would discard it.
- `editor.rs`: individual-note presentation, editing controls, and per-view inactivity timers.
- `manager.rs`: All Notes selection, filtering, actions, and embedded editor.
- `app.rs`: deck state and ordering.
- `appearance.rs`, `assets/all-notes.css`, `settings.rs`, `ui_assets.rs`: drawing, styles, Appearance controls, bundled fonts and icons.
- `storage.rs`: SQL and migrations. Schema 2 stores per-note appearance separately.
- `platform.rs`: event-driven Omarchy DND state watching, with no desktop configuration changes.

## Remaining acceptance checks

Use [manual-testing.md](manual-testing.md) for the current checkpoint and [ui-implementation-plan.md](ui-implementation-plan.md) for the revision's acceptance checks. Update the manual checklist as the revision lands.

- Final user review of All Notes, individual notes, themes, toolbar controls, and text sizes. Incorporate the pending checklist and settings designs.
- Finish the single vertical edge-strip design and integrated search. Verify four-note ordering, pointer targeting, title readability, all four edges, keyboard focus, and reduced motion.
- Verify the two-minute timeout and Always open with the final UI.
- Verify fullscreen hides the strip while open notes stay visible. DND currently reads Omarchy shell state; other notification servers are not supported yet.
- Verify installed launch after login. A generated autostart entry alone does not prove this works.
- Recheck window movement/floating. An All Notes floating rule exists in the reference user's desktop configuration; portable installation must not assume every user has it.
- Investigate the graphics invalid-rectangle warning if it recurs. It was not reproducible in the later checks and is not declared fixed.
- Remeasure the final UI: at most 200 MiB PSS, ordinary interaction below 2% CPU, normal animation below 3%, exceptional one-second spikes at most 5%, idle returning to 0%. Use interval measurements with labeled activity. A prior comparison baseline was about 74 MiB PSS; it is not a final UI pass.

## Verification status

The current audit checks passed formatting, strict Clippy, 20 non-display tests, release build and diff checks. The existing GTK shared-editor regression passed in isolation; its previous fixed-deadline timing failure remains noted. Disposable checks confirmed menu retention, a missing edge refresh after blank-note creation, and settings controls remaining on rejected values. See [code-audit.md](code-audit.md) for findings and the proposed fixes. These checks do not replace final visual and compositor approval.
