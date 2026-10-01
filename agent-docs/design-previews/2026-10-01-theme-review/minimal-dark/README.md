# Minimal Dark visual review

Playful and Minimal Light are approved. The user explicitly authorized Minimal Dark with "do the dark mode too now". Dark is implemented and under review. Stop here for visual review. No commit or push was made.

[Open the GTK comparison gallery](preview.html). All application images are actual GTK widgets, rendered with disposable config/data/cache and the current note drawing. HTML only arranges the captured images and original references.

## Concrete changes

Dark now has a provisional 28px Medium title and 17px body, replacing its old 20px title and 21px body. Both views use 2px above and 3px below paragraph spacing. These Dark choices were evaluated against its reference and the requested Minimal layout proportions. They remain separate branches from approved Light. Compact native checkboxes use 14px chrome, a thin 1px outline and the existing light checked fill.

Notes are opaque charcoal #272c32 with off-white #edf0f4 text, 16px rounded corners, the retained 5px tag-colored top strip, a thin light outline and a slightly stronger restrained shadow. Dark-only drawing guards leave other themes untouched. Window chrome is #20242a, dividers #414953, selected fills #34465e, hover fills #2c3541 and focus rings #66adff. Selected tool segments retain a blue border on hover. Disabled controls are dimmed to 40 percent opacity.

The main layout matches approved Minimal Light proportions. Header height is 60px, filter pills 24px, sidebar 240px. The horizontal divider stops at the sidebar edge, and that edge continues through the filters. Rounded selected/hover fills have a 5px gap before separate straight row dividers. Tag bars, saved dates and title truncation remain visible. Note padding and 22px footer icons match the compact layout. Footer actions remain exactly Pin / Pop out / More.

The smallest changes are in assets/all-notes.css, appearance.rs, editor.rs and manager.rs. No controls, persistence, checklist semantics, icon assets or shared design-system abstractions changed. Paper, Playful and Light styling decisions remain intact.

## Provisional typography

| Text | Small | Medium | Large |
|---|---:|---:|---:|
| Title | 25px | 28px | 31px |
| Body | 14px | 17px | 20px |

Minimal Dark defaults to Inter. Virgil and JetBrains Mono remain global Settings choices. The gallery also uses the supplied Virgil font with the reference's Today content. Regular is 400 and Bold is 700. App chrome is Inter, with a 23px/800 All Notes heading, 14px/700 sidebar titles, 11px snippets and 10px dates. Dark typography awaits visual review.

## Captures and reference comparison

All Notes is 720x540 with all six Weekend Plans items, a Blue tag, an explicitly selected row, five other sidebar notes, a long Project Ideas title and saved Apr 26 through Apr 16 dates. There is no Dark main reference, so the supplied Minimal Light main image guides its layout. The gallery pairs the original Dark individual reference with matching Today content: Finish project draft, Read 20 pages, Workout and Plan tomorrow.

Individual notes are captured at 352x376 default and 300x280 minimum. Both Weekend Plans and Today include default Inter, with an extra Virgil Today comparison. Native open tools and All Notes More captures are included. Exact-size assertions check tools preserve both note dimensions. Long selected titles, selected hover, unselected hover, focused controls and disabled controls have their own captures.

Hover and focus examples use explicit GTK state flags. The disabled example uses the existing widget sensitivity state. These captures verify actual GTK styling, rather than native pointer/keyboard interaction.

## Remaining differences

Default Inter differs from reference handwriting. Supplied Virgil also differs from the mock. Compact text leaves more space in the four-item Today note. The retained Blue tag palette shade differs from the brighter reference strip. The reference's large shadow is restrained at actual note size. No automatic title underline or decorative note-face pin was added. Sidebar markers follow the agreed Minimal bars despite dots in the light main mock. The existing compact tools contain sixteen colors. Long content wraps and scrolls at minimum size; long titles scroll inside their entry.

## Verification

Formatting, 22 non-display tests, strict Clippy, release build and diff checks passed. The existing GTK shared-editor regression passed separately, including failed-save retention. Disposable GTK assertions check title/body sizes in both editors, S/M/L, Regular/Bold, all sixteen colors with reopened-database persistence, all three global fonts, reference content selection, exact dimensions and tools preserving note size. The completed render log has no GTK warnings or errors.

Matched before/after captures inspect Paper, approved Playful and approved Minimal Light at main/default-note/open-tools sizes. Light is pixel-identical in all three captures. Paper note/tools and Playful main/tools are pixel-identical. Paper main differs only at its search caret, a 1px column at x183/y21-36. Playful note differs only at a body caret, a 1px column at x203/y209-231. Other-theme source values are unchanged in this pass.

Native pointer resizing, Escape, Always open/timers, edge anchoring and milestone performance checks were not repeated. No desktop configuration changed. Only this disposable preview process's windows were floated and sized.

Changes remain uncommitted. Minimal Dark is under review; corrections stay in Dark.
