# UI implementation plan

Started September 30, 2026; updated October 1, 2026. The references and initial plan were committed as `65e3758`. Stage 1, individual Pin/text controls, and the remaining note-controls pass are implemented in the working tree. The active milestone remains final UI work and phase 5A/5B verification.

## Sources and scope

Playful, Minimal Light and Minimal Dark are visually approved. The user confirmed the review was already complete and explicitly requested commit and push before moving forward. This also approves the subsequent filter spacing, corner/paragraph consistency, canvas-pattern and unified Appearance corrections shown in the [final GTK gallery](design-previews/2026-10-01-theme-review/unified-appearance/preview.html). Include this implementation, documentation and saved previews in the checkpoint. This does not authorize starting the remaining Settings, edge or checklist stages. Native pointer/Escape/timer verification remains outstanding for the wider milestone. Earlier sections retain the historical approval sequence.


The [September 30 reference set](design-references/2026-09-30/README.md) contains the original UI/UX document, nine images, and two SVG packs. The [October 1 update](design-references/2026-10-01/README.md) preserves the revised spec, Settings image and change report. Direct user decisions take precedence over conflicting reference text or example imagery. The reference update was documentation-only. The subsequent request authorizes completing the note controls. It does not authorize Settings, edge or checklist implementation, commit or push.

Use the current Rust, GTK4/libadwaita, Layer Shell, SQLite, and shared `NoteSession` implementation. Preserve note data, live cross-view editing, autosave, failed-save retention, archive/restore, deletion Undo, and Copy text. Keep existing module responsibilities. No replacement editor, generic event system, new UI framework, or independent note buffers are needed.

Multi-monitor work, sync, custom/manual ordering, and release expansion remain later roadmap work. The Settings reference has arrived, but its draft scope and open behavior choices need settling. Checklist redesign still depends on further user input. Stage 1 and the requested removal of infinity, individual Always open Pin, size/weight rows and Settings-only font family are implemented. The remaining note-controls work is now authorized and implemented. The edge and Settings/checklist stages require their own authorization. The requested checkpoint commit covered the saved references and plan before stage 1 changes.

## Agreed target

| Area | Intended behavior |
|---|---|
| All Notes window | Keep the compact 720x540 default and resizing. Retain the 240px list. The existing responsive drawer remains until the new Settings host and width are settled. |
| Main note actions | Exactly three footer buttons: Pin, Pop out, and More. More contains checklist, text style, tag colors, and other existing note actions. |
| Individual notes | Both docked and separate notes are resizable, with matching minimum sizes and a subtle hover-enhanced corner grip. Keep Close visible and long content inside a wrapping, vertically scrolling editor. |
| Note toolbar | Hidden by default. A subtle opening control becomes clearer on hover; clicking opens a floating panel without changing note dimensions. Provide size, weight, and color controls directly in a compact layout based on `tools.png`. Keep the panel collapsible and accessible by keyboard. The revised spec formally confirms this arrangement. |
| Pin in All Notes | Persistent ordering preference. Pinned active notes remain ahead of unpinned active notes. |
| Pin in an individual note | Implemented beside Checklist as Always open for that view. It does not change persistent note ordering. Infinity is removed from runtime assets; tooltip and accessible label identify the role. |
| Inactivity | Normal individual views close after two minutes without input. Always open cancels the timer; turning it off starts a fresh two-minute period. Keep independent timers for each view. All Notes has no timeout. Close and Escape remain available with Always open enabled. Preserve failed-save protections. |
| Fonts | Keep Virgil, Inter, and JetBrains Mono as global Settings-only note fonts. Both editors follow Settings, including notes with older stored font overrides. App headings, lists, menus, and buttons use the UI font. |
| Text sizing | Final title/body sizes and S/M/L values remain undecided until preview review. The requested rows use provisional body sizes of 18/21/24px. Medium retains the current body size, with titles adjusted by the same 3px steps. These values are not final design approval. |
| Corners | Use rounded corners throughout the interface, including Minimal. Keep Paper's torn bottom and Playful's wavy outline as their edge treatments. |
| Paper | Warm paper, soft shadow, shallow torn bottom, and a tag-colored side strip. The All Notes editor and individual note share the same appearance. |
| Minimal Light/Dark | Fully opaque note fills, rounded corners, a tag-colored top strip, and soft shadows. Dark All Notes follows the light layout with dark colors. |
| Playful | Tag-colored note fill, dark wavy outline, offset shadow, and pink theme accent. The pin badge shows the relevant pin state for its view. |
| Sidebar tag markers | Playful uses dots. Paper and Minimal use vertical bars. Marker color follows the note's tag. Selection styling remains separate from tag color. |
| Color choice | The shared palette now has 16 common colors in a four-column grid. Use the same chosen tag color for sidebar/edge markers, Paper/Minimal accents, and Playful fill. Do not add a separate Playful background-color setting. |
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
| Settings | Image and fuller draft received. Prune/confirm sections, settle host/width, Minimal mode placement, along-edge presets, enablement semantics and Reposition save/cancel behavior. Shortcut defaults/registration remain open. | Yes, keep current Settings available until a specific implementation scope is authorized. |
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
- Show only the three main footer actions. Use a compact floating individual tool panel and a subtle hover-enhanced opening control. Opening the panel preserves the writing area and window size. Fit its width to the inline controls rather than adopting the original document's 40px width blindly.
- Make the editor layout flexible and allow both docked and separate notes to resize. Use matching minimum sizes and a subtle hover-enhanced corner grip. Use the window's normal resize path for separate notes and update the docked editor's requested size for the Layer Shell view. Keep an edge-docked note anchored and within available space during resizing. Preserve safe text margins, wrapping, and vertical scrolling. Avoid reserving a large empty toolbar gutter while the toolbar is closed.
- Keep the current text sizes provisional. Present the same sample note in All Notes and an individual window, with toolbar closed/open, and review actual title/body size, padding, and control proportions.

