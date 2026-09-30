# UI implementation plan

Date: September 30, 2026. The user authorized stage 1 and a planning checkpoint commit on October 1, 2026. Application code is unchanged at this planning checkpoint. The active milestone remains final UI work and phase 5A/5B verification.

## Sources and scope

The [saved reference set](design-references/2026-09-30/README.md) contains the original UI/UX document, nine images, and the two latest SVG packs. This plan records the user's later clarifications. Those clarifications take precedence over conflicting reference text or example imagery.

Use the current Rust, GTK4/libadwaita, Layer Shell, SQLite, and shared `NoteSession` implementation. Preserve note data, live cross-view editing, autosave, failed-save retention, archive/restore, deletion Undo, and Copy text. Keep existing module responsibilities. No replacement editor, generic event system, new UI framework, or independent note buffers are needed.

Multi-monitor work, sync, custom/manual ordering, and release expansion remain later roadmap work. Settings and checklist redesign depend on further user input. Stage 1 is authorized; later implementation stages and push still require their own authorization. The requested checkpoint commit covers the saved references and plan before stage 1 changes.

## Agreed target

| Area | Intended behavior |
|---|---|
| All Notes window | Keep the compact 720x500 default and resizing. Retain the 240px list and current responsive drawer arrangement until the settings design is supplied. |
| Main note actions | Exactly three footer buttons: Pin, Pop out, and More. More contains checklist, text style, tag colors, and other existing note actions. |
| Individual notes | Both docked and separate notes are resizable, with matching minimum sizes and a subtle hover-enhanced corner grip. Keep Close visible and long content inside a wrapping, vertically scrolling editor. |
| Note toolbar | Hidden by default. A subtle opening control becomes clearer on hover; clicking opens the toolbar. Provide the size, weight, and color controls directly in a compact layout based on `tools.png`. Keep the toolbar collapsible and accessible by keyboard. |
| Pin in All Notes | Persistent ordering preference. Pinned active notes remain ahead of unpinned active notes. |
| Pin in an individual note | Always open for that view. It does not change persistent note ordering. Replace the separate infinity control with this role. Make the role clear in the tooltip and accessible label. |
| Inactivity | Normal individual views close after two minutes without input. Always open cancels the timer; turning it off starts a fresh two-minute period. Keep independent timers for each view. All Notes has no timeout. Close and Escape remain available with Always open enabled. Preserve failed-save protections. |
| Fonts | Keep Virgil, Inter, and JetBrains Mono as note-content choices. A note's choice applies in the individual note and All Notes editor. App headings, lists, menus, and buttons use the UI font. |
| Text sizing | Final title/body sizes and S/M/L values remain undecided until an actual preview. The proposed 18/21/24 values were not approved. Existing sizes may be used provisionally for the first review. |
| Corners | Use rounded corners throughout the interface, including Minimal. Keep Paper's torn bottom and Playful's wavy outline as their edge treatments. |
| Paper | Warm paper, soft shadow, shallow torn bottom, and a tag-colored side strip. The All Notes editor and individual note share the same appearance. |
| Minimal Light/Dark | Fully opaque note fills, rounded corners, a tag-colored top strip, and soft shadows. Dark All Notes follows the light layout with dark colors. |
| Playful | Tag-colored note fill, dark wavy outline, offset shadow, and pink theme accent. The pin badge shows the relevant pin state for its view. |
| Sidebar tag markers | Playful uses dots. Paper and Minimal use vertical bars. Marker color follows the note's tag. Selection styling remains separate from tag color. |
| Color choice | Expand beyond the current four colors to common colors in a compact swatch grid. Use the same chosen tag color for sidebar/edge markers, Paper/Minimal accents, and Playful fill. Do not add a separate Playful background-color setting. |
| Save feedback | Routine Saving/Saved status appears only in All Notes. Save failures use toasts in the affected visible view, including individual notes. Keep failed edits dirty and preserve the existing protection against discarding them. |
| Archived notes | Keep the note's theme and appearance. Editing stays disabled until Restore. Retain a clear archived indication and Restore action rather than inventing another visual theme. |
| Empty and no-result states | An empty collection shows a short No notes yet message and New note action. A search with no matches shows No matching notes and a way to clear the query. Keep existing data intact and do not seed example notes. |
| Decorations | No automatic title underline. When underlines or strike-through are used with Virgil, their appearance should be scribbled. Checklist completion styling awaits the user's checklist behavior specification. A general rich-text underline feature is not defined by this visual request. |
| Edge notes | Four active notes in the same order as All Notes. Custom ordering is later work. Replace the adjacent/overlapping designs with the single vertical title-tab design from the edge image. |
| Edge interaction | Resting tag dots, hover-revealed tabs, New note/Search/All Notes utility controls, and integrated search. Clicking opens one editable docked note. Its theme and tag treatment match the same note elsewhere. Include the decorative pointer shown in the reference. |

