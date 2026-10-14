# Current verification checklist

Run against the release build on the reference compositor. Record failures and pending checks, not a history of every run. Final UI approval belongs to the user.

## October 15 chrome and visible-note count

Owner review is pending. Tests and app sessions were not run. In all four themes and on all four edges, check the stronger dormant bar, unchanged hover wave, and absence of decorative focus/pressed/selected rings. Color checkmarks, switch positions and selected values must remain readable. Scroll Settings below Appearance to Edge strip and switch Visible notes between 4 and 5 with at least five saved active notes, including while a note is docked. Check persistence after restart and that a rejected config write restores the accepted count. Hidden drafts must not consume slots. Bottom-menu layout is unchanged; compact menu suggestions await approval.

## October 14 title clearance and note transitions

Owner review only; no tests or app sessions were run. On all four edges, hover the first, middle and last title and check that neighboring titles have consistent overlap and readable text. Check that the bar, tabs and utilities reveal and reverse together. Click rapidly between notes, including notes with different colors and fonts, then close during a switch. The sheet should crossfade in place with no blank frame; the latest note must remain interactive and the old view must disappear. New notes should be 320 × 376 px, with height unchanged and the 300 × 280 minimum preserved. Check reduced motion, disabled GTK animations and the existing failed-save guard.

## October 14 stacked title hover

Manual review is pending with the owner. No automated tests or app sessions were run for this pass. In Paper, Playful, Minimal Light and Minimal Dark, check Left/Right/Top/Bottom at both ends of their position range. Sweep across all titles, reverse quickly, move within one title and leave halfway through. The nearest title should grow most, nearby titles should respond less, and the stack should keep its overlap and screen-edge attachment. Check enlarged-area and overlapping clicks, reduced motion, disabled GTK animations, and Reposition. There should be no preview or tooltip.

The existing hover and stacked-tab regressions now describe continuous pointer tracking, neighbor growth and the larger end reserve. Their assertions for edge attachment, click routing and fixed input/surface bounds remain. These tests were updated but not executed. Earlier peek captures are obsolete.

## October 12 undo grouping

In each individual/All Notes title and body, type a phrase without pausing and undo it once. Pause for at least one second between phrases and undo them separately. Repeat with Enter, an insertion-to-backspace transition, arrow/mouse selection movement, paste, cut and drop. Keep formatting and list/checkbox commands as separate steps. Undo and redo must restore text/spans together, return to the changed range and preserve Unicode. Typing after undo must clear redo. The fake-clock unit tests exercise the pause rule without sleeping; the native regression also exercises GTK title/body input, clipboard, list and checkbox actions.

Verification passed: 48 non-display tests, all 18 native GTK tests, three affected native rechecks and all three release shutdown probes. Formatting, strict Clippy with existing GTK deprecations allowed, the release build and diff checks passed. [The report](/tmp/stickies-undo-check/review.md) includes the grouping behavior, added tests and scoped diff. User review remains pending.

## October 12 footer controls review

Review the individual note and selected All Notes preview in Paper, Playful, Minimal Light and Minimal Dark, resting and with Color, Text, More and the selection bubble open. Verify the footer does not move the title/body or change note size. At 300 × 280, the individual writing area must remain usable and all five footer controls must fit.

Check the exact menu contents in the redo plan. There must be one popup level, no pattern dropdown, no old header launcher and no inline tools panel. S/M/L, Regular/Bold, color and per-note patterns must still save through the existing handlers. The Checklist footer toggle adds/removes checklist mode; embedded checkboxes toggle completion. Archived previews allow only Copy and Restore.

Verify Ctrl+B/I/U, Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y in both body editors. Selecting another All Notes note must replace the bubble's buffer subscription. Clearing selection, archiving or hiding a view must hide the bubble. Formatting still survives reload.

Reject a save in disposable data, then Archive/Delete from an individual note. The editor must remain. With writes restored, Archive closes it silently. Delete from docked and pop-out notes closes them and shows the edge Undo toast without opening All Notes. A new deletion replaces that toast and resets its six-second lifetime. Undo restores previous collection and pin state. Check screen clamping, Overlay/DND visibility, bounded input and reduced motion. All Notes deletion keeps its own six-second Undo toast.