Acceptance: Paper, Playful, Minimal Light, and Minimal Dark remain readable at the compact window size; note appearance matches between views; the toolbar stays unobtrusive when closed; both docked and separate resized notes neither clip controls nor stretch border shapes badly. Docked resizing remains within available space on each edge. User reviews this preview before final text sizes are committed.

### 2. Note controls, colors, pin roles, and save feedback, implemented

Responsibilities: `editor.rs`, `manager.rs`, `note_session.rs`, `app.rs`, `config.rs`, `storage.rs`, `migrations`, styles, and bundled icons.

- Wire the same note tools into the individual toolbar and All Notes More surface. Keep Copy text, bullet/numbered lists, Archive/Restore, and Delete available where applicable through the existing coordinator/actions. Use existing concrete controls and handlers; share small pieces only where both views need them.
- Keep ordering pin state in `NoteSession`. Use the existing per-view Always open toggle and timer for the individual pin icon, so it cannot reorder the note. Preserve explicit close behavior and independent timers.
- Expand the shared color choices and preserve the meanings of existing saved colors. The existing SQLite `notes.color` CHECK constraint allows only four values, so include a small transactional migration for the additional colors. Preserve note IDs, text, dates, pin/order values, archived/deleted state, and related note styles. Update storage parsing, CSS, drawing, and both pickers together. Use a compact grid with readable selected/focus states. Exact shades are calibrated during preview.
- The separately requested S/M/L and Regular/Bold controls now extend the existing per-note style TOML with defaulted fields. No database migration is needed. Keep the numeric sizes provisional until review. Do not revive the retired global `font_size` setting.
- Keep the global note font and per-note appearance live across all views and after restart.
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
- Retain existing reposition behavior until the received Settings draft's customize mode is settled and authorized. The image provides a Reposition entry button, but its visual drag state and commit/cancel behavior remain open. Do not silently add a long-press gesture.

Acceptance: creating a blank note updates the edge immediately; the four active notes match All Notes order; typing in edge search does not corrupt ordering; click/keyboard actions work on every edge; hover cannot replace an open docked note; reduced motion and fullscreen/DND behavior still work.

### 4. Settings reference received; checklist specification pending

Checklist responsibilities: `note_session.rs`, `editor.rs`, `manager.rs`, and note styles/drawing. Preserve the shared buffer, plain-text markers, Copy text, and persistence unless the approved checklist behavior explicitly changes them. Use the existing `TextChildAnchor`/`CheckButton` work as the starting point, and compare its result with the user's design. Implement hand-drawn completion marks only after their appearance and behavior are settled.

