# Current milestone

[SPEC.md](SPEC.md) is the current product truth derived from the code. [HISTORY.md](HISTORY.md) preserves the former milestone log unchanged. The active milestone remains final UI work and phase 5A/5B verification; [implementation.md](implementation.md) records later roadmap work.

## October 15 final menu and timeout corrections

The owner requested a local commit of the accumulated stage. All Notes More now has one icon row for Copy, Archive/Restore and Delete beneath the canvas samples, retaining tooltips, accessible labels and the existing handlers. Sort reuses the note popover chrome and row hover styling with left-aligned labels. A successful docked inactivity close now collapses the strip to its marker; failed saves retain the note and tabs. Manual Close continues to reveal tabs.

Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 GTK deprecation warnings. No tests or app sessions were run; runtime and visual verification remain with the owner. This stage is authorized for a local commit. The owner's `implementation.md` edit remains excluded; no push is authorized.

## October 15 native editing menu and adjacent emoji picker

The owner authorized wiring the existing right-click Undo/Redo entries and shrinking/moving the emoji picker. Body and title native PopoverMenus now override only their history actions with the shared NoteSession handlers; native Cut/Copy/Paste/Delete/Select All/Emoji entries remain. Menu availability reads the real history and editability at open and on session changes. Keyboard shortcuts and formatting storage are unchanged. The native EmojiChooser has compact UI fonts, eight-column grids, smaller category buttons and a 180 px results cap. Its pointing rectangle moves from the caret to the sheet boundary: inward for each docked edge, left by preference for pop-outs and All Notes, with GTK popup constraints retaining screen-fit behavior. It closes the context menu and footer popovers before use.

The native history regression includes activating the existing body Undo and title Redo menu actions. Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 GTK deprecation warnings. No tests or app sessions were run; tests were compiled only. Visual and runtime verification remain with the owner. No commit or push is authorized. The owner's `implementation.md` edit remains untouched.

## October 15 checklist caret and list-edit crash

The owner's manual run aborted with SIGABRT at 02:19:07 IST, PID 1683626. The recorded stack enters `note_text::decorate` through `TextView::iter_at_location`, then GTK's visible-line-index conversion reports byte index 29 beyond the line. The renderer now starts at `line_at_y` rather than hit-testing coordinates and skips unmarked text, anchors and hidden markers before requesting decoration geometry. The checkbox callback preserves the caret/selection across its internal marker replacement; native edits and checklist decoration move a collapsed caret out of the hidden prefix. Checkbox clicks retain text focus. Stored marker text and inline-format encoding remain unchanged.

The existing native typing/history regression now uses the actual decorated view and includes typing before an anchor and typing after checkbox activation. These cases were compiled only. Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed; the build retains 24 GTK deprecation warnings. No tests or app sessions were authorized or run; manual confirmation of the crash fix and caret behavior remains pending. The journal stack was read without extracting a core or altering saved notes. No commit or push is authorized; `implementation.md` remains untouched.

## October 15 Settings spacing and formatting feedback

The owner requested more breathing room in Settings, full checklist strike coverage and selected formatting states. Undo/Redo remain keyboard-only. Settings now spaces rows by 16 px, with full-width Edge, Position and Auto-close choices below their labels. The existing B/I/U/S buttons show a selected fill for a fully marked visible selection and refresh on range/tag changes without moving the capsule. Mixed selections stay inactive; hidden checklist prefixes and anchors are excluded when determining and toggling the selected mark. Checklist decorations use visible cursor geometry after hidden prefixes, avoiding the raw-offset displacement. Titles remain plain text; adding saved title formatting was discussed but is not approved.

Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 GTK deprecation warnings. No tests or app sessions were authorized or run. Manual review remains with the owner. No commit or push is authorized; the owner's `implementation.md` edit remains untouched.

## October 15 contextual footer capsule and menu simplification

The owner locked the design and explicitly authorized implementation without tests or native app sessions. The agreed scope is one fixed, opaque footer capsule in both editors; its normal controls switch to B/I/U/S while editable body text is selected, with no separate popup, positional tracking, animation or extra Back control. A homogeneous Stack reserves the larger page size. Selection and editable-state notifications follow buffer replacement, and formatting uses the existing handlers without clearing selection. Escape clears the selection before closing an individual note. Archived notes remain read-only.

