# Stickies — UI/UX Specification (consolidated, current)

This is the single source of truth as of this point in the design process. It supersedes
`stickies-app-plan.md` and `stickies-design-spec.md` — everything decided across both,
reconciled and de-duplicated, plus what's been settled since. Treat colors as a calibrated
starting palette, not hex-exact; the *relationships* (contrast, which layer sits above which)
are what matter most. Anywhere marked **OPEN** is a genuine unresolved decision, not an
oversight — don't silently resolve those when building from this doc.

---

## 0. Product identity, one line
A native sticky-notes app for Hyprland/Wayland, written in Rust + GTK4. Core promise: your
most-used notes are one hover away at the screen edge, not buried in a window you have to
find and open.

---

## 1. Design tokens

### Spacing scale (base unit 4px)
```
--space-1: 4px    --space-2: 8px    --space-3: 12px
--space-4: 16px   --space-5: 20px   --space-6: 24px
--space-8: 32px   --space-10: 40px
```
Note body inner padding = `space-6`. Sidebar row padding = `space-4`. Tool rail width =
`space-10`; icon-to-icon gap in the rail = `space-3`.

### Corner radius
```
--radius-sm: 6px   /* search bar, small buttons, switch tracks */
--radius-md: 10px  /* sidebar rows, settings swatches, toggle buttons */
--radius-lg: 16px  /* note window, main panels — reads closer to 8–10px visually */
```
The note's bottom edge is never a plain radius — see §4.1's border treatment.

### Shadow scale
```
--shadow-note:    0 1px 2px rgba(30,25,15,.04), 0 6px 16px rgba(30,25,15,.10)
--shadow-panel:   0 1px 3px rgba(30,25,15,.06), 0 10px 24px rgba(30,25,15,.08)
--shadow-popover: 0 2px 8px rgba(30,25,15,.16)
--shadow-hard (Playful only): 3px 3px 0 rgba(42,39,35,.85)   /* zero blur, offset only */
```
Paper/Minimal shadows stay soft and under ~10% opacity — this is a big part of why the app
reads "clean minimal" rather than skeuomorphic. Playful is the one exception: a hard,
zero-blur offset shadow, deliberately, for its neo-brutalist "stamped down" character.

### Typography — two roles, never mixed in one text block
| Role | Font | Weight | Size | Used for |
|---|---|---|---|---|
| Handwritten (content) | Kalam / Patrick Hand / Caveat | 400 | 20px title / 18px body | Note title + body text, **every theme, including Minimal** |
| UI (chrome) | Inter / Cantarell | 400–600 | see below | Everything outside the writing surface |

```
--font-panel-title:  16px/600   /* "All Notes", "Appearance" */
--font-row-title:    14px/600   /* sidebar note titles */
--font-row-subtitle: 13px/400   /* muted, --text-secondary */
--font-label:        13px/500   /* section labels: "Theme", "Font"... */
--font-caption:      12px/400   /* timestamps, muted */
--font-button:       13px/500
```
**The single easiest rule to accidentally violate:** handwritten font lives *only* inside the
paper writing area. The ×, tool-rail icons, timestamps, footer icons — all UI font, never
handwritten, in every theme. Confirmed explicitly: Minimal keeps handwritten note content too
— theme restyles the frame, never the content's identity.

---

## 2. Themes

### Paper (default)
```
--bg-app:          #F3EFE6     --bg-panel:        #FBF8F1
--bg-note:         #F6EFDB     --bg-row-active:   #F0E6D2
--border-hairline: #E5DFCF     --text-primary:    #2A2723
--text-secondary:  #8A8378     --text-placeholder:#B5AE9E
--accent-focus:    #C98A3B     /* selection states, focus rings — used sparingly */
```

### Minimal — light + dark (replaces the earlier "Glass" concept)
Same visual identity blur would have given (slate panel, thin accent line, restrained
selection) but **fully opaque, no compositor dependency**. This pivot happened after a real
screenshot showed background bleed-through with transparency and no blur running — see
history note in §7.
```
--bg-app-minimal-light:   #F4F5F7   --bg-panel-minimal-light: #FFFFFF
--bg-note-minimal-light:  #F7F8FA   --border-minimal-light:   #E2E5EA

--bg-app-minimal-dark:    #23262B   --bg-panel-minimal-dark:  #2B2F36
--bg-note-minimal-dark:   #2F333A   --border-minimal-dark:    rgba(255,255,255,.10)

--accent-minimal-line:    #4C8DFF   /* top accent line on open note + selected row border */
```
No `rgba` alpha below 1.0 anywhere in this theme — that was the entire point of the pivot.
Light/dark is a toggle *within* Minimal, not a 4th top-level theme.