Settings responsibilities: `settings.rs`, `config.rs`, `manager.rs`, `main.rs`, `autostart.rs`, styles and only the icons actually needed. Use the [received Settings reference](design-references/2026-10-01/settings.png) and the scoped planning table below. Preserve compact All Notes, live appearance changes, per-note pattern/shape/size/weight and rejected-write recovery. Font family remains global. The draft section list is not yet a commitment to implement every section.

| Draft section | Controls and intended presentation | Existing foundation and work still needed |
|---|---|---|
| Appearance | Three theme swatches; global Handwritten/Clean/Mono segments; Plain/Dots/Grid/Ruled segments; Reduce motion. UI font, section dividers and outlined selection states match the image. | Existing controls and persistence can be reused. Change heading to Settings and use Close. Keep Minimal System/Light/Dark and the theme-default font reset capability even though the image omits them; settle placement. |
| Edge Strip | Enable switch, Left/Right/Top/Bottom anchor, along-edge position presets and Reposition. | Config stores anchor and normalized `marker_offset`; dormant-marker dragging exists. `edge_hover_enabled` disables hover, not the entire strip. Define true strip enablement and what happens to an open docked note before wiring a switch. Reuse offset persistence for presets; propose Top/Center/Bottom on vertical anchors and Left/Center/Right on horizontal anchors. Settings is the pictured entry point to customize mode; do not add long-press implicitly. Settle visible drag state, commit/cancel and exit behavior. |
| Behavior | Launch at login switch; default tag color for new notes in a compact picker. | Login config and autostart reconciliation exist. Both creation paths currently use Yellow; a shared configurable default must reach All Notes, edge creation and later search-created notes without recoloring existing notes. Reuse the expanded shared palette rather than limiting it to the image's four samples. |
| Shortcuts | Readable quick-search and new-note key badges. | No active global bindings are currently registered in the code. The image's Super combinations are examples. Display only actual registered bindings or a clear unassigned state. Default combinations, global registration and optional remapping need separate decisions after core UI stabilization; do not edit desktop configuration as part of this reference update. |
| About | App icon, actual build version and View on GitHub. | Reuse project metadata and application icon. Verify the repository URL during implementation and add only missing GitHub/Reposition icons. Example version text is not a new release request. |

Use one vertically scrollable Settings surface with section headers, not tabs. Scale spacing to the chosen compact host instead of copying the portrait image's pixel dimensions. Final placement as an overlay drawer or separate window, and its width, remain open. No new monitor picker, sync section, always-on-top control or per-note font-family setting is inferred from this image.

Acceptance for an eventual approved Settings pass: controls fit the compact host, are keyboard accessible and recover their saved state after rejected writes; global font changes reach both editors; Minimal modes remain available; strip enablement and repositioning preserve unsaved notes; anchor/presets work on all four edges and survive restart; default color affects only new notes from every creation entry point; launch at login still reconciles correctly; shortcut badges accurately reflect availability; About uses real metadata. These are planned checks, not passed tests.

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

## Stage 1 preview, October 1, 2026

The first implementation pass is ready for review. The references and plan were committed as `65e3758`; the stage 1 code and verification records remain uncommitted. The [preview gallery](design-previews/2026-10-01-stage1/preview.html) contains real GTK renders of the same disposable note in all four appearances, including default, minimum, and larger individual-note sizes. [Verification details](design-previews/2026-10-01-stage1/README.md) distinguish this pass from the remaining controls work.

Implemented matching note drawing between views, Paper side accents, Minimal rounded corners and top accents, shared Playful fills/outlines/shadows, rounded controls, the agreed sidebar marker shapes, and horizontal More dots. All Notes keeps its compact default and three footer actions. The individual tool panel opens on click, collapses, and keeps its existing font/color/list controls available without reserving a gutter. It uses a GTK popover because mapped-window size requests do not reliably grow and shrink the window after compositor resizing.

Both note types resize with a 300x280 minimum. Separate notes retain floating initialization and enable native resizing after their surface enters a monitor. Docked notes keep their position along the edge, clamp to available space, and place the resize grip away from the anchor. Right/Bottom layer surfaces keep a fixed origin on their resizing axis; only the actual note is in the input region. Pointer tests confirmed an 80x40 drag grows the note by 80x40 on every edge. Escape closes both types with Always open and the tool panel enabled.