Separate notes now have a small native drag handle in their existing top margin. More uses canvas samples without a heading, bullet/numbered list icons in individual notes, compact icon-and-label actions and a separated Delete row. Aa shows the same sample in each bundled font with full-name tooltips, plus S/M/L. Whole-note weight controls and handlers are removed; new styles omit the weight key, while existing stored bold stays readable and survives later style edits. Both editors share note-menu chrome. All Notes keeps per-note styling and Archive/Restore. Individual deletion still uses the edge Undo host; All Notes keeps its own toast.

Settings starts with theme previews and compact labeled rows, without Appearance/Edge strip/Behavior headings. Its existing choices, Reposition and bottom switches remain. Classic tabs use one button extending off screen, removing the shadow extension seam while preserving the 6 px inward pull and attachment on every edge. Stacked motion is unchanged. The 300 × 280 minimum remains.

Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 existing GTK deprecation warnings. Existing native expectations were updated for the contextual controls, compact Settings rows and preserved legacy weight; they were compiled but not executed. No tests or app sessions were run. Visual approval is pending. No commit or push is authorized for this pass. The owner's `implementation.md` edit remains untouched.

## October 15 selected-note styling in All Notes

The owner approved restoring Color, Font and Canvas pattern to the selected-note editor in All Notes. It now reuses the individual note's eight-column color palette and Aa font family/size/weight popover, with the same flat pattern choices in More. Handlers use the selected note's shared session and persistence, so another open view receives the same style. Archived notes keep styling disabled. Global Settings remains unchanged.

Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 existing GTK deprecation warnings. No tests or app sessions were run. Existing menu-capture and archived-control assertions were updated for the restored controls and compiled only. Changes remain uncommitted for owner review, and `implementation.md` is untouched.

## October 15 per-note styling and compact Settings

The owner requested implementation without tests or app sessions. Color, font family/size/weight and canvas controls now exist only in docked and separate notes. All Notes keeps Checklist, Pin, Pop out and More. Archive remains only in All Notes. Individual Color uses eight columns and two rows; Aa includes the four bundled font families. More opens toward the screen interior on the footer row. Nonmodal popovers allow a direct first click on another footer option, and options use a subtle gray hover fill.

Selection-bubble updates coalesce cursor/selection changes and retain unchanged placement. The shared native TextView paints font-sized underline/strike strokes while preserving tags, stored spans and history. Legacy global font/canvas values migrate into saved note styles before their config keys are retired; the database schema is unchanged.

Settings retains theme/mode, edge/position, a visible Reposition button, four/five notes, Classic/Stacked, Pills/Bar and docked timeout. Non-theme choice rows are smaller, and Reduce motion, Hover to reveal and Launch at login switches sit at the bottom. The old dots value now draws pills. Classic tabs only pull inward 6 px; Stacked keeps the accepted magnification/spread. Both use the existing interruptible follower, screen attachment and reduced-motion handling.

Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 existing GTK deprecation warnings. No tests or native app sessions were run. Existing Settings, config/style and editor control assertions were updated for the removed controls and compiled only. Manual verification is pending. Changes remain uncommitted; the owner's `implementation.md` edit is untouched.

## October 15 compact Search, restored strip choices and menus

The owner requested a local checkpoint before this pass. Commit `1157b27` includes the preceding stacked-hover, transition and chrome work, excluding the owner's `implementation.md` edit. No push was requested or made.

Quick Search now requests 224 px width, with content-driven height and a 128 px cap on results. Rows are compact and the scrollbar is hidden; wheel scrolling and keyboard selection retain access to every result. The no-match creation label is shortened. Docked placement now moves the sheet only when the new tab lies outside its arrow range, preventing an arrow clamped to the sheet from indicating a different note.

