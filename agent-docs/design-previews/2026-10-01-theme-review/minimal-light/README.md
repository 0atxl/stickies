# Minimal Light visual review

Minimal Light is approved. The user authorized Minimal Dark with "do the dark mode too now". The [Dark review](../minimal-dark/preview.html) is now active. This record preserves the Light implementation and its verification. Paper and approved Playful remain preserved. No commit or push was made.

[Open the GTK comparison gallery](preview.html). Screenshots use disposable data/config/cache directories and actual native GTK widgets with the current Cairo note drawing. The gallery only arranges captured images and original references.

## Concrete changes

Minimal Light now has a provisional 28px Medium title and 17px body. Its previous 20px title and 21px body gave little title hierarchy and pushed two Weekend Plans items below the visible area. Small/Large still apply minus/plus 3px. Both editing views use the same sizes and the same 2px above/3px below paragraph spacing. Checkbox chrome is a compact 14px with a thin 1px border.

The note is opaque white with the existing tag-colored 5px top strip and 16px rounded-corner geometry. Its soft shadow is slightly clearer and its thin outline lighter. Light-only branches control drawing and typography; Dark keeps its prior values. No drawing abstraction or shared design-system refactor was added.

Window chrome is near white with blue #1687ff theme focus/selection controls and pale #eaf3ff row selections. Header height is 60px; filters remain flat at 24px. Sidebar markers remain vertical tag-colored bars. Sidebar snippets are 11px, dates 10px, and titles remain 14px/700. The divider stays beneath the sidebar only; the sidebar's vertical edge continues through the filters. Selected and hovered fills sit inside rounded row content, with straight separators and a 5px gap outside them. Long sidebar titles still truncate beside their saved dates.

Note/editor padding is tighter, and the footer's existing standard SVG icons are 22px. Footer actions remain exactly Pin / Pop out / More. Tool panels use white backgrounds, blue segmented selection outlines and clear selected-color/focus rings. The existing controls, sixteen colors, pin roles, timers, save feedback, archive behavior and checklist semantics remain unchanged.

## Provisional typography

| Text | Small | Medium | Large |
|---|---:|---:|---:|
| Title | 25px | 28px | 31px |
| Body | 14px | 17px | 20px |

Minimal's default font remains Inter. Virgil and JetBrains Mono remain global Settings choices. The gallery includes an additional Virgil comparison because the reference uses handwriting; this does not change the default or saved user preferences. Regular is 400, Bold is 700. App chrome remains Inter, with its 23px/800 All Notes heading. These Light numeric values are approved. Dark decisions are recorded separately.

## Matching content and captures

All Notes is 720x540. Its Weekend Plans note has all six original reference checklist items, a Blue tag matching the reference treatment, an explicitly selected row, five other sidebar entries, saved Apr 26 through Apr 16 dates, and a long Project Ideas title. Normal selection, selected hover and long selected-title captures are included. Hover is a programmatic GTK state preview, not a native pointer test.

Individual Weekend Plans notes are shown at 352x376 and 300x280. The individual reference's Today note has Finish project draft, Read 20 pages, Workout and Plan tomorrow, shown with default Inter and with the global Virgil choice. Tools have their own native popover captures. Exact-size assertions confirm opening tools preserves default and minimum note dimensions.

## Remaining differences

Inter is the retained Minimal default; the reference is handwritten. The Virgil comparison uses the supplied font, which still differs from the mock handwriting. Smaller content leaves more open space, especially in the four-item Today note. Minimum-size long content wraps and scrolls, and long titles scroll horizontally in their entry. No automatic title underline or decorative note-face pin control was added. Sidebar markers follow the agreed Minimal bars, despite dots in the reference. The toolbar adapts the tall mock to existing compact controls and sixteen colors.

## Verification

Formatting, 22 non-display tests, strict Clippy, release build and diff checks passed. The existing GTK shared-editor regression passed separately, including shared state and failed-save retention. Disposable GTK checks verify title/body sizes in both editors, S/M/L, Regular/Bold, all sixteen colors and persistence, all three global fonts, matching selected content, exact dimensions and tools preserving note size.

Paper, approved Playful and untouched Minimal Dark were captured before/after at main/default-note/tools sizes. Their appearance is preserved. In the initial comparison, all Paper and Playful captures and Dark main/tools were pixel-identical; Dark's note differed only in a small scrollbar-frame region. Shared code changes are Light-only branches, and all added CSS excludes `.minimal-dark`. Source values for Paper, Playful and Dark remain unchanged. In the final saved captures, Paper main differs by a thin search-caret line; Dark note retains the small scrollbar-frame variation. Approved Playful remains pixel-identical in all three final captures.

Native pointer resizing, Escape, timer/Always open, edge anchoring and milestone performance checks were not repeated. No desktop configuration changed. Only the disposable preview process's windows were floated and sized for captures. No GTK warnings/errors occurred in the completed preview run.

Changes remain uncommitted. Minimal Light is approved and Dark is authorized. The Dark review record contains the current status.