The existing 21px body and 23px Paper/Playful or 20px Minimal title sizes remain provisional. S/M/L, Regular/Bold, expanded colors, the final pin roles, and save-feedback presentation belong to stage 2 after preview review. The current checklist and settings remain available pending their supplied designs. Stage 1 does not authorize implementing those later stages.

## Requested individual tools revision, October 1, 2026

The user's follow-up authorizes the individual panel's Pin and text controls ahead of the rest of stage 2. Infinity is removed from the panel and runtime icon resources. Pin sits beside Checklist and reuses that view's Always open timer. The ordering control is now only in All Notes. Close and Escape remain available with Pin enabled.

S/M/L and Regular/Bold replace the per-note font dropdown. Changes persist in `note_styles.settings`, update every live editor, and leave the compact tool panel hidden until opened. Regular and Bold set note text to weights 400 and 700. S/M/L body values are provisional 18/21/24px; Paper/Playful titles use 20/23/26px and Minimal titles use 17/20/23px. Font family comes only from global Settings; legacy font overrides still decode but are ignored and are not written back. Paper and shape overrides remain intact.

The [tools preview](design-previews/2026-10-01-tools/README.md) records the renders and verification. The focused-panel Escape path now uses the note's normal Close action, including its failed-save guard. Pointer/keyboard checks passed on all four docked edges and from the nested More menu. The remaining color expansion, All Notes More controls, save-feedback changes, checklist/settings designs, and edge strip stay in their planned passes.

## Updated reference review and proposed next steps, October 1, 2026

The revised spec changes only the individual-tools clarification in section 3.1 and Settings draft in section 3.3. Its floating-panel, infinity/Pin and global-font corrections are already implemented. The received Settings image adds visual guidance and a fuller control list; it does not authorize implementation. The source still contains older conflicting rail dimensions, Pin behavior, fonts, All Notes size and checklist status. See the [change report](design-references/2026-10-01/README.md) for the carry-forward decisions.

1. Review the saved compact UI and size previews; settle final numeric sizes when ready. Confirm/prune Settings sections and resolve the host, Minimal modes, position and Reposition details. Obtain the pending checklist item specification.
2. Completed after the user authorized note controls: All Notes More parity, broader shared colors and their migration, and main-only routine save status with error toasts. Individual Pin and size/weight were reused.
3. Continue with the four-note edge strip, shared ordering and integrated search. Coordinate its enablement and positioning with the settled Settings choices rather than building two unrelated mechanisms.
4. Implement the settled Settings surface and checklist revision in separately authorized passes. Actual global shortcuts can follow core UI stabilization.
5. Finish the existing visual, save-failure, desktop and performance checks before closing the milestone.

The reference review stopped at documentation. The later instruction, "complete the note controls", authorizes the controls pass described below.


## Completed note controls, October 1, 2026

All Notes More and the individual panel share `note_controls.rs` for S/M/L, Regular/Bold and the four-column palette. Main More also exposes Checklist, bullet/numbered lists and the existing pattern override; Copy, Archive/Restore and Delete fit in a compact action row. The main footer remains exactly Pin / Pop out / More. Archived content and editing controls remain read-only until Restore. Font family remains a global Settings choice.

The 16 colors are Amber, Orange, Red, Pink, Sage, Lime, Teal, Cyan, Blue, Indigo, Lilac, Magenta, Brown, Gray, Black and White. The original four storage names and shades remain unchanged. Paper/Minimal accents and markers use the tag accent; Playful uses its corresponding soft fill, including readable neutral fills. `003_note_colors.sql` updates schema 3 in a transaction and preserves every existing note column and style entry. Migration preservation, rollback, archive/delete Undo and color reopen tests cover the persistence changes.

Individual routine save status and inline errors are removed. Each view reports a new save failure with a plain-text toast, retains the session's error/dirty state and suppresses repeated notifications for the same failure. Successful retry clears stale error toasts. All Notes keeps its save indicator and reduced-motion behavior. Main ordering Pin failures use the same shared error reporting rather than a duplicate action toast. Close/switch safeguards are unchanged.

See the [controls verification and preview](design-previews/2026-10-01-note-controls/README.md). Text sizes remain provisional, Settings/checklist decisions are pending and the edge redesign has not started. This pass is uncommitted.

### Reference polish follow-up

