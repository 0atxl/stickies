# Current milestone

## Status and next steps

The theme and Appearance review is approved and committed/pushed as `b63a1b1`, "Polish reviewed themes and unify Appearance drawer". Paper, Playful, Minimal Light and Minimal Dark retain their approved note appearance and text sizes. The [theme gallery](design-previews/2026-10-01-theme-review/unified-appearance/preview.html) records that checkpoint.

The October 7 lightweight edge implementation keeps titled tabs and utilities beside an open note or Search panel. The [new GTK gallery and recordings](design-previews/2026-10-07-lightweight-edge/preview.html) and [review record](design-previews/2026-10-07-lightweight-edge/README.md) cover the implementation and verification. Both layer windows retain fixed screen geometry; edge state changes and CSS feedback have no animation. Tabs/Search keep transient fullscreen/DND policy, while committed notes retain their visibility protection. The approved note appearances and previous uncommitted work remain. The user authorized committing this checkpoint on October 7. No push or further implementation is authorized by that request. Wider milestone checks remain open. Earlier dated sections record historical behavior and review states.

The current milestone remains final UI work and phase 5A/5B verification. Settings host/behavior, checklist redesign, installed login launch, final performance and remaining compositor checks are unfinished. Multi-monitor, sync and custom ordering remain later work. The [implementation plan](ui-implementation-plan.md), [manual checklist](manual-testing.md) and [open audit findings](code-audit.md) define the remaining scope. The stored reference documents are visual guidance; direct user decisions take precedence.

## Agreed UI revision

Direct user decisions below take precedence over conflicting details in both saved UI/UX versions and earlier UI plans. Stage 1 implements appearance, layout, floating tools and resizing. Individual Pin, size and weight are also implemented. The remaining note controls are implemented. The edge strip is implemented and under review. Settings and checklist work follow the implementation plan and need their own authorization.

- Four edge notes in the same ordering as All Notes. Replace the two existing title layouts with vertical tabs on Left/Right and upright horizontal tabs on Top/Bottom. Use resting dots, New note/Search/All Notes utility controls, integrated search, and one theme-following docked note with a decorative pointer. Custom ordering remains later work.
- Keep All Notes compact at a 720x540 default and resizable. Its footer stays Pin / Pop out / More. More contains the other note tools. Keep the existing 240px list and responsive drawer arrangement until the received Settings draft's host and width are settled.
- Make both docked and separate individual notes resizable, with matching minimum sizes and a subtle hover-enhanced corner grip. Preserve edge anchoring, wrapping, and vertical scrolling for long content.
- Keep the individual toolbar hidden by default. Its subtle opening control becomes clearer on hover and opens on click. The expanded toolbar shows compact size, weight, and color controls based on `tools.png`. Keep Close visible. Routine save status appears only in All Notes; errors use toasts in the affected view.
- All Notes Pin remains a persistent ordering preference. The individual pin icon instead controls that view's Always open behavior. Keep independent view timers; turning Always open off starts a fresh two-minute period. Close and Escape remain available, with existing failed-save retention preserved.
- Keep Virgil, Inter, and JetBrains Mono as global Settings-only note fonts, applied in both editors. Older per-note font overrides load but no longer override Settings. UI chrome uses the UI font. Individual tools show S/M/L and Regular/Bold; these persist per note and update both editors. Paper preview body sizes are 14/17/20px and titles 27/30/33px. Other themes retain body sizes 18/21/24px, with titles adjusted by the same 3px steps. Final sizes still await review.
- Use rounded corners throughout, including Minimal. Retain existing Paper tear and Playful wave drawing. Paper has a tag-colored side strip and Minimal a tag-colored top strip in both note views. Playful's note fill follows its tag color. Sidebar markers are dots in Playful and vertical bars in Paper/Minimal. Expand the shared color palette in a compact grid.
- Minimal Dark All Notes uses the light layout with dark colors. The title underline in the references is example content, not automatic decoration. Handwritten underlines and strike-through should appear scribbled when used; the action for general underline is not defined by that visual request.
- Archived notes retain their normal appearance and remain read-only until restored. Existing clickable checklists need visual/behavior redesign; wait for the user's item specification.
- Settings reference received: one scrollable surface with Appearance, Edge Strip, Behavior, Shortcuts and About sections. The spec explicitly keeps the section list draft/open. Use app UI typography, dividers, switches and outlined selected segments. Settle the host/width, preserve Minimal System/Light/Dark, clarify horizontal position presets and Reposition commit/cancel behavior. The strip-enable switch must not be confused with the existing hover-enable preference. Default new-note color should use the shared palette; four image samples do not limit it. Shortcut badges are examples, not currently working global bindings.