Settings shows the Reposition label in all four themes. Verification passed: 35 non-display tests, all 17 native GTK tests on isolated Sway at 125%, and all three release shutdown probes. Formatting, strict Clippy with existing GTK deprecations allowed, the locked offline release build and diff checks passed. [The review report](/tmp/stickies-controls-check/review.md) lists updated tests and both editors' five-state screenshots, plus minimum-size, edge Undo and Reposition captures. User visual review remains pending.

## October 12 removal-only review

This pass supersedes the editing-bar and edge-style checks in older records below. Review the individual note, selected All Notes preview and Settings in Paper, Playful, Minimal Light and Minimal Dark. Both writing areas must have no editing/search bar. Individual tools retain their first-level contents and have no More or pattern dropdown. All Notes actions remain unchanged.

Check Ctrl+Z and Ctrl+Shift+Z in both body editors, including after selecting another All Notes note. Ctrl+F/B/I/U must have no note-search/formatting action. Existing formatting and pattern overrides must survive reopening. Check stacked tabs and the bar marker on all edges without tab tooltips. Settings must omit White paper, Tabs, Marker and both hide-policy controls. The README documents the remaining file-only hide policies.

Individual Bullet list, Numbered list, Pattern override, Copy text and docked Open away from edge return in Prompt 2. Do not review the footer or selection bubble as implemented yet.

Verification passed: 35 non-display tests and all 15 native GTK tests, with three affected edge tests rerun after marker CSS cleanup. The isolated display used Sway at 125%. All three release shutdown probes passed. Formatting, strict Clippy with existing GTK deprecations allowed, the release build and diff checks passed. [The review report](/tmp/stickies-removal-check/review.md) includes the test changes and individual/All Notes/Settings screenshots in all four themes, plus lower Settings and wide All Notes views. User visual review remains pending.

## October 12 daily-use feedback

The approved feedback pass passed 34 non-display tests and 15 native GTK tests in an isolated 125% Wayland display. The final two settings checks were rerun after fixing rejected-switch feedback and updating the minimum-size expectation. Formatting, strict Clippy with existing deprecations allowed and the offline release build passed. Logs and screenshots are in `/tmp/stickies-feedback-check`.

The new tests cover Unicode formatting ranges, checklist-prefix edits, undo/redo, atomic failed saves, reopening formatted notes, compact/wide tool placement across themes, stacked-tab click routing and bounds on all edges, and reduced motion. Existing shared-session, timeout, Escape, positioning and blank-note checks also pass.

For owner review:

- Open All Notes tools at 680 × 520 and at wider sizes. Tools must stay outside the sheet; the compact palette and actions must be visible. Date metadata returns when compact tools close.
- Select body text for bold, italic, underline and strikethrough; edit, undo/redo, close and reopen. Check the same note in two views. Find in note advances on Enter and wraps; Copy text remains plain text.
- Compare Virgil, Inter, Mono and Kalam with checked and unchecked items, including archived previews.
- Select Classic/Stacked and Dots/Bar independently, then switch back. Check all screen edges and reduced motion. Existing styles remain defaults.
- Check the concise Settings controls and font/pattern samples. Global keybinding instructions are in README. Launch-at-login still needs an actual login check.
- Open a separate note on the owner's desktop and confirm the Hyprland border rule while retaining the note's own theme drawing.

## October 10 UI verification

