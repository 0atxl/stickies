# Playful visual review, October 1, 2026

Current status: the user approved Playful after the sidebar correction and explicitly authorized Minimal Light next. The verification record below describes the completed Playful pass. Minimal Dark is not started. No commit or push is authorized.

Playful is approved after the sidebar correction. Minimal Light is now under review in its own gallery. Minimal Dark has not started and needs explicit approval and authorization. Paper is preserved. No commit or push was made.

[Open the comparison gallery](preview.html). Captures show native GTK widgets and Cairo note drawing on Wayland, using disposable data/config/cache directories. The HTML only arranges saved renders and references. It does not recreate the application.

## Changes

Playful has a larger title relative to smaller body text, compact 14px checkboxes, a 24px filter pill and 11px sidebar snippets. The header/editor spacing is tighter. The filter divider stops at the 240px sidebar. Sidebar tag markers are round dots. Sidebar dates and title truncation remain intact. The existing supplied Playful SVGs remain in use, with clearer 22px footer icons.

The note keeps its tag-colored fill. Its seeded smooth outline now has 48px segments, a 3.4px wave amplitude and a 3.2px dark stroke. The offset shadow is 6px right and 7px down. The drawing reserves another 4px at the right/bottom so the shadow fits. Chrome and tools are near white, with pink #ff4f91 accents and #ffe5ee selections. Selected tools and colors use pink outlines. Typography, colors and drawing choices are scoped to Playful; no shared design-system refactor was added.

| Provisional typography | Small | Medium | Large |
|---|---:|---:|---:|
| Title | 27px | 30px | 33px |
| Body | 14px | 17px | 20px |

The default note font is Virgil. Global Settings still selects Virgil, Inter or JetBrains Mono for both editors. Note text uses weight 400 for Regular and 700 for Bold. App chrome stays Inter. Sidebar titles remain 14px/700, snippets are 11px, dates and main metadata are 10px, and the All Notes heading remains 23px/800. Both Playful editors use 1px above and 3px below paragraphs, plus the existing top margin and control spacing. Numeric choices await visual approval and do not approve another theme's sizes.

## Captures and remaining differences

All Notes is 720x540 with all six reference Weekend Plans items. The selected note was found by title and explicitly selected before capture. The fixture also has five other sidebar entries, fixed updated dates from Apr 26 through Apr 16, and a long Project Ideas title. The actual note color is Sage, so its sidebar dot also follows Sage rather than copying the reference's mismatched pink dot.

Individual notes are 352x376 and 300x280. Both Weekend Plans and the individual reference's Today's Plan are shown. Today's Plan contains all four items and an ordinary heart character. Tools are shown as separate GTK popover captures, since they have their own native surface; opening them preserves the note's dimensions at both sizes. They retain the compact Pin/Checklist row, Aa, S/M/L, Regular/Bold, sixteen colors and More.

Virgil differs from the reference handwriting. The compact body leaves more space, especially in the four-item note. No automatic title underline or decorative note-face pin badge was added. Long titles scroll horizontally inside their entry at the minimum width; the six-item Weekend Plans list wraps and scrolls vertically. The tall mock toolbar is adapted to the existing compact controls. Checklist semantics remain unchanged.

## Verification

- Formatting, strict Clippy, 22 non-display tests, release build and diff checks pass.
- The existing GTK shared-editor regression passed separately. It covers shared state and failed-save retention.
- Disposable GTK checks passed for exact 720x540 main, 352x376 default and 300x280 minimum sizes, selected reference content, tools preserving note dimensions, shared S/M/L title and body values, Regular/Bold, all sixteen colors including persistence, and all three global fonts.
- Paper and both Minimal variants were rendered before/after at main/default note sizes with their tools. Their existing note appearance and controls are preserved. Some images differ in live hover, scrollbar fade and rounded-edge antialiasing. Paper's individual note and all three other-theme tools were pixel-identical in the comparison. These are regression captures, not implementation or approval of either Minimal theme.
- Native pointer resizing, Escape, timer behavior, edge anchoring and full milestone performance checks were not repeated. No desktop configuration changed. Disposable preview windows were floated/resized through their own compositor addresses.

Two initial helper runs failed exact-size assertions because the backend/window state was wrong; neither was used as a preview. A concurrent baseline helper launch then crashed in GTK while assigning an application to a window. The core showed `gtk_window_set_application`; a duplicate preview application identity is the likely cause. Separate identities and serial launches succeeded. This temporary helper issue is not claimed as an application fix, and no user notes were involved.

Application changes and documentation remain uncommitted. Playful was subsequently approved and Minimal Light authorized. See the [Light review](../minimal-light/preview.html) for the current cycle.