## Existing behavior to preserve

- Pinned active notes precede unpinned notes. Overflow stays available in All Notes. Tabs and utilities remain visible and stationary beside docked notes and Search. Closing or Escape returns to the same expanded strip position. Clicking a different tab switches notes after a successful flush; hover never replaces an open note. Failed saves retain the editor. Edge reveal, collapse, switching and resizing have no animation, including with reduced motion disabled.
- Open away from edge creates or focuses a separate window without removing or reordering the note. Several different notes can remain open. No attachment/detachment animations.
- The current All Notes drawer is 260px and overlays below 980px. Individual notes default to 352x376 and resize to a minimum of 300x280. The tool panel floats beside the note without changing those dimensions.
- All Notes has a directly editable note, shared live title/body with individual windows, search/sort, and Pin / Pop out / More actions. Copy text copies the live body. Archive, restore, and deletion Undo are implemented.
- All / Active / Archived are visible filters. Archived notes remain read-only until restored. There is no pull-down archive gesture or collection menu.
- Docked and separate notes close after two minutes without input by default. Individual Pin means Always open and disables that view's timeout. The All Notes editor has no inactivity timeout. Ordering Pin is in All Notes; infinity and individual ordering Pin have been removed.
- Appearance changes apply immediately. Keep theme, global font, relevant paper patterns, reduced motion, and Minimal System/Light/Dark controls. Respect per-note pattern, shape, size, and weight choices.
- Virgil is the decided handwritten font. Inter and JetBrains Mono are the other choices. A reference image is a visual guide, not permission to replace agreed fonts.
- Paper uses soft ivory, a shallow torn bottom edge and soft shadow. Playful uses colored notes and dark outlines. Minimal uses opaque light/dark notes and a thin top accent. Stage 1 gives Minimal rounded corners and matching tag accents in both editing views.
- Medium Paper body text is 17px and its title is 30px. Playful uses a 30px title and 17px body; Minimal Light and Dark use 28px titles and 17px bodies. Small/Large apply provisional minus/plus 3px adjustments. These are implementation values, not final approval of text sizes. Routine save status is now limited to All Notes. Each view shows deduplicated save-error toasts and dismisses its stale error toast after successful retry.
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

- Review the new edge strip and apply any requested corrections. Settle the Settings host/behavior and obtain the checklist item design before their implementation passes.
- Review the implemented edge-strip previews. Complete any visual corrections within this pass before proceeding to Settings or checklist work. See its review record for native and GTK checks.
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

## All Notes window sizing

The user reviewed the larger window and requested restoration of the compact layout. All Notes returns to a 720x500 default, 680x460 minimum, 240px sidebar and the original 420px preview limits. Reference fidelity should be improved through proportions and styling within that size. Individual note sizes and committed polish remain unchanged.

## Focused Paper reference pass

The user approved a Paper-only visual pass at 720x500. Paper now has a larger title relative to its body, smaller checkbox chrome, warmer ivory and peach selections, a wider tag strip and a less uniform torn edge. Reduced header/editor padding gives the note more space without enlarging the window. Sidebar separators and clearer footer icons follow the reference. Paper tools use warm segmented selections. Existing behavior, fonts and other themes remain intact; numeric sizes still await visual approval.

22 tests, strict Clippy, formatting and release build passed. Disposable GTK renders compare the reference content at the default size and check the 300x280 individual-note minimum. Live keyboard/pointer checks were not repeated.

## Paper height, dates and tear follow-up

The user approved a small height increase to 720x540, retaining the 680x460 minimum and 240px sidebar. Sidebar titles now have compact saved updated dates, refreshed after edits. Paper uses slightly warmer window chrome while tools remain nearly white. The seeded tear now mixes irregular small breaks with broader shallow contours instead of uniform zigzags. The preview includes a long sidebar title to check truncation beside its date.