After the user authorized tests, 34 non-display tests and all 12 ignored native GTK tests passed. Formatting, strict Clippy with existing deprecations allowed, release compilation and the three shutdown signal probes also passed. Native assertions cover theme/edge geometry, default/minimum sizes, focus, interrupted/reduced motion, Search scrolling, rejected Appearance changes, list prefixes, shared editing and layered Escape. Release screenshots confirm the corrected toolbox and Search behavior. See [the results and evidence](../plans/README.md#verification-status).

Large-content All Notes performance remains unresolved: 1,284 notes with roughly 20 KB bodies took 54.2 seconds to map and used 379.4 MiB PSS on the isolated 125% output. Final visual approval and the broader desktop/installation checks below remain pending. Historical "not tested" statements later in this file describe their earlier runs.

## Isolated setup

Close any running Stickies instance first. The application is single-instance, so a second invocation can connect to an existing process despite different XDG paths.

In a dedicated terminal:

```bash
stickies_test_dir=$(mktemp -d /tmp/stickies-manual.XXXXXX)
export XDG_DATA_HOME="$stickies_test_dir/data"
export XDG_CONFIG_HOME="$stickies_test_dir/config"
export XDG_CACHE_HOME="$stickies_test_dir/cache"
cargo build --locked --release
./target/release/stickies &
stickies_test_pid=$!
```

Close or flush all test editors before stopping the process. This is a disposable-data test, not a shutdown durability test:

```bash
kill -TERM "$stickies_test_pid"
wait "$stickies_test_pid"
```

Restart from the same terminal to reuse only this test data. Keep failed-case data until investigated; delete only this disposable directory when no longer needed. Never clear the normal database for a test.

## Shutdown durability

Build with `cargo build --release --locked`. In an isolated Wayland display and private session bus, run `STICKIES_TEST_SANDBOX=1 python3 tests/probe_shutdown.py`. The probe uses disposable data, types a new title and body, verifies autosave has not run, sends SIGTERM within 200 ms of typing, and verifies a clean exit and both saved fields. Repeat with `--signal SIGHUP` and `--signal SIGINT`. It prints and preserves the test directory and stderr for inspection. Never run the input probe on the normal desktop.

Deliver a session-end notification in the isolated session and verify the same pending text is saved. Reject a shutdown write in disposable data and verify stderr names the failing note and error, other active sessions still save, and the application exits.

## Notes and editing

1. Empty start shows New note and All Notes with no seeded content. Restarting an empty database keeps it empty.
2. Create notes from both the strip and All Notes. Confirm each appears in both collections, including a note left blank. Edit title/body, close immediately once and after autosave once, restart, and verify complete text.
3. Open All Notes with existing notes. It must start without a selected row or note preview and show "Choose a note". Search, sort and switch collections before selecting; those actions must not choose a note. Select a row and confirm its preview opens; refresh/reordering keeps that selection while the note remains in the results. Close All Notes and reopen to confirm it starts unselected again. New note still opens its new editor. Open the same selected note in All Notes and a separate window. Alternate edits, close either view, and confirm the other keeps its buffer and pending save. Pop out again focuses the existing window. Pop out a different note opens a separate window.
4. Pin a note from All Notes, create/edit unpinned notes, and verify pinned-first ordering. Selection in All Notes follows the same note when rows reorder. Changes to other notes do not close an open docked note. The individual panel's Pin only enables Always open for that view; it must not change ordering or another view's Pin state.
5. Search titles and bodies, change sort order, clear search, and verify empty/no-result states. Check that a failed save prevents switching away from dirty text.
6. Copy text from More before autosave and verify the latest live body is copied, including checklist markers.
7. Toggle checklist items in each live view. Verify shared state, strike-through, Enter continuation, empty-item termination, Backspace removal, and persistence. Exercise existing bullets and numbered lists. Archived previews may still show markers.
8. Archive, find through All and Archived, and restore. Archived content is read-only. Delete and use the toast's Undo, including an archived note. Delete without Undo, restart, and verify it stays absent. Deleting all test notes must not reseed examples.

Existing storage/session regressions exercise failed-write retention and retry. Do not damage the real database to reproduce them.

## UI and desktop

9. Review Paper, Playful, Minimal Light and Minimal Dark in All Notes and individual windows, with Color, Text, More and the selection bubble open and closed. Compare the same note's fill, tag accent, corners, border, and shadow in both editing views. Verify Virgil, Inter, Mono and Kalam rendering, readable icons, selected controls, long content, scrolling, resizing and keyboard focus. Test individual notes at 320x376, 300x280 minimum, and larger sizes. Opening and closing tools must preserve the note's dimensions. Drag the transparent corner resize area on each docked edge and in a separate window; the corner should follow the pointer, remain within the available area, and keep Close usable. Separate notes should open floating on the reference compositor and remain resizable. Preserve per-note overrides across restart and global theme changes.
10. Use All / Active / Archived filters. Verify archive badges and read-only behavior. No pull-down gesture or collection-menu shortcut is expected.
11. Reveal and dismiss the four-note strip, vertical on Left/Right and horizontal with upright labels on Top/Bottom. Test a brief edge graze, the 140ms dwell and 350ms leave grace, all three utilities and dormant-marker dragging. Utility/tab clicks must preserve its position. Compare four active slots with All Notes under Recent/Title and pinned priority; reopen All Notes and verify the current session's sort. Blank creation must refresh the edge immediately. Search active titles and bodies including overflow, type with native focus, use Up/Down/Enter, click a result, create from a nonempty no-match query, and return with Escape or a click into another window. Archived/deleted notes stay excluded. Inspect full-title tooltips and ellipsis. Open a docked note, verify hover cannot replace it, verify clicking another tab deliberately switches it after a successful save, then close/Escape and verify the tabs stay at the same position. Tabs and utilities must remain visible beside both notes and Search. Check pointer direction and note/strip bounds at both end positions. Overflow remains in All Notes.
The [October 9 motion frames and recordings](design-previews/2026-10-09-edge-motion/README.md) cover reveal, collapse, collapse re-entry outside the marker bounds, mid-reveal re-entry, a brief graze and leaving just after dwell. Check intermediate frames for fixed strip/note geometry and continuous progress. The final input rectangle must apply from reveal start until collapse finishes. Leave grace starts after an active reveal finishes. Search typing and Reposition stay immediate; reduced motion and disabled system animation jump to the final state.

12. Settings > Behavior > Docked note timeout offers Off / 1 / 2 / 5 minutes, default 2. Select each value, restart and confirm it persists. Off keeps docked notes open; otherwise a docked note with individual Pin off closes after the selected interval. Pin on prevents auto-close; turning it off starts a fresh selected interval. Changing the setting while a docked note is open restarts or cancels its timer immediately. Pointer-enter and pointer motion over the note or tools reset the timer alongside keys, clicks and scroll. Separate notes and All Notes never auto-close, with individual Pin on or off. Activity in those other views must not reset the docked timer. A rejected Settings write restores the saved timeout choice.

Open individual tools in docked and separate notes. First Escape dismisses the tools while preserving the note; second Escape closes the note, including with Always open enabled. Without an open popover, Escape closes immediately. Repeat with focus in the note body and in the popover. Close remains available. In disposable data, reject a save and confirm Escape still dismisses tools first but retains the dirty note when closing fails; restore writes and retry.

For old disposable configs, omit `inactivity_seconds` and verify the two-minute default. Valid legacy custom values such as 90 seconds also load as two minutes without rewriting the file on load. Existing invalid values outside the old range and unknown keys remain errors.

Use S/M/L and Regular/Bold from both editors' Text popovers and confirm both editing views update immediately. Restart and verify the note's choices persist; also check an archived preview before restoring. Font family is available only in global Settings. Change it with an older per-note font override saved in the disposable database and confirm both editors follow Settings. Numeric text sizes remain provisional until the user reviews the preview.
13. Reduce motion and system-disabled animation suppress motion. Fullscreen and Omarchy DND hide the strip as configured while open notes remain available. Test the hide preferences disabled too.
14. Verify All Notes and separate notes float and move on the reference compositor. Inspect stderr for graphics warnings and save any reproducible trigger. Measure labeled idle, editing and animation intervals against the budget in current-milestone.md.

## Phase 5A/5B completion

15. Launch again while running. No second persistent process or duplicate strip should appear. Repeat with `--new-note`, `--search`, `--toggle-strip` and `--show-all-notes`, one per invocation. Verify New note focuses the title, repeated Search preserves its query, Toggle strip reveals/collapses safely and All Notes reuses its window. Start without an existing instance using each flag separately. Follow the [README D-Bus commands](../README.md#verify-the-exported-d-bus-actions) to list and activate all four exported actions. A rejected save must retain the docked editor when an action would close it; Toggle strip cancels Reposition when collapsing.
16. Test valid partial configuration, invalid values, and unknown keys in the disposable config. Invalid files must remain unchanged. Restore valid contents before testing settings changes.
17. Toggle `launch_at_login` in the disposable config. Verify creation/removal of its autostart entry and that invalid configuration does not alter it.
18. Separately verify an installed executable launches after an actual login. The desktop entry uses `Exec=stickies` and `TryExec=stickies`, so the desktop session must find the executable. Check the application icon and launcher entry as part of installation.

Cursor-following multi-monitor behavior and Google Drive sync require their own checks when those phases begin. They are not implemented acceptance claims for this checkpoint.

## Planned Settings checks, reference received October 1

These apply only after the Settings scope is settled and implementation is authorized. The image and draft are saved in [design-references/2026-10-01](design-references/2026-10-01/README.md). No Settings pass has been started.

- Review the scrollable section layout, outlined selected segments, Close, keyboard focus and theme contrast in the chosen compact host. Keep Minimal System/Light/Dark and global font behavior available. Check controls restore their saved state after rejected writes.
- Verify true strip enablement separately from hover enablement, with a docked note open and dirty. Reposition/presets should work on all four anchors, stay within available space and persist. Verify the agreed commit/cancel path and unchanged separate notes.
- Confirm default color affects only newly created notes from All Notes, the edge and later search creation; existing notes retain their tags. Use the same shared palette throughout.
- Recheck launch-at-login reconciliation and installed launch. Settings > Shortcuts contains selectable Hyprland bind examples and Copy buttons for all four CLI flags. Paste each copied line into a text editor and confirm the exact command; check wrapping and keyboard access in the compact drawer. Bind examples do not register global keys. About must use the real build version, app icon and repository link.


## Note controls checks, October 1

- All Notes keeps three footer buttons. More contains Checklist/list tools, size/weight, the shared 16-color grid, relevant pattern choice and Copy/Archive/Restore/Delete. Archived editing controls are disabled while Copy/Restore/Delete remain usable.
- Choose colors from either view, check immediate shared selection/fill/accents, and reopen the disposable database. Include original colors, bright colors and neutrals. The original four saved names and shades must survive migration. Check notes with custom styles, pin/order metadata and archive/delete state.
- Routine Saving/Saved indicators appear only in All Notes. Inject failures only in a disposable database. Verify plain-text error toasts appear in the affected views, repeated buffer notifications do not queue duplicate errors, and successful retry clears stale toasts. Close/switch guards must retain unsaved content, including with Always open enabled. Restore a rejected size/weight control to its saved selection.
- Repeat the default/minimum-size and focused-panel Escape checks. Review [saved controls previews](design-previews/2026-10-01-note-controls/README.md); these do not settle final text sizes or replace the remaining milestone checks.

## Edge review record, October 1

The [edge GTK gallery](design-previews/2026-10-01-edge-strip/preview.html) is under visual review. Separate theme folders contain all four anchors, resting/tabs/search states and default/minimum docked notes. The accompanying README distinguishes native checks, GTK assertions and remaining milestone checks. Prior theme approval remains intact; edge approval is pending.


## October 3 edge correction review

Use the [tested disposable launcher](design-previews/2026-10-01-edge-strip/correction-2026-10-03/README.md#tested-disposable-review-launch) for each edge and offsets 0/0.5/1. It uses temporary configuration/fixtures and a separate D-Bus session, so the real app does not need to be closed. Verify the workspace release build, then inspect opening, Close/Escape and Search at normal speed and through intermediate frames. Tabs must return at the edge, controls must retain their size, and outgoing tabs must not overlay the incoming panel. Hover/focus feedback must keep button bounds stationary.

Check Top/Bottom wide tabs, title truncation/tooltips, group alignment, four/fewer/empty states and both ends. Right/Bottom transparent space must pass desktop input through while the note remains resizable. Press the resting marker before dwell, hold beyond 140ms, drag along the edge and verify persisted position. A slow approach can reveal tabs before pressing; there is still no discoverable edge selector or reposition mode. The [current correction record](design-previews/2026-10-01-edge-strip/correction-2026-10-03/README.md) distinguishes native interactions, GTK bounds/captures and remaining hands-on/milestone checks.


## October 7 lightweight edge review

Use the [current gallery and normal-speed recordings](design-previews/2026-10-07-lightweight-edge/preview.html) and the existing disposable launcher. Earlier October 1/3 sections are historical. Inspect all four edges and themes with animation enabled and reduced motion enabled. No reveal, collapse, control hover/focus, panel replacement, resizing or pointer movement should animate. Check intermediate frames, not just settled screenshots. Appearance drawer animation remains outside this edge requirement.

Keep tabs visible and stationary beside notes and Search. Switch with clicks, cross the gap slowly, resize near both screen ends, and return with Close/Escape. Tab hover must never switch notes. Verify that typing reaches Search immediately after a strip click and that clicking a tab while Search owns focus still opens that note. Click disposable All Notes through blank transparent space to check click-through and intentional Search dismissal. Bare wallpaper may retain keyboard focus under compositor policy.

Reject a disposable database write and try another tab, Search, New note, Close and Escape. The old editor and dirty content must remain. Restore writes and verify successful retry. Check keyboard selection/Enter, overflow body search, no-match creation, separate-window reuse, Always open and unchanged per-view timers. Tabs/Search remain transient under fullscreen and DND; committed notes retain their existing protection. Press and hold the resting marker through dwell before dragging. Do not add a reposition mode as part of testing.

See the [verification record](design-previews/2026-10-07-lightweight-edge/README.md) for completed checks and limitations. Stop for visual and hands-on review.

## Settings positioning, October 7

Use the [positioning review gallery](design-previews/2026-10-07-settings-position/preview.html) and disposable launcher. This pass follows checkpoint `79a78b1` and remains uncommitted.

- Open the gear and scroll Settings to Edge strip. Check Left/Right/Top/Bottom and the corresponding vertical or horizontal position presets in every theme. Reopen Settings after direct marker dragging; a custom position leaves all three presets unselected.
- Choose a preset, restart the disposable app and confirm its edge and position remain saved. Rejected configuration writes must restore the previous button selection.
- Select Reposition. Drag the visible strip along its edge; it must stay within the screen. The Done/Cancel panel follows immediately without animated movement or blocking the rest of the desktop.
- Done saves. Cancel and Escape from the panel or All Notes restore the saved position. A failed Done retains the preview and shows an error; retry after restoring write access. Cancelling must work without disk writes.
- Test with a dirty docked note. A successful save permits repositioning; a failed save must keep the note and its content. Existing editor switching, separate windows and save ownership must still pass their regression.
- DND activation and closing All Notes cancel a temporary position. Reposition respects transient fullscreen/DND visibility.
- Check the compact and minimum Settings layouts, scrolling, keyboard focus and the unchanged Appearance controls. Remaining Behavior controls and global shortcuts are outside this pass.

## Slim strip and empty new notes, October 7

Use the [current before/after gallery](design-previews/2026-10-07-slim-strip/preview.html). Check 24px strip thickness, visible titles, readable utility icons and hover/focus contrast on every edge and theme. Notes and Search must retain the persistent strip and immediate state changes. Reposition must still drag and save/cancel correctly with the smaller controls.

- Create a blank note from the strip and from All Notes. Close it or switch away without typing; it must disappear. Repeat with only spaces in title/body, and by creating another note immediately.
- Enter a title only or body only and leave; it must remain. Existing blank notes from before this pass must remain.
- Open the new blank note in two views. Leaving one must preserve it; leaving the last must remove it. Pop out a docked blank note and verify it stays editable until the separate window closes.
- Repeat the individual close path with Escape and the existing inactivity timeout. All Notes never auto-closes.
- Reject writes in disposable data. Failed save or blank-note removal must retain the editor. Restore writes and retry; content must save or the blank must disappear as appropriate.

The verification record distinguishes completed GTK assertions from remaining native hands-on review.

## Edge-title hover, October 9

Use the [hover comparisons and pointer recordings](design-previews/2026-10-09-edge-hover/README.md). Hover each title across all four edges/themes: its color and text should stay clear, its outline should strengthen, and it should enlarge slightly while staying attached to the screen edge. Move quickly between titles, leave midway and move onto the enlarged free edge; the effect should reverse smoothly and clicks should still open the intended note. Selected tabs must retain their selection styling. Reduced motion and disabled system animation apply the hover size immediately; Reposition keeps titles at their normal size. Surface dimensions, note/pointer placement and 140/350ms hover timers retain their existing behavior. This supersedes the historical prohibition on title-hover motion.

The follow-up hover handoff uses matching 120ms entry and return timing. The old title returns as the new title enlarges, both from their current sizes, and hover continues smoothly during reveal. This adjustment was formatted and compiled only, as the user requested no tests. The earlier recordings show the preceding 110ms return implementation.

The arrow-attachment follow-up overlaps the base with the note and clips its stroke against the shared sheet outline. Check that it joins the note cleanly on all edges, including Playful waves, without a floating triangle or a line crossing its base. Position and fade behavior retain their existing rules. No tests or recordings were run for this follow-up, as requested.

Pressed edge titles now use a stronger outline with their original fill, replacing the grey pressed overlay. Check mouse-down and release on each theme, including selected titles. This CSS correction was compiled only; no tests or recordings were run under the current user instruction.

The latest title-feedback correction removes extra hover/press rings and outlines. The slight enlargement remains the hover cue; the existing selected-note indicator and keyboard-focus outline remain. No tests or recordings were run, as requested; only the release build was compiled.
