# Minimal Dark unified Appearance drawer

The user authorized removing the circled Appearance back arrow, combining the heading and control body into one animated panel, and dimming the filter row with the background. [Review this theme](preview.html), or [all themes and actual GTK animation](../../unified-appearance/preview.html).

The header and scrolling controls now share one native OverlaySplitView sidebar. The filter row and notes share its content area. This lets the existing drawer animation and modal scrim cover the complete panel/background without manual header visibility or separate animation state. The heading remains aligned with the 44px filter row. The Settings button continues to toggle the entire panel; the circled arrow is removed. Header text uses theme ink. Palette, typography, spacing, patterns and individual notes remain unchanged.

GTK assertions verify common parentage, no back arrow, exact 44px heading/filter alignment, modal hit testing blocking filters while open and restoring access when closed, Settings toggling, scrolling with attached header, 720x540 and 680x460 layouts, retained 980px responsive layout, default/minimum individual sizes and tools preserving note size. Native opening/closing frame captures show header/body moving together through intermediate positions while filters stay stationary. Reduce motion disables the GTK animation as before. A deferred allocation after opening keeps the native scrim sized after a wide-to-compact layout change. Pixel comparisons confirm the same dimming on filters and notes in all four themes with reduced motion enabled.

Formatting, 22 tests, strict Clippy, release build, diff and the existing GTK shared-editor regression pass. Final render logs have no GTK warnings/errors. Native pointer, Escape and timer checks were not repeated. All data/config/cache is disposable. No desktop configuration changed. This correction remains under visual review; Dark is not approved. No commit or push.


## Approval and checkpoint

The user confirmed visual review was already complete on October 1, 2026, and explicitly requested commit and push. All three themes and the subsequent shared corrections are approved. Earlier review-status statements are historical. Native pointer/Escape/timer checks remain outstanding for the wider milestone; the request does not authorize the next implementation stage.
