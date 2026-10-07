# Current verification checklist

Run against the release build on the reference compositor. Record failures and pending checks, not a history of every run. Final UI approval belongs to the user.

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

## Notes and editing

1. Empty start shows New note and All Notes with no seeded content. Restarting an empty database keeps it empty.
2. Create notes from both the strip and All Notes. Confirm each appears in both collections, including a note left blank. Edit title/body, close immediately once and after autosave once, restart, and verify complete text.
3. Open the same note in All Notes and a separate window. Alternate edits, close either view, and confirm the other keeps its buffer and pending save. Pop out again focuses the existing window. Pop out a different note opens a separate window.
4. Pin a note from All Notes, create/edit unpinned notes, and verify pinned-first ordering. Selection in All Notes follows the same note when rows reorder. Changes to other notes do not close an open docked note. The individual panel's Pin only enables Always open for that view; it must not change ordering or another view's Pin state.
5. Search titles and bodies, change sort order, clear search, and verify empty/no-result states. Check that a failed save prevents switching away from dirty text.
6. Copy text from More before autosave and verify the latest live body is copied, including checklist markers.
7. Toggle checklist items in each live view. Verify shared state, strike-through, Enter continuation, empty-item termination, Backspace removal, and persistence. Exercise existing bullets and numbered lists. Archived previews may still show markers.
8. Archive, find through All and Archived, and restore. Archived content is read-only. Delete and use the toast's Undo, including an archived note. Delete without Undo, restart, and verify it stays absent. Deleting all test notes must not reseed examples.

Existing storage/session regressions exercise failed-write retention and retry. Do not damage the real database to reproduce them.

## UI and desktop

9. Review Paper, Playful, Minimal Light and Minimal Dark in All Notes and individual windows, with the floating individual tool panel and main drawer open and closed. Compare the same note's fill, tag accent, corners, border, and shadow in both editing views. Verify Virgil, Inter and Mono rendering, readable icons, selected controls, long content, scrolling, resizing and keyboard focus. Test individual notes at 352x376, 300x280 minimum, and larger sizes. Opening and closing tools must preserve the note's dimensions. Drag the corner grip on each docked edge and in a separate window; the corner should follow the pointer, remain within the available area, and keep Close usable. Separate notes should open floating on the reference compositor and remain resizable. Preserve per-note overrides across restart and global theme changes.
10. Use All / Active / Archived filters. Verify archive badges and read-only behavior. No pull-down gesture or collection-menu shortcut is expected.
11. Reveal and dismiss the four-note strip, vertical on Left/Right and horizontal with upright labels on Top/Bottom. Test a brief edge graze, the 140ms dwell and 350ms leave grace, all three utilities and dormant-marker dragging. Utility/tab clicks must preserve its position. Compare four active slots with All Notes under Recent/Title and pinned priority; reopen All Notes and verify the current session's sort. Blank creation must refresh the edge immediately. Search active titles and bodies including overflow, type with native focus, use Up/Down/Enter, click a result, create from a nonempty no-match query, and return with Escape or a click into another window. Archived/deleted notes stay excluded. Inspect full-title tooltips and ellipsis. Open a docked note, verify hover cannot replace it, verify clicking another tab deliberately switches it after a successful save, then close/Escape and verify the tabs stay at the same position. Tabs and utilities must remain visible beside both notes and Search. Check pointer direction and note/strip bounds at both end positions. Overflow remains in All Notes.
12. With the individual Pin off, two minutes without input closes each individual view independently. With it on, the view stays open. Turning Pin off starts a fresh inactivity period. All Notes never auto-closes. Close and Escape must still close an Always open note, including while its tool panel has focus. Input to one view must not reset another view's timer.

Use S/M/L and Regular/Bold from both the individual panel and All Notes More and confirm both editing views update immediately. Restart and verify the note's choices persist; also check an archived preview before restoring. Font family is available only in global Settings. Change it with an older per-note font override saved in the disposable database and confirm both editors follow Settings. Numeric text sizes remain provisional until the user reviews the preview.
13. Reduce motion and system-disabled animation suppress motion. Fullscreen and Omarchy DND hide the strip as configured while open notes remain available. Test the hide preferences disabled too.
14. Verify All Notes and separate notes float and move on the reference compositor. Inspect stderr for graphics warnings and save any reproducible trigger. Measure labeled idle, editing and animation intervals against the budget in current-milestone.md.

## Phase 5A/5B completion

15. Launch again while running. No second persistent process or duplicate strip should appear.
16. Test valid partial configuration, invalid values, and unknown keys in the disposable config. Invalid files must remain unchanged. Restore valid contents before testing settings changes.
17. Toggle `launch_at_login` in the disposable config. Verify creation/removal of its autostart entry and that invalid configuration does not alter it.
18. Separately verify an installed executable launches after an actual login. The desktop entry uses `Exec=stickies` and `TryExec=stickies`, so the desktop session must find the executable. Check the application icon and launcher entry as part of installation.

Cursor-following multi-monitor behavior and Google Drive sync require their own checks when those phases begin. They are not implemented acceptance claims for this checkpoint.

## Planned Settings checks, reference received October 1

These apply only after the Settings scope is settled and implementation is authorized. The image and draft are saved in [design-references/2026-10-01](design-references/2026-10-01/README.md). No Settings pass has been started.

- Review the scrollable section layout, outlined selected segments, Close, keyboard focus and theme contrast in the chosen compact host. Keep Minimal System/Light/Dark and global font behavior available. Check controls restore their saved state after rejected writes.
- Verify true strip enablement separately from hover enablement, with a docked note open and dirty. Reposition/presets should work on all four anchors, stay within available space and persist. Verify the agreed commit/cancel path and unchanged separate notes.
- Confirm default color affects only newly created notes from All Notes, the edge and later search creation; existing notes retain their tags. Use the same shared palette throughout.
- Recheck launch-at-login reconciliation and installed launch. Shortcut badges must describe actual available bindings, including an unassigned state, rather than show the reference examples as working hotkeys. About must use the real build version, app icon and repository link.


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
