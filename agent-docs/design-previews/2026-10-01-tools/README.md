# Individual tools preview

October 1, 2026. These are actual GTK renders using disposable notes and a separate D-Bus session on the reference Wayland compositor. The original references and the earlier stage 1 screenshots remain unchanged.

The [panel comparison](tools-overview.png) shows Paper, Playful, Minimal Light, and Minimal Dark at their native sizes. Each floating panel is 163x293, and opening it leaves the individual note at 352x376. Pin sits beside Checklist. The [enabled Pin](paper-pinned-tools.png) has a filled icon and selected background.

The [size comparison](text-size-preview.png) shows the provisional S/M/L body sizes of 18/21/24px. Paper/Playful titles use 20/23/26px and Minimal titles use 17/20/23px. Regular/Bold apply weights 400/700 to note content. Final numeric sizes still await the user's review. Compare [Regular](paper-24.png) with [Bold](paper-bold.png), or inspect the matching All Notes examples, [Regular](paper-main-24.png) and [Bold](paper-main-bold.png).

Font family comes only from global Settings. Size and weight are per-note choices stored in the existing style TOML. Both editing views follow them immediately. The individual Pin controls only that view's Always open timer; the All Notes Pin continues to control persistent ordering. Infinity is removed from runtime assets. The original SVG packs remain stored as supplied.

## Verification

Formatting, strict Clippy, 20 non-display tests, the shared-editor GTK regression, release build, and Git diff checks passed. The GTK regression covers shared control selections, persisted text style after database reopening, failed style writes, independent Always open state, and existing failed-save retention.

The live preview checked actual Pango family, size, and weight in both editors for every size/weight choice across all four appearances. Global Inter, JetBrains Mono, and Virgil choices overrode an older saved per-note font. Saved choices survived reopening the database. All Notes footer Pin still changed ordering without enabling an individual view's Pin.

A shortened one-second test timer confirmed individual Pin prevents timeout and turning it off starts a fresh period without changing ordering. Actual Wayland pointer and keyboard input verified Escape closes notes with Pin focused on Left, Right, Top, and Bottom, as well as a separate note. Escape also worked from the nested More menu. A rejected database write kept the docked note open with its unsaved text; removing the rejection allowed the same Escape action to save and close it.

An early run emitted transient pixman rectangle warnings during text relayout. Neither the final normal run nor the debugger run reproduced them, and the captures remained readable. Final compositor and resource checks remain in the milestone checklist.

The broader palette, All Notes More controls, save-feedback redesign, settings, checklist styling, and edge-strip revision remain in their planned passes.