### Playful — governing rule: one signature accent, tag color stays a dot never a fill, one decoration per screen
```
--bg-app-playful:        #FBF9FE   --bg-panel-playful: #FFFFFF
--bg-row-active-playful: #FCE7EE   /* soft pink tint, not lavender — matches the one accent below */
--border-playful:        #2A2723 (2px, bold — not a hairline)
--accent-playful:        #E24C86   /* the ONE signature color: selected tab, selected row, pin badge */
```
**Correction from an earlier draft:** this used to specify a lavender wash (`#F4EEFC`) for
the whole app background. The real running app uses a neutral near-white background instead,
with pink doing all the accent work (selected filter tab, selected row tint) — confirmed
against a real screenshot as the better version, and it's actually the more disciplined
choice per this theme's own rule: a colored wash *plus* a pink accent is two colors sharing
the "this is Playful" job, where neutral-plus-one-accent spends only one.
- **Row treatment:** same dot-not-fill pattern as every theme. Selected row gets
  `--bg-row-active-playful` + `2px solid --accent-playful` left border — border signals
  selection, dot signals category, kept visually distinct on purpose (this was the direct
  fix for an earlier cluttered draft that used color for both at once).
- **Card treatment:** background may be the note's own tag color (solid fill) instead of
  cream — legitimate Playful differentiator, same mechanism the edge strip's docked card
  uses. Pairs with the 2px bold outline + `--shadow-hard`.
- **Decoration budget:** at most one doodle per screen, near a heading, never scattered
  around the card. A colorful card already spends the screen's "boldness budget" — don't
  stack a doodle on top of it too.
- **Typography exception:** headings may use the accent color on a single word for emphasis
  — the one deliberate exception to the "handwritten only in writing area" rule's spirit,
  and only for a single word, never a whole heading or body text.

### Tag colors — shared across all themes, independent axis from theme
```
--tag-orange: #D98C2B   --tag-green: #6FAE68
--tag-purple: #A98FD1   --tag-blue:  #5B8DEF
```
A 3–4px solid bar (sidebar) or dot (edge strip, quick search), never a gradient, never a full
row/card fill except the one deliberate Playful/edge-strip exception above. This is a
**separate axis from app theme** — don't conflate in CSS naming (`note-tag-orange` vs.
`app-theme-minimal`).

---

## 3. Components

### 3.1 Individual note window

**Collapsed:**
- Default card ratio **~0.875:1** (taller than wide), resizable — not a fixed size.
- Header **44px**: title left (`space-6` inset), × right (`space-4` inset), 20×20 glyph in a
  32×32 hit target minimum.
- Body inset `space-6` on left/right/top; bottom ~12–16px reserved for the border treatment
  below — keep a `space-6` text-safe margin above it.

**Border/edge treatment — per theme, one shared technique:**
Perturb points along an edge and connect them either with straight lines (sharp jagged teeth)
or smooth curves (gentle wave) — same mechanism, different look:
- **Paper:** sharp torn teeth, **bottom edge only**, straight segments, amplitude ~3px,
  wavelength ~18px. Corners elsewhere stay plain rounded arcs.
- **Playful:** smooth wavy border, **all four edges**, curved segments, amplitude ~2.5px,
  wavelength ~32px, paired with the 2px bold outline.
- **Minimal:** no wobble at all — plain rounded rectangle. Minimal doesn't tear or wave,
  by design (glass/slate doesn't have a paper texture to disturb).
Two implementation routes: a static SVG 9-slice asset (works fine for Paper's bottom-only
case since the other three edges stay straight), or procedural draw seeded by note ID so each
note's wobble is unique but stable across redraws/resizes (needed for Playful's all-sides
case, where a stretched static asset would distort unevenly). The `wobbly_border.rs`
reference implementation covers both edge-selection and style (torn vs. smooth) via one
shared function.

