# Filter/list spacing correction

The user explicitly requested correcting the cramped filter band in one theme first, then copying its verified spacing to all other themes. This authorizes the spacing-only correction across Paper, Playful, Minimal Light and Minimal Dark without expanding other visual choices.

[Open the reference and four-theme GTK comparison](preview.html). Minimal Dark was implemented/rendered first and asserted at 44px filter-band height, 24px pill, 10px top/bottom padding, 8px first-row gap and 240px sidebar. Only then were the two padding values copied into the existing CSS rules for Paper, Playful and Light. No shared refactor was introduced.

The reference main images have about a 44px filter band when scaled to the current sidebar width, with a flat pill and a smaller gap before the first highlight. The retained 24px pill was already approved. Earlier 6px top/bottom padding produced a 36px band, followed by a larger 10px first-row gap. The correction balances that section while preserving separate selected fills/dividers and each theme's own colors, borders, icons, typography and row sizes.

All final theme renders passed exact GTK spacing and dimension assertions, matching selected content and tools preserving both note sizes. Formatting, 22 non-display tests, strict Clippy, release build, diff and the existing GTK shared-editor regression passed. Completed preview runs have no GTK warnings/errors. All default individual notes and tools are pixel-identical before/after. Minimal Dark's minimum note differs only at a blinking caret; Light's minimum note differs at a caret and scrollbar-frame region. Paper and Playful minimum notes are identical. Native pointer resizing, Escape, timer and edge checks were not repeated. No desktop configuration changed.

Actual screenshots are saved separately in each theme's filter-spacing directory. The gallery crops captured GTK PNGs for detail and links the full captures. Original references and historical previews remain preserved. All Notes default/minimum, individual default/minimum, tools, hover and long sidebar title/date captures are included.

Playful/Minimal Light appearances were previously approved; this requested spacing correction is under review. Minimal Dark remains under review and has not been approved. No commit or push.