Paper spacing follow-up approved: keep the divider beneath the sidebar filters only, removing its continuation above the note preview, flatten filter pills to 24px, use 11px sidebar snippets and provisional 17px Medium note body text. Heading sizes remain unchanged.

## Playful review cycle, October 1, 2026

Implemented only the authorized Playful polish. Medium uses a provisional 30px title and 17px body, with Small/Large at minus/plus 3px and Regular/Bold at 400/700. Playful has compact checkbox chrome and content spacing, round sidebar dots, 11px snippets, 24px filters, a sidebar-only divider, near-white chrome and pink selected controls. Its stable outline uses broader smooth waves and a visible 6x7px offset shadow. Supplied Playful icons and existing behavior are preserved.

The [review record](design-previews/2026-10-01-theme-review/playful/README.md) includes exact typography, matching Weekend Plans/Today's Plan fixtures, GTK main/default/minimum/tools captures, regression renders and remaining reference differences. Formatting, strict Clippy, 22 non-display tests, the existing GTK shared-editor regression, release build and diff checks passed. Disposable checks verify shared title/body sizes, weight, colors/persistence, global fonts and unchanged note sizes with tools open. Native pointer/Escape/timer checks were not repeated. Paper and Minimal appearances are preserved; their renders are regression checks only.

| Theme | Current review status |
|---|---|
| Playful | Under review; not visually approved |
| Minimal Light | Not started; requires explicit approval and authorization |
| Minimal Dark | Not started; requires explicit approval and authorization after Minimal Light |

No theme in this three-theme cycle is approved. Stop for Playful review. Settings, edge, checklist behavior and later roadmap work remain outside this scope. Changes remain uncommitted.

## Playful sidebar correction

The user requested another Playful polish pass for the missing filter-band sidebar edge and selected-row separation. Playful CSS now continues the vertical edge through the filters and draws straight row separators outside the rounded selection/hover fills. A 5px gap separates highlight and divider. Sidebar dates and truncation remain intact. Typography and behavior are unchanged.

The [fresh GTK review](design-previews/2026-10-01-theme-review/playful/sidebar-polish/preview.html) includes normal selection, selected hover, a long selected title, default/minimum notes and tools. Formatting, 22 tests, Clippy, release build, diff and disposable GTK typography/control/dimension checks passed. Other themes were inspected; this follow-up changes only Playful CSS. Native pointer/Escape/timer checks were not repeated. Playful is still under review; Minimal Light and Minimal Dark have not started. No commit or push.

## Minimal Light review cycle

The user approved Playful and authorized Minimal Light next. Minimal Light now uses a provisional 28px title and 17px body at Medium, with S/M/L at 25/28/31px titles and 14/17/20px bodies. Inter remains its default; the gallery also compares the reference content in Virgil. Light has compact checkboxes, opaque white notes with tag-colored top strips, a restrained soft shadow, blue selected tools and compact chrome. Sidebar bars, dates, truncation and separate highlight/divider treatment are preserved.

The [Light review record](design-previews/2026-10-01-theme-review/minimal-light/README.md) includes actual 720x540 main, 352x376 default, 300x280 minimum, open tools, matching Weekend Plans/Today content and other-theme regression captures. Formatting, 22 tests, strict Clippy, release build, the existing GTK shared-editor regression, diff checks and disposable control/font/dimension checks passed. Native pointer/Escape/timer checks were not repeated. Paper, approved Playful and Minimal Dark appearances are preserved; shared code changes contain Light-only branches.

| Theme | Current approval status |
|---|---|
| Playful | Approved after sidebar correction |
| Minimal Light | Implemented, under review; not approved |
| Minimal Dark | Not started; not authorized |

Earlier entries record historical approval states; this table is current. Stop for Minimal Light review. No commit or push is authorized.


## Minimal Dark review cycle

The user approved proceeding from Minimal Light and authorized Minimal Dark. Dark has its own provisional 25/28/31px titles and 14/17/20px bodies, retained Inter default, compact 14px checkboxes, opaque charcoal notes with light outlines, restrained shadows and tag-colored top strips. Its main layout follows approved Light proportions. Dark chrome uses separate hover, selection and blue focus colors, with dividers outside rounded sidebar highlights.