**Expanded (tool rail open):**
- Rail width **40px**, full card height, hairline border separating it from the writing pane.
- **Finalized set, identical in every theme, 5 icons:** pin · checklist · "Aa" text style ·
  color · more. No sticker/infinity icon — retired; its original intent ("always on top,
  never closes") folded into pin rather than kept separate.
  - **Pin** does double duty: quick-access pinning *and* keeping the window on top. The
    "on top" half can't be forced by a Wayland client itself — implement via Hyprland's
    native `pin` dispatch (`hyprctl dispatch pin`) rather than fighting the compositor for
    it client-side. While pinned, × no-ops or prompts to unpin first.
  - **"Aa"** replaces separate font-size/weight icons — one button, one popover with size
    (S/M/L) and weight (Regular/Bold) together.
  - **Color** opens the tag-color picker, present in every theme. In Playful only, the same
    popover gets one extra section — background fill — since Playful is the only theme with
    a customizable card background. Same icon/position everywhere; only the popover contents
    vary.
  - Chevron toggle uses the `handle-grip` icon (three small dots), not a bare chevron on a
    hairline — the plain chevron was easy to miss, same fix applied to the edge strip's
    resting state.
- Card **grows wider by exactly the rail's width (40px)** when the rail opens — the writing
  area itself never shrinks.
- **Checklist content, current state:** typed `- [ ]`/`- [x]` text styled to resemble
  checkboxes — not yet real interactive state.
  **OPEN / future:** real structured checklists are planned. When built, use
  `Gtk.TextChildAnchor` + embedded `Gtk.CheckButton` rather than a separate data model — this
  keeps checklist items inside the same shared `Gtk.TextBuffer` used for cross-view sync (see
  §3.2), so a real checkbox doesn't break that sync model. Prototype the anchor approach
  (scrolling/selection/copy-paste around an anchor) before committing.
- **Animations:** rail open/close 150–180ms ease-out-cubic (slightly faster than GTK's
  Revealer default). Note open: scale 0.96→1.0 + fade, ~120ms. Note close: same reversed,
  ~90ms (closing should always feel snappier than opening).

### 3.2 All Notes window
- Default size **1100×620**. With settings open: **292px list / 488px preview / 320px
  settings** (sums exactly to 1100 — not a coincidence).
- Resizing: list+preview hold proportion; **settings overlays rather than squeezes** below a
  threshold near 1100px (tune against real minimum comfortable widths, not a hard cutoff).
  This is `Adw.OverlaySplitView`'s native `collapsed` behavior — put settings in that role and
  the behavior comes for free. **Correction:** `Adw.OverlaySplitView` has exactly two
  children (`sidebar`/`content`), not three — nest list+editor inside `content`, settings as
  the `sidebar` (position `end`). Responsive collapsing needs an explicit `Adw.Breakpoint`;
  it is not automatic.
- List min-width ~240px regardless of the above, so titles never get unreadably narrow.
- Both this window and individual note windows are **ordinary floating top-levels, not
  layer-shell** — verify the person's Hyprland floating rules actually keep them floating;
  that's compositor-side, outside the app's control.
- Top bar **64px**: "All Notes" left, search center-left (~340px default), sort/filter +
  gear right.
- **Filter tab row, directly below the top bar:** All / Active / Archived, plain text tabs
  with the selected one shown as a filled rounded pill in the theme's accent color (pink for
  Playful, `accent-focus`/`accent-minimal-line` for Paper/Minimal) — this row was missing
  from earlier drafts of this doc entirely, confirmed present in the real running app.
- Sidebar row **72px**, `space-4` padding, tag-color bar 4px wide/full height/flush to the
  row's rounded corner (mask the bar's own corner or it pokes out).
- Sidebar list: `Gtk.ListView` + `Gtk.SignalListItemFactory`, wrapped in `Gtk.SingleSelection`
  — **not** `Gtk.NoSelection`, which has no selection state at all and can't drive "which
  note is open." List insertion/removal is **not** auto-animated by GTK regardless of model
  choice — build it yourself (opacity+height transition on realize) or skip it for v1.
