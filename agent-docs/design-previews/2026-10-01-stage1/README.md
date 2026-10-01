# Stage 1 visual preview

October 1, 2026. These are renders of the actual GTK widgets using disposable notes, a separate D-Bus session, and the reference Wayland compositor. The original design references remain in `design-references/2026-09-30`.

Open the [interactive gallery](preview.html) to switch appearances and individual-note sizes. The [overview](overview.png) shows all four appearances at their native capture sizes. Each theme also has a separate comparison image: [Paper](paper-preview.png), [Playful](playful-preview.png), [Minimal Light](minimal-light-preview.png), and [Minimal Dark](minimal-dark-preview.png).

## Implemented in this pass

- Matching note drawing, fills, tag accents, rounded corners, borders, and shadows in both views. Paper has a side strip, Minimal a top strip, and Playful a tag-colored fill with its existing seeded waves.
- Playful sidebar dots; Paper and Minimal vertical bars. Rounded controls, matching content padding, and horizontal More dots.
- All Notes remains 720x500 by default with Pin, Pop out, and More in its footer.
- Individual tools open on click as a compact floating panel. The opening button becomes clearer on hover. The panel has a collapse action and preserves the writing area and window dimensions. Existing font, color, list, ordering, Always open, pattern, and pop-out actions remain available.
- Both individual note types resize. Defaults are 352x376, minimum 300x280; a 600x500 sample is provided. Text wraps and scrolls vertically. Corner grips follow their edge anchor, and docked notes stay within the available area.
- Separate notes present with their existing fixed-size floating hint, then enable resizing when the mapped surface enters its initial monitor. This was verified on the reference compositor without adding or editing desktop rules.
- Docked resizing preserves the note's position along the edge. Right/Bottom keep the layer origin fixed on the resizing axis; the input region includes only the note.

## Verification

Formatting, strict Clippy, 20 non-display tests, the existing GTK shared-editor/failed-save regression, release build, and Git diff checks passed.

The same sample note was rendered in All Notes and individual editors across Paper, Playful, Minimal Light, and Minimal Dark. Default, minimum, and larger individual-note dimensions were checked, including the floating panel. An assertion verified opening tools does not change the note's dimensions.

Actual Wayland pointer input exercised the grips. A separate note changed from 352x376 to 432x416 for an 80x40 drag. Docked notes changed from 500x440 to 580x480 on Left, Right, Top, and Bottom; the appropriate drag directions were reversed on Right and Bottom. The provided docked images show the note before those drags.

Actual keyboard input confirmed Escape closes a separate note and a docked note on every edge with Always open enabled and the tool panel open. The normal All Notes window remained available.

## Pending review and later passes

Body text is still 21px. Titles remain 23px for Paper/Playful and 20px for Minimal. These values are provisional. The screenshots make wrapping and control proportions visible before choosing S/M/L values. Note-content defaults are Virgil for Paper/Playful and Inter for Minimal; UI text uses Inter.

This pass retains the existing checkbox rendering and settings. Their redesign awaits the user's references. Size/weight controls, broader colors, the agreed final pin roles, and routine/error save-feedback changes belong to stage 2. The existing infinity control and individual save indicator shown here are retained until that controls pass. The edge-strip redesign is stage 3.

Final visual approval, timer-duration checks, fullscreen/DND, installed login launch, portable compositor behavior, and final resource measurements remain in the milestone checklist. A successful preview does not close phases 5A/5B.