The [Dark review record](design-previews/2026-10-01-theme-review/minimal-dark/README.md) and [gallery](design-previews/2026-10-01-theme-review/minimal-dark/preview.html) contain fresh actual GTK main/default/minimum/open-tools, matching Weekend Plans/Today content, long titles/dates and control-state captures. Formatting, 22 tests, strict Clippy, release build, the existing GTK shared-editor regression, diff and disposable typography/control/persistence/dimension checks pass. Hover/focus are GTK state previews. Native pointer/Escape/timer/edge checks were not repeated.

Shared drawing, font and paragraph-spacing changes affect only Dark branches. Matched regression captures preserve approved Light exactly and Paper/Playful apart from blinking caret pixels. No shared design-system refactor or desktop configuration changes were made.

| Theme | Current approval status |
|---|---|
| Playful | Approved after sidebar correction |
| Minimal Light | Approved; user authorized Dark next |
| Minimal Dark | Implemented, under review; not approved |

This table supersedes historical approval states above. Stop for Minimal Dark visual review. No commit or push.


## Filter-band and first-row correction

The user's screenshot identifies a crowded filter band and inconsistent spacing before the first note in every theme. The user explicitly authorized fixing one theme first and copying the verified spacing to the others. This is a narrow exception to the theme-by-theme review order, limited to these two spacing values.

Minimal Dark was corrected and rendered first. Its GTK measurements confirmed a 44px filter band around a centered 24px pill, with 10px padding above/below and an 8px gap from the sidebar divider to the first note. Those padding values were then copied into the existing Paper, Playful and Minimal Light CSS rules. All four themes match these measured values. Theme colors, border/selection treatment, typography, icons, row heights and existing behavior retain their prior values. No shared design-system refactor was added.

The [reference comparison and four-theme review](design-previews/2026-10-01-theme-review/filter-spacing/preview.html) links separate theme directories with actual GTK main/default/minimum/open-tools, hover and long-title/date captures. Default individual notes and tools are pixel-identical before/after. Minimum note variations are confined to caret/scrollbar frames. Formatting, 22 tests, strict Clippy, release build, diff checks and the GTK shared-editor regression pass. Disposable GTK assertions verify exact spacing, window/note sizes, selected content and tools preserving note dimensions. Native pointer/Escape/timer checks were not repeated.

Playful and Light were approved before this requested correction. The spacing correction is now under review across all themes; Dark remains under review. No commit or push.


## Corners, spacing, Appearance and canvas-pattern correction

The user authorized fixing inconsistencies across themes, placing Appearance on the filter row, and making dot/plain/square patterns affect the canvas. This request authorizes the narrow cross-theme and Settings corrections below. It does not start the separately planned Settings redesign, edge work or checklist semantics.

Main headers now have 58px content height in every theme; row content is 55px with a 5px gap before separate straight dividers and rounded 8px highlights. Paper now retains the same vertical sidebar line through the filter band. The approved 44px band, 24px pills, 10px filter padding, 8px first-row gap and 240px sidebar remain. Both editors use 1px above/3px below paragraph spacing. Controls have 8px corners and grouped segments have connected interiors. Note corners use 18px while retaining Paper's tear and Playful's seeded waves. Font families, numeric sizes, colors and shadows retain prior choices.

All Notes now hosts Appearance and its Close control in the filter row. The settings builder retains the scrolling control body. GTK verifies the 260px heading aligns with the drawer at x460 in a 720px window and x420 at 680px. Theme-specific filter padding is checked so Paper/Light do not shift the heading.

Minimal previously skipped pattern drawing and hid its selectors. All themes now apply the four existing Plain, Dots, Grid and Ruled choices. Light-theme patterns use stronger dark ink; Dark uses light ink. Global Settings is labeled Canvas pattern. Both per-note selectors remain available with a Default pattern tooltip. Explicit overrides retain priority and persistence, and no real user overrides or notes were changed.

The [actual GTK gallery](design-previews/2026-10-01-theme-review/consistency/preview.html) links separately saved captures per theme for main/default/minimum notes, open tools, long titles/dates, hover, Appearance default/minimum and each pattern in both views. Formatting, 22 tests, Clippy, release build, diff and the existing GTK shared-editor regression pass. Disposable assertions verify heading/filter/row/paragraph geometry, all exact dimensions, tools preserving note sizes, matching content, four distinct persisted pattern renders per canvas, reopened note override persistence and unchanged Grid drawing after global Plain changes. Final GTK logs have no warnings/errors. Native pointer/Escape/timer checks were not repeated.

