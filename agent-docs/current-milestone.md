# Current milestone

[SPEC.md](SPEC.md) is the current product truth derived from the code. [HISTORY.md](HISTORY.md) preserves the former milestone log unchanged. The active milestone remains final UI work and phase 5A/5B verification; [implementation.md](implementation.md) records later roadmap work.

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