The owner explicitly restored both Classic/Stacked and Dots/Bar in Settings, overriding their earlier removal. Both keys serialize again, with Stacked/Bar defaults; old adjacent/overlapping values load. Classic uses a 4 px gap and Stacked an 8 px overlap, sharing the accepted hover wave and edge attachment. Dot markers follow saved visible notes, with a neutral empty marker. Vertical titles shorten when needed to fit five Classic tabs on a small screen. Reposition now belongs to Position. Canvas choices have visible labels and larger samples drawn with stronger theme ink. All Notes defaults to 720 × 500 with a 680 × 480 minimum.

Approved footer-menu polish uses left-aligned 32 px action rows, tighter inline pattern choices, 8 px popup corners, smaller arrows and lighter shadows. Menu contents and single-level behavior are unchanged.

Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 existing GTK deprecation warnings. Tests and app sessions were not run. Existing config migration/round-trip assertions now retain restored choices; native Settings checks no longer expect Tabs/Marker to be absent, and window-size cases use the shorter sizes. These were compiled only. Current changes remain uncommitted for manual review; `implementation.md` remains untouched.

## October 15 chrome and visible-note count

The owner requested removal of decorative interaction outlines/highlights, a more visible dormant bar, a fresh Settings audit and bottom-menu suggestions only. Tab selection accents and their class-management code are removed. App buttons no longer gain decorative hover/pressed fills or focus rings; color/theme checks, switch positions, selected value fills, collection filters and selected rows remain readable. Static paper/control boundaries and shadows remain. The accepted hover wave and note transitions are unchanged. The dormant bar grows from 3 × 28 to 4 × 40 px with a subtle shadow.

The owner explicitly approved Visible notes 4/5. Settings now saves the existing key, restores the accepted value on rejection, and applies it to the query and horizontal strip sizing. Rendering uses the saved-note query even while the deck retains an open note. Hover to reveal now sits under Edge strip. Current source still includes screen edge, position and Reposition below Appearance in the scrolling drawer. Historical removals were deliberate: White paper, tab/marker layout choices and both hide-policy controls; shortcut examples moved to README. Shape has stored data but no picker. True strip enablement and default note color have no implemented Settings controls or keys. No other controls were added.

Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 existing GTK deprecation warnings. Tests and app sessions were not run, as requested; visual review remains with the owner. Bottom-menu layout remains unchanged, with recommendations reported separately. The owner's `implementation.md` edit is untouched. No commit or push was requested.

## October 14 title clearance and note transitions

The owner accepted the hover wave and requested title clearance, bar-to-strip and note-switch transitions, and narrower individual notes. The hover response remains unchanged. Drawing now proceeds from the farthest neighbor toward the hovered title; overlap clicks use the same order. Labels reserve 10 px at either end, beyond the 8 px overlap. One shared reveal clock replaces separate tab/marker/utility clocks and their stagger, keeping 160 ms entry and 110 ms exit.

Docked note switches keep the current position and dimensions and crossfade editor children through the existing GtkStack over 120 ms. The existing motion helper controls completion and the new arrow fade. Save/release guards remain, the destination session loads before releasing the current view, outgoing input is disabled, and completion/closure/replacement removes the retired child. Reduced/system motion settles immediately. New docked and separate notes start at 320 × 376 px, with the existing 300 × 280 minimum. No controls or settings were added.

Existing strip regressions now observe the common clock, and the state-transition regression includes a direct docked-note switch. The obsolete stagger-only unit test was removed with the retired stagger code. Tests and native app sessions were not run, as requested. Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 existing deprecation warnings. The owner's `implementation.md` edit remains untouched. This pass remains uncommitted pending manual review.

## October 14 stacked title hover

The owner requested the competitor clip's stacked motion with Dock-style hover, title-only, polished for all four edges. The existing 160/110 ms reveal/collapse and 20 ms stagger remain. Hover now has a continuous proximity wave, peaking at 24% growth with 6% growth in the immediate neighbors when centered on a title. Spacing follows painted sizes and retains the 8 px overlap. Screen-facing pivots keep all tabs attached; 24 px end reserves cover the horizontal and vertical wave. Resting centers prevent hover feedback, and outgoing front order persists until the wave settles.