These requested corrections are under review across all themes. Playful and Light retain their prior approvals before this correction; Dark remains unapproved. No commit or push.


## Unified Appearance drawer correction

The user requested removing the circled back arrow, making Appearance and its lower controls one animated panel, and dimming filters with the notes. The heading and controls now share one OverlaySplitView sidebar. The filters and notes share its content box, so the existing native animation/scrim applies to both complete areas. The manual header visibility callback and separate back button are removed. Settings continues to toggle the panel. The heading stays aligned with the filters; theme ink keeps it readable. A deferred allocation after opening keeps the native dimming layer sized when reduced motion is enabled, including after returning from the wide layout.

The [four-theme GTK gallery and recorded opening/closing animation](design-previews/2026-10-01-theme-review/unified-appearance/preview.html) include default/minimum/wide layouts, scrolling and individual-note/tool regression captures in separate theme directories. GTK checks common parentage, removed arrow, heading/filter geometry, modal hit testing that blocks filters while open and restores access after closing, Settings toggling, scroll attachment and exact dimensions. Animation frames verify shared header/body movement through intermediate positions and stationary filters. Pixel comparisons confirm matching filter/note dimming in all themes with reduced motion enabled; native scrim bounds are checked after wide-to-compact transitions. Reduce motion is honored. Existing wide responsive behavior remains.

Formatting, 22 tests, strict Clippy, release build, diff checks and the GTK shared-editor regression pass. Final render logs have no GTK warnings/errors. Font/palette/pattern choices and individual-note behavior remain unchanged. Native pointer/Escape/timer checks were not repeated. No desktop configuration changed. This requested correction is under visual review; Dark remains unapproved. No commit or push.


## Visual approval and Git checkpoint, October 1, 2026

The user confirmed that visual review was already complete and requested commit and push before moving forward. This supersedes earlier pending-review and no-commit entries for this work.

| Theme | Current approval status |
|---|---|
| Playful | Approved, including subsequent corrections |
| Minimal Light | Approved, including subsequent corrections |
| Minimal Dark | Approved, including subsequent corrections |

The shared filter spacing, corner/paragraph consistency, canvas patterns and unified Appearance drawer corrections are also approved. No theme in this cycle remains under review or not started. The checkpoint includes implementation, review documentation and separately saved GTK previews. Formatting, 22 tests, strict Clippy, release build, diff checks and the GTK shared-editor regression passed at the reviewed code state. Native pointer/Escape/timer and broad phase 5A/5B checks remain outstanding. No later milestone work is authorized by this checkpoint request.

## Edge-strip review, October 1, 2026

The authorized edge pass uses four active notes in the same live Recent/Title ordering as All Notes, including pinned priority. Reopening All Notes retains that sort within the current app session. Older layout/count fields still load, while runtime uses four vertical tabs. The coordinator receives successful blank-note creation immediately. Storage shares the existing ordering clauses; no database migration or custom ordering was added.

Resting dots are 8px with 6px gaps. Tabs are 34x80px, with 4px gaps and rotated Inter 11px/600 titles. The slightly longer tabs keep Groceries readable. New note, Search and All Notes use 34x28px rectangles and the existing icon packs. Left/Right arrange the tabs vertically; Top/Bottom arrange those same vertical tabs across the edge. Long titles truncate with full-title tooltips.

Search stays in the strip at 288x300px, queries active titles/bodies including overflow, supports Up/Down and Enter, and creates a note titled from a nonempty query when no result exists. Escape restores tabs. Search opens around the initiating utility button along the edge so resizing does not move the pointer outside the panel and discard focus. Losing focus to another window restores tabs. Focus still follows compositor policy. Opening a result reuses the existing shared session and separate-window reuse rules.

Hover has a 140ms dwell and 350ms leave grace. Normal tab hover shifts 4px over 140ms, and stack transitions take 180ms. Reduced motion and system-disabled animation suppress both. Marker dragging is preserved; rejected drags cannot reset its position after a tab/utility click.