- **Cross-view sync:** don't fork the note's `Gtk.TextBuffer` per surface. Look it up by note
  ID from a shared registry in `NoteStore` (create-once, reuse) so the preview pane, a
  popped-out window, and the edge strip's docked card for the same note all share one buffer
  — live sync with zero custom merge logic.
- Footer: **the same 5 tools as the standalone note's rail — pin, checklist, "Aa" text
  style, color — plus pop-out (tooltip "Open in floating window") and more** (`Gtk.MenuButton`:
  Archive / Delete / Copy text). One control set, two layouts: vertical rail in the
  standalone note window (which has side space, not much footer space), horizontal row here
  (which has footer space, not side space) — same icons, same popovers, same behavior either
  way, so a person never has to pop out to a separate window just to change text size or tag
  color. No pencil — the preview pane is already directly editable by clicking in.
- **Animations:** row selection background cross-fade ~100ms. Switching the editor pane's
  note: `Gtk.Stack` crossfade, 150ms, not a hard cut.

### 3.3 Settings — **draft scope, not final**
- Current build: single Appearance panel (theme swatches, font, paper pattern, reduce
  motion). Panel width 320px; theme swatches 72×56px with the checkmark badge overlapping the
  border by half; segmented toggles use an *outline* selected state (1.5px border), not a
  filled background.
- **Draft section list for the fuller settings surface** (confirmed direction: one scrollable
  window with section headers, **not tabs**):
  - Appearance (as above)
  - Edge Strip (on/off, anchor edge, position, reposition control)
  - Behavior (launch at login, default tag color for new notes)
  - Shortcuts (quick search hotkey, new-note hotkey)
  - About (version, GitHub link)
- **OPEN:** exact final section list — the above is a working draft to prune, not a
  commitment.

### 3.4 Edge strip
Content-sized cluster near the edge, not a full-edge-length surface. Only ever shows *one*
note's full content at a time.

**State 0 — resting dots:** 4 dots, 8px diameter, 6px gaps, sized to content (~56px total),
tag-colored, full opacity. Anchor edge (left/right/top/bottom) and position along that edge
are both user-configurable. **Enter-dwell ~120–150ms** before reacting to hover at all — this
specifically guards against accidental edge-grazes, which matter more here than in a generic
app because Hyprland users routinely flick the cursor to edges for compositor gestures
unrelated to this app.

**State 1 — tab rail (hover, uncommitted):** dots grow into tabs, ~34px protruding × 64px
long, vertical text label (`Gtk.Label` angle 90°/270°), 4px gaps, same stacking order as the
dots (position maps 1:1, never reorders). Leaving without clicking collapses back to State 0
after ~350ms grace. Transition: 180ms ease-out, tabs stagger in 20–30ms apart.
- **Utility row below the tabs — 3 rectangular buttons, not circular:** new note (+),
  **search**, open-all-notes (grid icon). Same width as the tabs above them, small fixed
  height (~28px), same corner radius as the tabs — one consistent shape language for
  everything in this cluster, not circles mixed into a rectangle stack.
  - **Search transforms the panel in place, rather than opening anything separate.**
    Clicking it crossfades the tab list into a single-line search input occupying the same
    panel width, with live-filtered results (title + tag dot, same compact style as a tab
    label) appearing below it as text is typed. Escape or clicking away reverts back to the
    normal tab list. Selecting a result docks it exactly like clicking a tab does (State 2).
    This replaces a previously separate centered "quick search" overlay entirely — merged
    into one surface instead of two, on your explicit call. A global hotkey can still
    summon the strip directly into this search state (useful on multi-monitor setups where
    physically reaching the edge is a longer travel), it just no longer opens a distinct
    window to do it.

**State 2 — docked note (committed by click):** clicking a tab collapses the whole rail and
replaces it with that one note's full, editable card, sharing its buffer per §3.2's sync
rule. Background is the note's **tag color as a solid fill**, not the active app theme —
flagged as a deliberate divergence (**OPEN**: confirm vs. theme-following, one-line change
either way). Does **not** auto-collapse on pointer-leave — it's committed, not a hover
preview; close via click-outside, Escape, or an explicit control. Switching notes requires
closing first, then re-hovering — no direct tab-to-tab swap while one is docked (confirmed
as intended, not a limitation to fix).
- **New, from reference images:** a small pointer/caret on the docked card, pointing back
  toward roughly where its originating tab sat — a cheap visual-continuity cue so the card
  doesn't feel like it appeared from nowhere. Purely decorative, non-interactive.