Implemented presentation polish on the existing controls and note drawings. Collection filters now span All Notes as in the reference, while the 720x500 default and 240px sidebar remain. Search, filters and tools use consistent rounded corners. No new controls or persistence changes were added. Text sizing and the separate Settings, checklist and edge-strip passes remain open. Preview review is still pending.

### Focused Paper review

Paper visual pass implemented at 720x500, with provisional 17px body and 30px title at Medium. Review the saved Paper comparison before applying these proportions to other themes. Checklist interaction behavior and Settings/edge work remain separate.

Paper follow-up: approved 720x540 default, sidebar updated dates, slightly warmer window chrome and an irregular seeded tear. Minimum window and individual-note sizes remain unchanged.

Paper spacing follow-up approved: keep the divider beneath the sidebar filters only, removing its continuation above the note preview, flatten filter pills to 24px, use 11px sidebar snippets and provisional 17px Medium note body text. Heading sizes remain unchanged.

### One-theme approval cycles

The current Playful implementation uses provisional 27/30/33px titles and 14/17/20px bodies, compact native checkboxes, pink theme selections, round sidebar dots, a smooth seeded dark outline and a stronger offset shadow. Shared drawing/font/line-spacing code contains separate theme choices; Paper and Minimal retain their previous values. No shared design-system refactor was introduced.

Use the [Playful review record](design-previews/2026-10-01-theme-review/playful/README.md) for actual GTK dimensions, matching sample content, controls/font/persistence checks, regression renders and remaining differences. Existing tests, GTK shared-editor regression, formatting, strict Clippy, release build and diff checks pass. Native pointer/Escape/timer and full milestone performance checks remain pending.

Playful is under review, with no final visual approval. Minimal Light and Minimal Dark are not started and not authorized. Stop after presenting Playful. Corrections remain in the same theme. Only explicit approval plus authorization for the next theme advances the cycle. Earlier all-theme plans do not override this rule. No commit or push is authorized.

Playful sidebar correction: the user authorized the missing filter-band sidebar line and clearer highlight/divider separation. Implemented in Playful-only CSS, with straight separators outside rounded selected/hover fills and a 5px gap. See the [fresh review gallery](design-previews/2026-10-01-theme-review/playful/sidebar-polish/preview.html). Typography, controls and other themes remain unchanged. Verification passed; native pointer/Escape/timer checks were not repeated. Playful remains under review and neither Minimal theme is authorized. No commit or push.

### Minimal Light approval cycle

Playful is approved after its sidebar correction. The user authorized Minimal Light next, and that visual pass is implemented. Minimal Light has independently chosen provisional 25/28/31px titles and 14/17/20px bodies, a retained Inter default, compact native checkbox chrome, opaque white rounded notes, existing tag-colored top strips, a restrained shadow and blue tool selections. The compact sidebar keeps tag bars, saved dates, truncated long titles and separate straight dividers outside rounded highlights.

See the [Minimal Light GTK review](design-previews/2026-10-01-theme-review/minimal-light/preview.html) and [verification record](design-previews/2026-10-01-theme-review/minimal-light/README.md). Shared typography/drawing/line-spacing changes have explicit Light-only branches; CSS excludes Minimal Dark. Paper, approved Playful and Dark were inspected for regressions. Formatting, 22 tests, strict Clippy, release build, the existing GTK regression, diff and disposable controls/font/dimension checks pass. Native pointer/Escape/timer checks remain unverified in this pass.

Current status supersedes the historical approval notes above: Playful approved, Minimal Light under review, Minimal Dark not started and not authorized. Stop after presenting Light. No commit or push is authorized.


### Minimal Dark approval cycle

The user authorized Dark after reviewing Light. Dark now uses independently reviewed provisional 25/28/31px title and 14/17/20px body values, Inter by default, compact checkboxes and the same Minimal layout proportions. Its charcoal note, light border, restrained shadow, off-white ink, selected/hover fills and blue focus rings are scoped separately. Sidebar dividers stay outside rounded highlights; tag bars, dates and long-title truncation remain intact. Existing controls and behavior are reused.

The [Dark GTK gallery](design-previews/2026-10-01-theme-review/minimal-dark/preview.html) includes original-reference comparisons, exact default/minimum dimensions, open tools, long sidebar titles and GTK hover/selection/focus/disabled examples. Formatting, 22 tests, Clippy, release build, GTK shared-editor regression, diff and disposable controls/font/persistence/dimension checks pass. Native pointer/Escape/timer checks were not repeated. Matched Paper/Playful/Light regression captures show preserved appearances, with only caret-frame differences for Paper/Playful. No shared design-system refactor was added.