The edge surface fits resting/tabs/search content. Docked Right/Bottom retain the existing transparent extent needed for stable resizing, with only the note in the input region. Layer placement ignores panel exclusive zones without reserving workspace space, matching the screen geometry used for clamping and preventing end positions from extending beyond the display. The decorative pointer has no hit target and uses each theme's sheet fill and border. Approved editors and text sizes are unchanged.

The [review record](design-previews/2026-10-01-edge-strip/README.md) contains verification results and remaining differences. Edge appearance is under review. Settings and checklist redesign have not started. No commit or push for this pass.

Edge verification completed: formatting, strict Clippy, 23 non-display tests, the existing shared-editor GTK regression, release build and diff checks passed. The complete corrected native probe passed four-edge interactions, marker dragging, real search typing/Enter, click-away, sorting/reopen and motion checks. GTK captures passed every theme/anchor at default and minimum note sizes and both end positions. DND/layer policy passed isolated checks. Actual fullscreen coverage, installed login, timer independence and final performance remain wider milestone checks. Visual edge approval is pending.


## Edge transition and orientation correction, October 3, 2026

The user's recording demonstrates malformed intermediate states despite earlier functional checks. The authorized correction removes the differently sized Stack crossfade and per-tab hover translation, keeps Right/Bottom transparent extent constant across states, limits input to visible content and avoids clearing retained anchors. An isolated transition-disabled check established that removing crossfade alone was insufficient. Fresh native recordings reproduce the before failures and show the corrected opening, closing and Search rendering. [Correction evidence and launch commands](design-previews/2026-10-01-edge-strip/correction-2026-10-03/README.md) accompany the updated gallery; all earlier artifacts remain.

Top/Bottom now have measured, capped horizontal tabs with upright labels, aligned utilities and truncation/tooltips. Left/Right orientation and all approved note appearances remain. Group spacing is 12px; dwell/leave grace remain 140/350ms. Utility hover retains an opaque theme fill, with fresh native checks of all three utilities in every theme. All four edges pass held resting-marker dragging, native search/Enter/Escape, resizing and repeated cycles. Transparent extent passes click-through to disposable All Notes. Four-theme GTK captures check default/minimum notes, Search and both ends; horizontal long-title, fewer-note and empty states pass. The review launcher verifies the current release executable and isolates notes/config and D-Bus on every edge.

Formatting, strict locked Clippy, 23 ordinary tests, the focused GTK geometry regression, existing shared-editor GTK regression, release build and diff checks pass. No Settings/reposition selector, checklist redesign, persistence change, desktop configuration change, commit or push. Reposition discoverability remains unresolved because dragging starts only on resting dots before hover reveals tabs. Original recording review uses extracted frames; direct browser playback of that local file is blocked by browser policy. Saved disposable before/final-after recordings were inspected at normal speed in the gallery and through intermediate frames. Visual/hands-on approval and wider milestone checks remain pending. Stop for this review.


## Lightweight persistent edge strip, October 7, 2026

The user's explicit implementation request supersedes hiding tabs while content is open and removes all edge animation. The [review record](design-previews/2026-10-07-lightweight-edge/README.md) explains the stable geometry, disjoint input regions, focus handoff, save guards and verification. The [gallery](design-previews/2026-10-07-lightweight-edge/preview.html) keeps all four themes and edges, normal-speed native recordings and approved-note regression views. Earlier captures remain untouched.

Marker dragging remains available only from resting dots and has the same discoverability limitation. Settings, checklist redesign, custom ordering, sync, multi-monitor, desktop configuration and later milestone work remain outside this pass. Stop for visual and hands-on review; do not treat implementation or test results as approval. No commit or push.

## Edge checkpoint authorization, October 7, 2026

The user requested a commit of the accumulated edge work through the lightweight persistent-tab implementation. This supersedes the earlier no-commit instruction for this scope. Include the existing edge ordering, Search, orientation corrections, stable placement, focus/save safeguards and tracked verification documentation. The gallery and recordings remain saved locally under the existing agent-docs ignore rule. No push was requested.

The next proposed implementation pass is Settings. The current gear opens Appearance only: theme, Minimal mode, global font/reset, canvas pattern and reduced motion. Edge controls and Behavior controls from the draft are not exposed there. Resolve the Settings host, true strip enablement and reposition commit/cancel behavior before implementation. Shortcut registration and checklist redesign still require their own decisions. Installed-login, timer/Always open and final performance verification remain open.