The existing motion helper now follows changing pointer targets on one frame callback per moving tab, rather than restarting timed animations on every event. It closes 99% of the distance in 120 ms, handles reversal/unmap/cancellation, and settles immediately under reduced or disabled system motion. No previews, controls, settings, dependencies or surfaces were added. Existing edge regressions were updated for continuous tracking and neighbor growth. The owner's `implementation.md` edit remains untouched. Tests and app sessions are explicitly forbidden for this pass and were not run; visual acceptance belongs to the owner. Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 existing deprecation warnings. This pass remains uncommitted.

## October 13 edge titles and restored hover

The preceding peek and draft/default-filter work is committed locally as `b9b45dd`, excluding the owner's `implementation.md` edit. No push was requested.

The owner confirmed that the perfect animation was the clean `b9b45dd` checkout. Its tab-crossing trigger, selected-tab targets, 120 ms handoff, reveal/collapse timing and motion helper are reused. Motion retargets only when the hovered tab changes, rather than restarting on every pointer movement. Neighboring spread and continuous proximity scaling are removed. The existing 24% edge-anchored scale remains in place of that commit's detached 6 px translation, preserving the owner's attached-title requirement. Reduced/system motion settings settle the values immediately; Reposition resets them. No motion timing was retuned.

All peek rendering, private buffers, label transfer, masks, timers, input exclusions and editor-handoff code remain deleted. The normal individual editor renderer has no read-only peek path. Draft visibility, Active defaults, note controls, storage and configuration choices are unchanged. README, SPEC and section 4.4 describe the title-only hover behavior.

Existing hover/stacked-tab regressions cover movement within one tab without retargeting, handoff/exit reversal, edge attachment, enlarged-area clicks, stable input/surface bounds and reduced/system motion plus Reposition. Tests and native app sessions are not authorized for this change and were not run. Formatting, strict all-target Clippy with existing GTK deprecations allowed, the locked offline build and diff checks passed. The build retains 24 existing GTK deprecation warnings. The owner requested a local commit of this stage, with no push. The clean comparison checkout remains at `/home/rinzler/.codex/worktrees/last-commit-review/Stickies` and is untouched.

## October 13 draft visibility and All Notes defaults

Owner-approved blank creation now opens a hidden draft from the strip, All Notes or the new-note action. The existing creation timestamp stores an unpublished marker, so schema 4 stays unchanged. List and strip queries exclude drafts before sorting and limiting. The first successful non-blank text/formatting save publishes the note atomically; failed saves retain hidden drafts and their editors. All Notes keeps its rows, list selection, filter and search while editing a draft, then selects the published row. Search-created titles and existing blank notes remain visible. Last-view blank cleanup retains the shared-view/save guards, and startup removes orphan drafts and their styles.

New All Notes windows and default collection fallbacks use Active with an empty search. Existing windows retain their state. The All / Active / Archived order and scoped search are unchanged. No UI, settings or formatting encoding changes were added. SPEC records both behaviors.

Verification passed: the four requested storage regressions and the updated last-view GTK regression in a disposable Wayland session. The GTK case also checks unchanged draft rows/selection, first-save publication with another note docked and failed-save guards in All Notes. The existing archived-preview test now explicitly chooses Archived after archiving, and the edge Search test now expects a newly created blank to be absent from the deck. Both were compiled but not executed. Formatting and strict Clippy with existing GTK deprecations allowed passed. Broader suites were not run. Work remains uncommitted for owner review.

## October 12 edge peek cards

Owner-approved section 4.4 replaces tab magnification and neighboring spread with one 6 px transform nudge. The hover card uses the individual editor's shared NoteSheet, with private read-only buffers, wrapped title, about five body lines, stored formatting and per-note pattern. Empty bodies show "Empty note". It stays within 260 px and screen bounds, swaps between tabs, fades over 120 ms and has a 150 ms leave grace. Reduced motion removes the fade. No tooltip, setting, second renderer, focus request, NoteSession, save path or note activity timer is added for peeks.

The existing panel surface hosts the card, which is subtracted from both input regions. Clicking keeps the save guards, carries its edge position to the editor and replaces it at full opacity. The current docked note's own tab never reopens a card over that editor; other tabs can preview without changing it. Utilities and the reserved note footer are unchanged. Formatting and strict Clippy with GTK deprecations allowed passed. The existing hover geometry test now expects translation rather than magnification. Test execution was not requested and the suites were not run. The pointer recording, extracted frames and scoped report are in `/tmp/stickies-peek-check/review.md`. Work remains uncommitted and stops for owner review.