Current status supersedes historical entries: Playful approved, Minimal Light approved, Minimal Dark under review. Stop for Dark review. No commit or push is authorized.


### Filter-band spacing correction across themes

The user authorized fixing the short filter band and first-row gap in one theme first, then copying the verified behavior to all other themes. Minimal Dark was rendered/measured before copying. All four themes now have 10px above/below padding around their retained 24px pills, creating a 44px band, and an 8px divider-to-first-row gap. The sidebar stays 240px. Only eight theme-scoped CSS spacing declarations changed. Theme-specific colors, typography, icons, borders, selected fills, row sizes and behavior remain intact.

The [four-theme GTK reference comparison](design-previews/2026-10-01-theme-review/filter-spacing/preview.html) links actual captures saved separately per theme. Default/minimum main windows, default/minimum individual notes, tools, hover and long title/date previews pass dimension/content/spacing assertions. Formatting, 22 tests, Clippy, release build, diff and GTK shared-editor regression pass. Native pointer/Escape/timer checks were not repeated. Default individual notes and tools remain pixel-identical; minimum-note variations are caret/scrollbar frames.

This spacing-only request supersedes the earlier restriction for these two values. Playful and Light retain their prior appearance approvals; the requested correction and Dark remain under visual review. No commit or push.


### Requested consistency and canvas-pattern correction

The user authorized cross-theme corner/spacing fixes, Appearance on the filter row and dot/plain/square canvas choices working. Headers, rows, dividers, control corners and paragraph spacing now match across themes while retaining colors, font sizes, shadows and Paper/Playful silhouettes. Appearance's header and Close are hosted in the All Notes filter band, aligned with the existing 260px drawer at both window sizes. The settings builder retains its scrolling controls.

Existing Plain/Dots/Grid/Ruled controls now draw in every theme, including Minimal, with visible dark/light ink. Controls are labeled Canvas pattern globally and remain available per note. Default pattern follows Appearance, while explicit overrides remain intact and persist. No note-session or storage change was needed. This is a targeted correction, not the pending Settings or checklist redesign.

The [new GTK comparison](design-previews/2026-10-01-theme-review/consistency/preview.html) includes before/after, Appearance at default/minimum, pattern renders in both views, long sidebar titles/dates, individual default/minimum and open tools in separate theme directories. Formatting, 22 tests, strict Clippy, release build, diff and GTK shared-editor regression pass. GTK asserts geometry, dimensions, heading alignment, pairwise-distinct canvas drawings for all four persisted global choices, reopened note overrides and stable override rendering after global changes. Native pointer/Escape/timer checks were not repeated.

Current status: requested corrections under visual review across all themes; Minimal Dark remains unapproved. Earlier Playful/Light approvals precede these corrections. No commit or push.


### Unified Appearance animation and filter dimming

The user authorized removing the Appearance back arrow and making its header/body appear as one panel, with filters dimming alongside notes. The header/body now share the existing native sidebar; filters/notes share its content area. This replaces the manual header visibility callback with native drawer behavior. A deferred allocation on opening keeps native dimming intact with reduced motion after wide-to-compact layout changes. Settings remains the toggle. Typography, palette, patterns and existing responsive wide behavior remain intact.

The [fresh GTK gallery and native animation recording](design-previews/2026-10-01-theme-review/unified-appearance/preview.html) verify the four themes at default/minimum/wide sizes, scrolling, background filter modal hit testing, Settings toggling, individual-note sizes and tools. Opening/closing samples prove header/body move together while the filters stay stationary. Pixel comparisons confirm filter/note dimming in all themes with reduced motion; GTK also checks the native scrim size after responsive layout changes. Reduce motion is respected. Formatting, 22 tests, Clippy, release build, diff and GTK shared-editor regression pass. Native pointer/Escape/timer checks were not repeated.

The correction remains under visual review; no theme approval is inferred. No commit or push.


### Checkpoint approval

On October 1, the user confirmed visual approval of the completed themes and corrections and explicitly requested commit and push before further work. That instruction supersedes pending-review and no-commit statements above for this checkpoint. No new implementation stage is started.