- Optional non-blocking polish: a thin sliver of adjacent unopened tabs' colors peeking from
  behind the docked card's edges — skip for v1 if it adds friction.

**Repositioning along the edge (new):** recommended **customize-mode only, not always
draggable** — free dragging at any time conflicts with the hover-to-reveal model (the system
can't reliably distinguish "starting a drag" from "about to hover-reveal" without an awkward
delay on every interaction). A dedicated "Reposition" action (from Settings, or a long-press
on the resting dots) enters a visibly different, temporary drag state; drop or an explicit
"Done" exits it. **OPEN:** exact entry gesture and visual treatment of the customize state —
revisit once more reference material is available.

**Ordering:** "recent 4" = most recently *edited* (`updated_at desc`), not created. Pin-
priority ordering (pinned notes fill slots first, ahead of pure recency) is a still-open
suggestion, not yet adopted.

This is just another `Gtk.Window` under `gtk4-layer-shell`, driven by the normal GTK main
loop — no custom render loop needed.

### 3.5 Quick search — **retired as a standalone surface, merged into the edge strip**
Originally spec'd as a separate centered Rofi/Wofi-style overlay. On explicit direction,
merged into the edge strip's hover panel instead — see §3.4's "search" utility button. No
separate window, no separate summon-and-dismiss surface to build; one less component overall.
The keyboard-navigable results list and "Enter with no match creates a new note" behaviors
from the original design carry over unchanged, just rendered in the edge strip's panel space
rather than a centered overlay.

---

## 4. Icon inventory
24×24 viewBox, `currentColor` (no baked-in color, so Minimal/Paper share assets and only
Playful needs its own pass), round caps/joins.
- **Base set (Paper + Minimal), 1.75px stroke:** search, filter-sliders, settings-gear,
  chevron-left/right, close, pin/pin-filled, checklist, text-style ("Aa"), color-swatch,
  more-dots, pop-out-window, checkmark, archive, delete-trash, copy, plus-new-note,
  handle-grip, tag-dot.
- **Playful set, 2.25px stroke:** same geometry, bolder weight only — one deliberate change,
  not a redraw. `more-dots`/`pin-filled`/`tag-dot` (fill-based, not stroke) carry over
  unchanged.
- Retired: sticker/infinity icon (see §3.1), old separate palette icon (replaced by
  color-swatch), old plain sort icon (replaced by filter-sliders, confirmed via real
  screenshots as the actual control in use).

---

## 5. Control state matrix
| Element | Default | Hover | Active | Focus-visible | Disabled | Selected |
|---|---|---|---|---|---|---|
| Sidebar row | `bg-panel` | `bg-row-active` 60% | `bg-row-active` 100% | 2px inset `accent-focus` | n/a | `bg-row-active` + tag bar full opacity |
| Icon button | transparent, `text-secondary` glyph | hairline circle behind glyph, glyph→`text-primary` | scale(.94), bg −8% | 2px outline, 1px offset | 40% opacity | persistent tint if toggle-style (pin) |
| × close | `text-secondary` | `text-primary` + faint red bg | scale(.92) | 2px outline | n/a | n/a |
| Theme swatch | 1px hairline | border→`text-secondary` | scale(.97) | 2px outline, 2px offset | n/a | 2px `accent-focus` + checkmark badge |
| Segmented toggle | hairline, `text-secondary` | bg +4% | scale(.98) | 2px outline | 40% opacity | 1.5px `text-primary` border, bolder text |
| Search field | hairline | no hover reaction | n/a | 1.5px `accent-focus` border | greyed placeholder | n/a |
| Switch | hairline track, white thumb | subtle thumb shadow | thumb scale(1.05) while dragging | 2px outline around switch | 40% opacity | track fills `accent-focus` |

Global rules: hover transitions 100ms ease-out, press/active 60ms (always faster than hover),
focus rings always 2px full-opacity `accent-focus` (never reduced for aesthetics — a11y
tooling will flag it), disabled = 40% opacity with all hover/active transitions suppressed.

