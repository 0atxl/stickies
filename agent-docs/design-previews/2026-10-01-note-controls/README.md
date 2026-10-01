# Note controls preview, October 1, 2026

The user authorized completion of the remaining note controls. These are GTK-rendered tool panels from disposable notes, not generated mockups. The paired images show All Notes More and the individual panel in Paper, Playful, Minimal Light and Minimal Dark. The individual panel is hidden until opened; the three main footer buttons remain unchanged.

Open [preview.html](preview.html) for the gallery. The [archived panel](archived-tools.png) shows disabled editing controls with Copy/Restore/Delete available. Body size values remain provisional at 18/21/24px; fonts remain Settings-only.

## Changes

- Both editors share S/M/L, Regular/Bold and a compact four-column grid of 16 colors. The main menu also has Checklist/list controls, relevant pattern choice and a compact Copy/Archive/Restore/Delete row.
- Original Amber, Blue, Lilac and Sage storage values and shades are preserved. Schema 3 expands the color constraint in a transaction while preserving notes and styles.
- Routine save status is limited to All Notes. Plain-text error toasts are deduplicated and clear after successful retry. Session errors and dirty content remain retained; Close/switch guards are preserved.

## Verification

Formatting, strict Clippy, 22 non-display tests and the release build pass. The shared-editor GTK regression passes with all 16 colors persisted and reopened. Migration tests preserve every note column/style entry, exercise archive/delete Undo, and reject an orphan style with a transaction rollback.

The disposable GTK check passes size/weight changes initiated in All Notes and reflected in individual controls/fonts, reverse synchronization, all 16 colors and database reopen, all four appearances, default 352x376 and minimum 300x280 note allocations with tools open, rejected-style selection recovery, error-toast presence in both views, failed Close retention with Always open enabled, successful retry, archived read-only controls and Restore. The rendered panels were inspected for contrast and selected states.

The desktop was locked during the later checks. No keyboard/pointer input or desktop capture was used in the final check. Live Escape/focus and pointer resizing must be rechecked after unlocking; earlier stage 1 checks are recorded separately and are not claimed as a new pass here. Complete note-window captures are omitted for this reason. Portal shutdown messages from the isolated D-Bus session are not application graphics warnings; the previous invalid-rectangle warning did not recur.

Settings, checklist redesign, the edge strip and final text-size approval remain pending. This pass is uncommitted, and does not complete the milestone's wider compositor, login and performance checks.