An open docked note hides the tab list. Hover does not replace it with another note. Close or Escape returns to the tabs. Opening away from the edge creates or focuses a separate window without removing the note from the deck. Several different notes can remain open. Preserve the existing behavior for these interactions unless a later approved decision changes it.

## Pending decisions

| Item | What remains needed | Can other work proceed? |
|---|---|---|
| Settings | User will supply the design. Settle its sections, controls, edge repositioning, and any shortcut choices then. | Yes, keep the current settings available. |
| Checklist | User will supply appearance and item behavior. Existing clickable checkboxes are functional but not visually approved. | Yes, preserve them while doing the other UI work. |
| Text sizes | Review real note content at the intended compact dimensions before selecting title/body sizes and S/M/L values. | Yes, use current sizes provisionally. |

Shared edge ordering is specified; carrying the All Notes search query or Archived filter into the edge is not. Treat ordering and collection membership separately. If a different edge membership rule is wanted, settle it before implementation.

The references do not settle whether underline should be a user-applied action. Do not infer a rich-text system from decorative example lines. Revisit only if the checklist specification or another direct request needs it.

## Implementation order

### 1. Compact visual preview

Responsibilities: `appearance.rs`, `assets/all-notes.css`, `editor.rs`, `manager.rs`, `main.rs` for resizing, and `ui_assets.rs` where icon presentation needs adjustment.

- Bring all four appearances toward the saved references while retaining the compact All Notes size.
- Use matching note fills, tag accents, corner treatments, borders, and shadows in the All Notes editor and individual note. Scale geometry to available space without scaling the chosen text size automatically.
- Reuse `sheet_path` and `edge_path` for Paper tears and Playful waves. Tune the result visually; no separate `wobbly_border.rs` implementation is required.
- Show only the three main footer actions. Build a compact expandable individual toolbar and a subtle hover-enhanced opening control. Fit its width to the inline controls rather than adopting the original document's 40px width blindly.
- Make the editor layout flexible and allow both docked and separate notes to resize. Use matching minimum sizes and a subtle hover-enhanced corner grip. Use the window's normal resize path for separate notes and update the docked editor's requested size for the Layer Shell view. Keep an edge-docked note anchored and within available space during resizing. Preserve safe text margins, wrapping, and vertical scrolling. Avoid reserving a large empty toolbar gutter while the toolbar is closed.
- Keep the current text sizes provisional. Present the same sample note in All Notes and an individual window, with toolbar closed/open, and review actual title/body size, padding, and control proportions.

Acceptance: Paper, Playful, Minimal Light, and Minimal Dark remain readable at the compact window size; note appearance matches between views; the toolbar stays unobtrusive when closed; both docked and separate resized notes neither clip controls nor stretch border shapes badly. Docked resizing remains within available space on each edge. User reviews this preview before final text sizes are committed.

### 2. Note controls, colors, pin roles, and save feedback

Responsibilities: `editor.rs`, `manager.rs`, `note_session.rs`, `app.rs`, `config.rs`, `storage.rs`, `migrations`, styles, and bundled icons.

- Wire the same note tools into the individual toolbar and All Notes More surface. Keep Copy text, bullet/numbered lists, Archive/Restore, and Delete available where applicable through the existing coordinator/actions. Use existing concrete controls and handlers; share small pieces only where both views need them.
- Keep ordering pin state in `NoteSession`. Use the existing per-view Always open toggle and timer for the individual pin icon, so it cannot reorder the note. Preserve explicit close behavior and independent timers.
- Expand the shared color choices and preserve the meanings of existing saved colors. The existing SQLite `notes.color` CHECK constraint allows only four values, so include a small transactional migration for the additional colors. Preserve note IDs, text, dates, pin/order values, archived/deleted state, and related note styles. Update storage parsing, CSS, drawing, and both pickers together. Use a compact grid with readable selected/focus states. Exact shades are calibrated during preview.
- After the size review, extend the existing per-note appearance data only as needed for note-wide text size and Regular/Bold. `note_styles.settings` already stores serialized TOML, so compatible defaulted fields may be sufficient for these style additions without another database migration. Do not revive the retired global `font_size` setting.
- Keep the selected note font and appearance live across all views and after restart.
- Remove routine save status from individual headers. Show save-error toasts when a new failure occurs, without repeating the same toast on every buffer notification. Keep the error and dirty text in the existing session so a disappearing toast cannot discard failed edits.
- Import or create only the SVGs actually used by the approved controls. Extend the established SVG style rather than generating raster icons. Review the Aa/color/filter/grid gaps noted in the reference inventory and replace the retired controls where needed.