---

## 6. Motion principles
- One easing curve everywhere: ease-out-cubic.
- Hover 100ms / press 60ms / theme switch ~200ms crossfade / panel reveal 150–220ms.
- Staggered reveals (edge strip tabs, quick search results appearing) use 20–30ms delay
  between items — cheap, reads as deliberate polish rather than everything popping at once.
- Respect `Reduce motion`: gate custom Cairo/Snapshot animations on this setting explicitly —
  GTK's own built-in animations respect the system setting automatically, but hand-rolled
  ones won't unless checked.

---

## 7. History notes (why some things are the way they are)
- **Minimal used to be "Glass"** with real compositor blur. Dropped after: (a) blur can't be
  verified from inside the app, (b) it requires the user to hand-edit Hyprland config, (c) a
  real screenshot showed background content bleeding through where transparency existed but
  blur wasn't actually running — which looks broken, not stylish. Minimal keeps the same
  visual identity, just fully opaque.
- **The tool rail used to differ by theme** (different icons, different counts) — this was
  the actual root cause of "looks inconsistent," not a styling problem. Fixed by making the
  rail identical everywhere and pushing the one real per-theme difference (Playful's extra
  background-fill option) into a popover instead of the rail itself.
- **The edge strip used to expand all 4 notes into full preview cards at once** — replaced
  after seeing a clearer reference: dots at rest, tabs on hover, and only ever one note
  docked open at a time.
- **Quick search used to be a separate centered overlay window** — merged into the edge
  strip's own hover panel instead, on explicit direction, so the product has one persistent
  surface for "reach a note fast" rather than two.

---

## 8. Open items — explicit list
1. Structured checklists (real checkbox state) — planned, not built; approach noted in §3.1.
2. Final settings section list — draft only (§3.3).
3. Docked edge-strip card: tag-color background vs. theme-following — needs confirmation.
4. Edge-strip repositioning: exact entry gesture / visual treatment of customize mode.
5. Quick search: exact default hotkey binding.

---

## 9. Image-generation prompt

Honest caveat before using this: image generators are unreliable for **multi-element UI with
legible small text** — expect garbled labels, misaligned icons, and inconsistent spacing on
anything with more than a few words in it. This prompt is tuned to get a **mood/identity
reference** (palette, texture, shape language) rather than a pixel-accurate screen — treat
the result as a style anchor to react to, not a spec to trace. For anything you need text-
accurate, a real design tool (Figma) will serve you better than generation.

```
A clean UI design mood board for a minimalist desktop sticky-notes app, shown as four
distinct visual styles side by side on a soft neutral gray background:

1. "Paper" style: a warm cream-colored sticky note card with a hand-torn irregular bottom
edge, soft subtle drop shadow, rounded top corners, handwritten-style casual script text,
a thin colored vertical accent bar on the left edge in warm orange.

2. "Playful" style: a mint-green sticky note card with a bold black hand-drawn wavy outline
on all four sides (not straight lines — a gentle uneven wobble), a hard-edged flat drop
shadow offset to the bottom-right with no blur, handwritten-style text, a small filled pink
circular pin icon in the corner.

3. "Minimal Light" style: a crisp white rounded rectangle card, no texture, a single thin
blue horizontal accent line across the top edge only, clean sans-serif UI icons, generous
whitespace, understated soft shadow.

4. "Minimal Dark" style: same as Minimal Light but on a dark charcoal-slate background with
light gray text and the same thin blue top accent line.

Consistent across all four: small colored circular dots (blue, green, purple, orange)
representing category tags, simple thin-line icons (pin, checkmark, search, gear), rounded
sans-serif UI typography for chrome elements only. Overall aesthetic: modern indie desktop
software, warm and tactile rather than corporate, restrained use of color with one accent
per style rather than many competing colors. Flat vector illustration style, no photorealism,
no 3D rendering, clean and legible at a glance.
```

If you want per-component renders instead of one mood board (likely to generate better),
isolate each bracketed style block above into its own separate prompt — a generator handling
one card and one texture at a time will render far more coherently than one asked to hold
four different styles, several icons, and small text all in a single frame.