## October 12 undo grouping

Owner-approved undo grouping keeps the note's shared custom title/body snapshots and formatting encoding. Typing/deletion groups until a one-second pause, typed newline, direction/field change, cursor movement or the 100-character cap. Paste/cut/drop, formatting and list/checkbox actions are isolated. Cursor restoration uses the actual changed range, including repeated text and partial formatting. Title shortcuts now use the same custom history; Enter in the single-line title also ends a group. The monotonic clock is injectable for deterministic pause tests. No UI or configuration key is added.

Verification passed: 48 non-display tests, all 18 native GTK tests and three affected native rechecks, plus all three release shutdown probes. Formatting, strict Clippy with existing GTK deprecations allowed, the locked offline release build and diff checks passed. The release build retains 24 GTK deprecation warnings. Thirteen pure history tests and one GTK integration test cover the requested grouping cases, cursor placement, title/body actions, clipboard, formatting, list continuation and checkboxes. Pause tests use a fake clock without sleeping. The results and scoped diff are in `/tmp/stickies-undo-check/review.md`. Work stops for owner review; this stage remains uncommitted.

## October 12 footer controls pass

Prompt 2 of [the redo plan](stickies-redo-plan.md) replaces both old tool presentations with the specified reserved footer, separate Color/Text popovers, flat More menus and selection-only B/I/U/S bubble. Individual More reuses list/pattern/pop-out handlers and the shared Archive/Delete save guards. All Notes More has only Copy text, Archive/Restore and Delete. Archived controls are disabled except Copy and Restore. The note minimum remains 300 × 280; its corner-drag input remains unchanged.

Owner clarification restores Ctrl+B/I/U and the Ctrl+Y redo alias. Settings Reposition now has a visible label. Formatting encoding and stored per-note overrides are unchanged.

Owner-approved individual deletion feedback uses the existing strip surface, beside its current position and clamped to the screen. Only the latest "Note deleted" / Undo toast remains, for six seconds. Its bounded input region leaves the rest of that surface click-through. It uses docked visibility policy and the existing 120 ms motion helper, with no fade under reduced motion. Archive has no toast; All Notes keeps its existing toast. No additional layer surface or config key is added.

Verification passed: 35 non-display tests, all 17 native GTK tests in an isolated Sway 125% Wayland output, and all three release shutdown probes. Formatting, strict Clippy with existing GTK deprecations allowed, the locked offline release build and diff checks passed. The release build reports 24 GTK deprecation warnings. Both editors have five-state captures in every theme, with additional minimum-size, edge Undo and labeled Reposition captures in `/tmp/stickies-controls-check/review.md`. No requested control was omitted; the 300 × 280 minimum retained usable body space in every theme. User review is required before further UI work. Earlier controls records below describe superseded stages.

The subsequent owner-approved polish places the selection bubble below when it cannot fit above without covering the title. It restores the subtle corner grip only on note hover, inside the unchanged drag area. Resting footer opacity is raised from 55% to 65% in both editors; hover and focus stay at full opacity. Formatting and the locked offline compile passed. Tests and native app sessions were not authorized for these follow-ups and were not run; the preceding captures and test results describe the footer pass before this polish.

## October 12 removal-only controls pass

Prompt 1 of [the redo plan](stickies-redo-plan.md) removes both body editing bars, Find in note, inline formatting access, the individual nested More menu, the five retired Settings choices and edge-tab tooltips. Stacked tabs and the bar marker are the only edge presentation. Old white-paper/tab-layout/marker-style keys are removed from valid configs on load. Fullscreen/DND keys remain file-only and are documented in README.

Existing formatting storage and per-note styles remain. Individual Bullet list, Numbered list, Pattern override, Copy text and docked Open away from edge are temporarily unreachable until Prompt 2. The first tools popover and All Notes actions stay unchanged. Footer/selection-bubble rebuilding and hover peek cards await separate prompts and approval. No new UI is added.