Acceptance: changing a note's color/font/style updates every live view; migrated databases preserve existing notes and styles; new colors survive restart; individual Pin only controls that view's timeout; main Pin controls order; error toasts appear in the affected view and save retry still works.

### 3. Edge strip redesign

Responsibilities: `main.rs`, `app.rs`, `storage.rs`, `manager.rs`, `config.rs`, appearance, and any needed icons.

- Limit the edge deck to four active notes. Share the main list's ordering choice through the existing coordinator and storage paths. Preserve pinned priority. Exclude archived/deleted notes and keep overflow available in All Notes.
- Replace the two current tab layouts with the reference's vertical design. Adapt placement to all four edges, clamp the cluster and docked note to available space, and preserve readable titles and useful hit targets. Accept older configuration safely if the old layout setting is retired.
- Implement resting dots, a short hover dwell, tabs, and the three rectangular utility controls. Put them in the arrangement from the reference; do not retain the older opposite-end arrangement by accident.
- Add search within the strip. Give its input/results a readable width, search active notes beyond the four visible slots, share the existing search behavior where possible, and support keyboard selection and Escape. A result opens the existing shared session as the docked note. The original document's Enter-with-no-match action creates a note from the query; show that action clearly in the no-result state and verify that it updates All Notes and the edge. No separate centered search window is needed.
- Keep the docked note's resolved theme, tag treatment, buffer, and explicit close behavior. Add the small non-interactive pointer without changing its input region or obscuring text.
- Gate custom motion on reduced motion and system-disabled animation. Keep hover leave grace and prevent accidental edge grazes from opening the deck.
- Retain the existing reposition behavior until the forthcoming settings design settles its replacement. Do not silently introduce a long-press or customize mode from the draft document.

Acceptance: creating a blank note updates the edge immediately; the four active notes match All Notes order; typing in edge search does not corrupt ordering; click/keyboard actions work on every edge; hover cannot replace an open docked note; reduced motion and fullscreen/DND behavior still work.

### 4. Checklist and settings after their specifications arrive

Checklist responsibilities: `note_session.rs`, `editor.rs`, `manager.rs`, and note styles/drawing. Preserve the shared buffer, plain-text markers, Copy text, and persistence unless the approved checklist behavior explicitly changes them. Use the existing `TextChildAnchor`/`CheckButton` work as the starting point, and compare its result with the user's design. Implement hand-drawn completion marks only after their appearance and behavior are settled.

Settings responsibilities: `settings.rs`, `config.rs`, `manager.rs`, and coordinator calls for existing desktop preferences. Integrate the supplied design into the compact window. Keep live appearance updates, per-note overrides, and rejected-write recovery. Do not add draft settings sections merely because they occur in the original document.

Adapt the [manual checklist](manual-testing.md) to the implemented design, including the single edge layout, the two pin roles, and any approved resizing behavior. If product terminology needs changing, update the relevant context document at that point rather than renaming the domain model to match an icon.

### 5. Final verification and review

- Run `cargo fmt --check`, strict Clippy, the existing non-display tests, and a release build after the relevant code changes.
- Run the existing shared-editor GTK regression when shared editing, anchors, or session behavior changes. Add focused coverage for actual new behavior or regressions, such as old/new color persistence, note-style defaults, and shared ordering. Avoid tests that repeat CSS values or widget construction.
- Use the disposable-data procedure in `manual-testing.md`. Review both editing views across all four appearances, several fonts, toolbar states, resized windows, long content, empty searches, archive/restore, and error toasts. Confirm autosave and Copy text use the latest shared content.
- Verify the timer with Always open on/off, a fresh two-minute period after disabling it, manual Close/Escape, and independent activity in multiple windows.
- Verify edge pointer targeting, search keyboard focus, all four anchors, hover dwell/leave, reduced motion, and fullscreen/DND hiding without losing open notes.
- Recheck resource use against the current milestone budgets, installed launch after login, floating/movement behavior, and any recurring graphics warning. A successful build is not final visual or compositor approval.
- Keep the open findings in [code-audit.md](code-audit.md) visible. Menu retention and missing creation refresh intersect this work and should be handled in the approved passes that touch them. Settings-write recovery and the CI dependency finding also need their own approved fixes; do not declare the milestone complete while required checks remain open.

## Authorized first implementation pass

Start with stage 1's compact visual preview: consistent note appearance, rounded controls, three-button main footer, a hidden compact toolbar, and resizable docked/separate notes. Keep text sizes provisional, reuse the existing border drawing and save/session model, and leave pending checklist/settings behavior in place. Verify the preview across all four appearances before proceeding to the controls/persistence and edge passes.

The stored originals remain unchanged throughout implementation. Record later approved decisions in this plan and `current-milestone.md`, and update the implementation checks as behavior changes.
