# Playful sidebar correction

Current status: the user approved Playful after the sidebar correction and explicitly authorized Minimal Light next. The verification record below describes the completed Playful pass. Minimal Dark is not started. No commit or push is authorized.

The user approved this Playful correction and authorized Minimal Light next. Minimal Light is now under review; Minimal Dark is not started or authorized. The earlier review assets remain intact.

[Open the fresh gallery](preview.html).

Only `assets/all-notes.css` changed application presentation in this follow-up. The 240px sidebar's right edge now continues through the full-width filter band. A theme-scoped CSS background line at x=239 joins the existing sidebar border without changing widget layout or drawing a horizontal divider over the note preview.

Row backgrounds moved onto their rounded content child. The row itself has a square, straight separator, with 5px padding before the line and 5px margin after it. The highlight is 55px high; tag markers have matching margins for 11px round dots. The selected-row separator remains visible. Hover highlights use the same child, and selected hover keeps the stronger selection fill. Titles and saved dates still fit beside each other; long titles truncate.

Actual GTK captures include 720x540 All Notes with the six-item Weekend Plans fixture, selected hover, a long selected title with its date, 352x376 and 300x280 individual notes, and both open tool panels. Today's Plan also matches the individual reference content. Exact-size assertions and unchanged note dimensions with tools open passed. The hover image uses GTK's programmatic hover state, not a native pointer test.

Typography is unchanged: title Small/Medium/Large 27/30/33px and body 14/17/20px. Regular/Bold are 400/700. Global Virgil, Inter and JetBrains Mono choices remain. App chrome uses Inter; sidebar titles are 14px, snippets 11px and dates 10px. These remain provisional until visual approval.

Formatting, 22 non-display tests, strict Clippy, release build and diff checks passed. The disposable GTK helper also verified shared title/body sizes, weight, all sixteen colors and persistence, global fonts, selection by matching title and note sizes. It produced no GTK warning or error. The existing shared-editor regression passed in the preceding pass and was not repeated for this CSS-only correction. Native pointer resizing, Escape and timers were not repeated.

Paper and both Minimal variants were rendered and inspected at the existing main/default-note/tools sizes. No CSS outside Playful or shared application code changed in this follow-up. Existing other-theme appearances remain intact. References are linked from the gallery; source images are unchanged.

Earlier remaining differences still apply: Virgil differs from the mock handwriting; compact body text leaves more space; long text scrolls at minimum size. No automatic underline or decorative note-face pin badge was added. The toolbar keeps existing compact controls and sixteen colors.

Changes remain uncommitted. Playful was explicitly approved after this pass. See the [Light review](../../minimal-light/preview.html) for the current cycle.