Verification passed: 35 non-display tests, all 15 native GTK tests in an isolated Sway 125% Wayland output, three affected edge rechecks, and all three release shutdown probes. Formatting, strict Clippy with existing GTK deprecations allowed, the locked offline release build and diff checks passed. The release build retains 28 existing deprecation warnings. The requested four-theme captures and the control/storage/test report are in `/tmp/stickies-removal-check/review.md`. User review is required before Prompt 2.

The daily-use record below describes the preceding pass; this removal pass supersedes its controls and edge-style choices.

## October 12 daily-use feedback

Implemented with owner approval: All Notes tools reserve space beside or below the sheet; no new click-away closing behavior; Kalam Regular/Bold; theme-ink checklist completion and read-only archived checkboxes; optional Stacked tabs and Bar marker with existing defaults preserved; exposed working behavior settings; compositor shortcuts moved to README; shorter hover labels; body undo/redo, Find in note and selection formatting stored separately in SQLite schema 4. All Notes now requests a 680 × 520 px minimum so compact tools remain visible. Its default size is unchanged.

Verification: 34 non-display tests and all 15 native GTK tests passed, with the two settings tests rerun after correcting a rejected-switch resubmission and updating the minimum-size assertion. Strict Clippy with existing deprecations allowed, formatting, diff checks and the locked offline release build passed. The release build retains 28 existing GTK deprecation warnings. Screenshots and logs are under `/tmp/stickies-feedback-check`; the compact/wide All Notes review is in `case-2vdrszpa/images`. User visual approval remains pending.

The note-only Hyprland border rule was added to the owner's user Lua config, with a timestamped backup. Reload and config-error checks passed. Actual login launch, final performance budgets and broader compositor acceptance remain open.

## Open items

- The October 11 edge-strip polish uses 6 px tab gaps, quieter tab/capsule shadows, a single selected-tab accent and contiguous utility cells with centered dividers. Title size and hover behavior are unchanged. The normal offline build passed with existing GTK deprecation warnings. Visual approval and the reported softness remain unresolved. No tests were run at the user's request.
- Complete final manual review of recent edge chrome/motion, positioning, Search/arrow behavior, blank-note cleanup and All Notes opening without a preview. Use [manual-testing.md](manual-testing.md); older reference drafts must be checked against SPEC.
- Confirm docked timeout choices, pointer activity, Always open, persistent pop-outs, popover-first Escape and the running-instance actions in the final desktop UI.
- Settle the remaining Settings scope: true strip enablement, default new-note color and About. Shape and visible-note-count UI also remain unimplemented; `visible_notes` is still fixed to four at runtime.
- The October 10 UI corrections passed 34 non-display tests and all 12 native GTK tests in a disposable Wayland session. Release checks confirmed toolbox placement, Search scrolling/Unicode activation and empty All Notes layouts. User visual approval remains pending. The icon-mask cache leaves a large-content startup/memory failure: 1,284 notes with roughly 20 KB bodies took 54.2 seconds to map All Notes and used 379.4 MiB PSS in the isolated 125% display. A bounded list excerpt is proposed, awaiting approval. See [the verification record](../plans/README.md). Recheck the old menu-retention finding against current weak callbacks before declaring it resolved; see [code-audit.md](code-audit.md).
- Complete native CI dependency coverage (libadwaita, Fontconfig and resource compilation) and verify on a clean runner. Include all bundled-font notices in distribution.
- Verify installed launch after an actual login; generating the autostart entry alone is insufficient.
- Finish fullscreen/DND, focus handoff, window movement/floating and scaling checks on the reference compositor. Portable installation must not depend on the development machine's floating rules. Wider compositor validation remains later roadmap work.
- Investigate the graphics invalid-rectangle warning if it recurs; it has not been declared fixed.
- Remeasure the final UI, including the two screen-sized layer surfaces: at most 200 MiB PSS, ordinary interaction below 2% CPU, normal animation below 3%, exceptional one-second spikes at most 5%, idle returning to 0%. Use interval measurements with labeled activity; earlier baselines are not final acceptance.
