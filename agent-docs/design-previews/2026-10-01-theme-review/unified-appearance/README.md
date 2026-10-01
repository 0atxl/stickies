# Unified Appearance drawer review

The user directly authorized removing the circled back arrow, making the Appearance heading and lower controls open/close and animate as one panel, and dimming the filters with the notes. [Open the four-theme gallery and recorded GTK animation](preview.html). Theme captures are saved separately in their unified-appearance directories.

Concrete cause: the heading was a sibling in the filter row, outside the animated OverlaySplitView sidebar, while its control body was inside. Filters were also outside the split-view content and therefore outside its modal scrim.

The small correction in manager.rs nests heading and controls inside one sidebar and filters/notes inside one content box. It removes the Appearance back button and manual header show/hide callback. Settings still toggles the existing drawer. CSS gives its heading/body a continuous theme background and theme ink, with one edge border. The 44px heading aligns with the filters. Fonts, patterns, sidebar/note spacing, palettes and individual-note behavior are unchanged. No extra animation implementation or design-system refactor was added.

Verification uses actual GTK widgets and scratch data/config/cache. Assertions cover drawer ancestry, arrow absence, default/minimum heading/filter alignment, filters blocked by native modal hit testing while open and restored afterward, Settings toggling, scroll attachment, existing wide responsive mode, and default/minimum notes with open tools. Frame sampling captures actual opening/closing GTK motion and asserts header/body positions match throughout, with stationary filters and multiple intermediate drawer positions. Reduce motion is respected. The GIF only encodes the captured frames and holds endpoints for review.

Formatting, 22 non-display tests, strict Clippy, release build, diff checks and the existing GTK shared-editor regression pass. Final logs have no GTK warnings/errors. Native pointer resizing, Escape, timers and broad milestone checks were not repeated. No desktop configuration changed.

This requested correction is under visual review across themes; Minimal Dark remains unapproved. Existing Playful/Light approvals precede these corrections. No commit or push.


## Approval and checkpoint

The user confirmed visual review was already complete on October 1, 2026, and explicitly requested commit and push. All three themes and the subsequent shared corrections are approved. Earlier review-status statements are historical. Native pointer/Escape/timer checks remain outstanding for the wider milestone; the request does not authorize the next implementation stage.
