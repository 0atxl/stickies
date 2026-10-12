# Stickies: current product specification

This describes the current source, including uncommitted changes. Implementation references accompany each section. Runtime acceptance and remaining work belong in [current-milestone.md](current-milestone.md); earlier decisions and verification records belong in [HISTORY.md](HISTORY.md).

## Edge strip

Sources: [edge.rs](../src/edge.rs), [main.rs](../src/main.rs), [motion.rs](../src/motion.rs), [edge-strip.css](../assets/edge-strip.css).

The strip starts dormant. Its working set contains up to four active, non-deleted notes, using All Notes' current Recent/Title ordering. The initial sort is Recent; changing it applies to the strip for the running application. Sorting is not persisted.

| State | Visible content and behavior |
|---|---|
| `Dormant` | The marker shows one 3 × 28 px line with a larger invisible hit area. Clicking reveals tabs. Hover reveals them after 140 ms when enabled. |
| `Tabs` | Title tabs plus New note, Search and All Notes buttons. Pointer departure schedules collapse after 350 ms. Escape starts collapse without the leave grace. |
| `Search` | Tabs remain visible beside an input and active-note results, including notes beyond the four slots. Pointer departure and focus loss leave Search open. |
| `Docked(id)` | Tabs remain visible beside one editable note. Hover and pointer departure do not replace or collapse it. Selecting another tab saves and switches directly. |
| `Reposition` | Expanded strip with a temporary drag preview and Done/Cancel controls. Done persists the position; Cancel or Escape restores the saved position. |

- Closing Search or a docked note returns to Tabs; the normal leave grace can then collapse it. Search also closes through its button, opening a result, or Toggle strip.
- Search uses title/body matching and the active collection's sort. Up/Down selects a result and scrolls it into view while focus remains in the entry; Enter opens it. Refreshed results keep their selected row visible after allocation. With no matches and a nonblank query, Enter or the displayed create action makes a note whose title is the trimmed query. An empty query never creates a note.
- Opening a tab/result for a note already in a separate window focuses that window. It does not create a second individual editor for the same note.
- Left/Right use rotated labels on vertical tabs, 24 px deep and 80 px long. Top/Bottom use upright labels on horizontal tabs, 24 px deep with measured, capped widths. Stacked tabs are the only layout. They overlap by 8 px, retain a 16 px end allowance and magnify up to 1.24 with smaller neighboring magnification based on pointer proximity. The hovered tab draws in front, and overlapping clicks follow that drawing order. The utility capsule has contiguous 24 × 24 px buttons with 16 px icons and faint dividers between cells. Long titles truncate without a tooltip. Hover peek cards are not implemented yet.
- Title tabs and the utility capsule use compact shadows without perimeter rings. The selected tab has a subtle fill tint and a single free-edge accent; Minimal retains its note-color stripe on unselected tabs. Keyboard focus remains outlined. Titles retain their 11 px font and scale with the tab on hover.
- A decorative arrow joins Search/docked content to its origin. For a visible docked tab its origin follows that note's current position; overflow notes use the available origin or cluster center. It has no interaction role.
- Marker dragging in Dormant saves its along-edge position on release. Settings Reposition previews dragging without writing until Done. A failed Done keeps the preview and error for retry or cancellation. DND or closing All Notes cancels Reposition. Entering it first saves/closes any docked note; a failed save blocks entry.
- Reveal uses a 160 ms slide/fade with 20 ms tab stagger; collapse uses 110 ms. Search/docked panels and their arrow fade over 120 ms. Reopening the same Search panel during its outgoing fade reverses from its painted opacity, with the arrow sharing that opacity. Stacked tab hover scales up to 1.24 over 120 ms. Motion reverses from the painted progress; leave grace waits for reveal to finish.
- Custom motion uses opacity/transforms inside stable geometry. It respects Reduce motion and GTK animation settings; Reposition settles strip motion. Typing settles an unfinished Search panel fade. Input uses final tab bounds during reveal and retains them through collapse, with a 20 px allowance for stacked hover growth and a 7 px dormant bar hit allowance.
- Two Layer Shell windows use namespaces `stickies-strip` and `stickies`, reserve no desktop space (`exclusive_zone(-1)`), and keep a screen-sized extent across states. Transparent space outside their input regions passes pointer input through.
- Transient content uses Layer `Top` when `hide_in_fullscreen` is true and `Overlay` otherwise. A docked note uses `Overlay`. This is a layer policy; there is no explicit fullscreen-window detector.
- With `hide_in_dnd`, Omarchy DND hides the strip and transient panel while preserving an open docked note. Detection requires `OMARCHY_PATH` and reads `~/.local/state/omarchy/notifications.json`; missing/invalid state means false. Other desktops have no DND provider.
- Dormant surfaces take no keyboard focus. Explicit opening requests focus, then returns to on-demand focus after activation. Hover alone does not request exclusive focus.
- Geometry uses GDK monitor zero, with a 720 × 500 fallback. Neither surface selects a monitor explicitly. Cursor-following multi-monitor behavior is not implemented.

## Note windows and editing

Sources: [editor.rs](../src/editor.rs), [main.rs](../src/main.rs), [note_session.rs](../src/note_session.rs), [note_controls.rs](../src/note_controls.rs).

- At most one note is docked. Several different notes can have separate windows; reopening an existing separate note focuses it. Popping out a docked note transfers its view without removing its collection membership.
- Individual editors start at 352 × 376 px and request a 300 × 280 px minimum. Docked resizing is clamped to available screen space and uses the corner appropriate to the edge. Separate windows are undecorated ordinary GTK top-levels; the header supports native movement and the existing corner input requests native resizing. A subtle grip appears in that corner only while the pointer is over the note. They become resizable after their initial monitor-enter event. Floating placement is controlled by the compositor.
- Title and wrapped, scrollable body are editable. The header contains only title and Close. The reserved 44 px footer contains Color, Text, Checklist, Always open and More. It stays quiet until hovered or focused. Color opens only the 16-color grid; Text opens only S/M/L and Regular/Bold. Individual More is a flat menu containing Bullet list, Numbered list, inline Default/Plain/Dots/Grid/Ruled pattern choices, Copy text, Archive, Delete and, for docked notes, Open away from edge. The old tool popover and header launcher are removed.
- The individual pin glyph means **Always open**: a per-view, non-persistent inactivity override. It does not change note ordering or request compositor pinning/always-on-top. Close remains available while enabled.
- Only docked editors have an inactivity timer. Off disables it; otherwise the selected 1/2/5-minute interval starts when opened. Keys, clicks, scroll, pointer-enter, pointer motion and note-control changes count as activity. A stationary pointer alone does not repeatedly reset it.
- Always open cancels the timer; disabling it starts a fresh interval. Changing the timeout restarts/cancels an existing docked timer immediately. Separate windows and All Notes have no inactivity timeout.
- In an individual editor, Escape dismisses footer popovers or the selection bubble first. A subsequent Escape closes the note. With no popover open, Escape closes it immediately. Close still observes save/removal guards.
- Checklist lines beginning `- [ ] ` or `- [x] ` display native check buttons in active editors. Toggling updates the plain-text marker; completed item text uses the font's Pango strike metrics and muted theme ink. Archived previews also show checkboxes, without allowing changes. Anchors/widgets are presentation only and are not stored.
- The Checklist footer toggle adds/removes checklist mode on the current line. Embedded checkboxes toggle completion. Bullet and numbering controls toggle the current line's prefix. Numbering removes any leading `u32` prefix followed by `. `, including continued items; otherwise it inserts `1. `. Enter without Shift continues a recognized list, using an unchecked next checklist item and incrementing numbers. An empty list item ends the list. Backspace at the checklist prefix removes it. These controls are not a general Markdown renderer.
- Active views of the same note share one title buffer, body buffer, style, color, pin state and save owner. A change in All Notes or an individual window is immediately available to the other live view.

- Both body editors have no editing bar or Find in note. A nonmodal B/I/U/S bubble appears only for selected editable body text. It opens below the selection when placement above would reach the title. It follows replacement buffers and hides with the view or when a footer popup opens. Ctrl+B/I/U use the same formatting handler. Ctrl+Z undoes; Ctrl+Shift+Z and Ctrl+Y redo. Ctrl+F has no note-search action. Copy text remains plain text.
- Shared sessions keep up to 100 custom text/formatting history states, including title edits. Both title and body shortcuts use that shared stack; GTK's own undo is disabled for these editing buffers. History is session-local and excludes presentation anchors. Continuous, adjacent insertion or deletion groups up to 100 Unicode characters. A pause of at least one second, a typed newline, a change of edit direction or field, or cursor/selection movement ends the group. Clipboard/drop operations, formatting marks, list-prefix and checkbox toggles and list continuation are isolated steps. The grouping clock is injectable; production uses monotonic elapsed time. Undo restores text and formatting together and places the cursor at the changed range. A new edit after undo clears redo. List-prefix changes preserve formatting on the item text. A formatting-load failure blocks saving that session.
- Hover labels are concise. Redundant checkbox, date and resize hints are omitted; screen-reader labels remain on icon actions. Compositor shortcut examples live in README, not Settings.

## All Notes

Sources: [manager.rs](../src/manager.rs), [main.rs](../src/main.rs), [storage.rs](../src/storage.rs), [all-notes.css](../assets/all-notes.css).

- One reusable ordinary application window, default 720 × 540 px, minimum request 680 × 520 px, with a 240 px list column. Opening an existing window presents it with its current state.
- A newly created All Notes window starts in All with an empty search and **no selected note or preview**, even when notes exist. Its preview says “Choose a note.” Clicking or keyboard-selecting a row opens it. Refresh retains a chosen note only while it remains in the results; it does not select a replacement automatically. Explicit creation and deletion Undo select their resulting note.
- All includes active and archived notes; Active excludes archived; Archived shows only archived. Every collection excludes deleted notes. Search matches title/body within the selected collection.
- Recent orders All/Active by pinned first, updated time descending, stored `sort_order` ascending, then ID descending. Title orders them by pinned first, title with SQLite `NOCASE`, then updated time/ID descending. Archived omits pinned priority. The sort choice survives closing/reopening All Notes within the same process, but resets to Recent at application restart.
- Active selections edit the shared session directly. Archived selections are read-only snapshots with completed-line styling; they must be restored before editing, pinning or popping out.
- The reserved preview footer contains Color, Text, Checklist, ordering Pin, Pop out and More. Color/Text have the same isolated popovers as individual notes. More has only Copy text, Archive/Restore and Delete. The large inline tools panel and its compact/wide breakpoint are removed. Archived selections disable every control except Copy and Restore. Copy text copies the current body, including checklist markers.
- Ordering Pin persists on the note and affects All/Active and the edge. It is distinct from an individual editor's Always open toggle.
- Archive preserves the note and removes it from Active/the edge. Restore returns it to Active. Delete is soft deletion; a six-second Undo toast restores its previous collection and pin state. Archive/Delete close matching individual views through the coordinator after the initiating save guard.
- Individual Archive/Delete use the same collection mutation path and shared save guards. Archive closes the note without feedback. Delete closes it and shows one six-second Undo toast on the existing strip surface. The toast is clamped beside the current strip, has only "Note deleted" and Undo, replaces any earlier toast, and accepts input only within its bounds on that surface. It uses Overlay and survives DND like a docked note. The existing motion helper provides a 120 ms fade, skipped for reduced motion. Undo uses the existing storage restore operation. All Notes retains its own toast.
- A selected active note has saving/saved/error feedback. Individual notes show save-error toasts without a routine saved indicator. Empty collections/searches offer Create a note, Back to notes or Clear search as appropriate.
- Closing All Notes releases its editor and leaves other note windows available. Failed text/color saves or blank-note removal block closing/replacing that editor.

## Settings

Sources: [settings.rs](../src/settings.rs), [manager.rs](../src/manager.rs), [main.rs](../src/main.rs), [autostart.rs](../src/autostart.rs).

The gear toggles one scrollable Settings drawer inside All Notes. Its width is 260 px; an explicit breakpoint overlays it, with the native scrim covering both filters and notes, at widths up to 979 px. At wider widths it occupies sidebar space.

| Section | Implemented controls |
|---|---|
| Appearance | Minimal/Paper/Playful; System/Light/Dark when Minimal is selected; Virgil/Inter/Mono/Kalam plus Reset to theme font; visual Plain/Dots/Grid/Ruled canvas samples; Reduce motion. |
| Edge strip | Left/Right/Top/Bottom; Top/Center/Bottom on vertical edges or Left/Center/Right on horizontal edges; Reposition. Presets save offsets 0/0.5/1. |
| Behavior | Docked note timeout: Off / 1 / 2 / 5 minutes; hover reveal; launch at login. Fullscreen/DND policies are config-file-only. |

- Changes validate and save config before replacing live application config, then refresh open views. Settings changes are blocked during Reposition and after an invalid/unreadable startup config.
- Errors are displayed. Position, timeout and Appearance controls resynchronize to the accepted live config after rejected writes. Appearance synchronization restores theme, Minimal mode visibility, effective font, pattern, Reduce motion and samples under a signal guard, without submitting another save.
- App Reduce motion forces GTK animations off. Turning it off releases the application override so GTK follows the current system preference. Unrelated settings writes leave that preference alone; rejected writes do not change it.
- There is no Settings control for true strip enablement, default new-note color, shape or visible-note count. There is no About section. Corresponding stored keys below remain available where implemented.
- `launch_at_login` reconciles `$XDG_CONFIG_HOME/autostart/dev.stickies.Stickies.desktop` on valid startup. True installs the supplied entry (`Exec=stickies`); false removes it. This does not establish that an installed login launch has been verified.

## Themes and note appearance

Sources: [appearance.rs](../src/appearance.rs), [app.rs](../src/app.rs), [config.rs](../src/config.rs), [ui_assets.rs](../src/ui_assets.rs), [all-notes.css](../assets/all-notes.css).

| Theme | Note surface and theme-default content font |
|---|---|
| Paper (default) | Opaque warm paper, colored left accent, soft shadow; default Ticket/Torn treatment tears the bottom edge. Virgil. |
| Minimal Light | Opaque white, colored top accent, smooth rounded rectangle. Inter. |
| Minimal Dark | Opaque dark slate, colored top accent, smooth rounded rectangle. Inter. |
| Playful | Opaque fill from the note's color, wavy edges, dark outline and offset shadow. Virgil. |

- Minimal's System mode reads GTK's prefer-dark flag/theme name. Light/Dark override it. Paper and Playful do not gain a separate dark appearance from this preference.
- Global font options are Theme, Handwritten (Virgil), Clean (`grotesk`: Inter), Mono (JetBrains Mono), and Kalam. Per-note size and weight apply to both title and body; font family is global only.
- S/M/L body sizes are 14/17/20 px in every theme. Titles are 27/30/33 px in Paper/Playful and 25/28/31 px in Minimal. Regular/Bold use weights 400/700. New note styles default to Medium/Regular.
- Plain/Dots/Grid/Ruled patterns work in all themes. A per-note pattern override wins over the global default; Default pattern follows Settings. Optional stored shape overrides also resolve over the global shape, but there is no shape picker. Minimal always draws the smooth rectangle; Paper Square is smooth and Ticket/Torn currently share the bottom-tear path; Playful draws waves on all edges.
- The shared 16-color palette is stored as `yellow`, `blue`, `purple`, `green`, `orange`, `red`, `pink`, `lime`, `teal`, `cyan`, `indigo`, `magenta`, `brown`, `gray`, `black`, `white`. The original four appear as Amber, Blue, Lilac and Sage. Color supplies list/edge markers, Paper/Minimal accents and Playful fill; there is no separate Playful fill preference.
- Bundled Virgil, Inter, JetBrains Mono and Kalam Regular/Bold TTF files are cached under `$XDG_CACHE_HOME/stickies/fonts` and registered with Fontconfig. An individual font failure logs a warning and registration continues; resource-registration failure can still stop UI initialization.

## Configuration

Source: [config.rs](../src/config.rs); consumers are the modules above. Path: GLib's user config directory plus `stickies/config.toml` (normally `$XDG_CONFIG_HOME/stickies/config.toml`). Enum strings use lowercase.

| Key | Default | Accepted values / current effect |
|---|---|---|
| `edge` | `right` | `left`, `right`, `top`, `bottom`; strip anchor/orientation. |
| `marker_offset` | `0.5` | Finite 0–1; along-edge position, with placement clamping. |
| `edge_hover_enabled` | `true` | Boolean; permits hover reveal. False still permits click/action reveal. |
| `launch_at_login` | `false` | Boolean; reconciles the autostart desktop entry. |
| `theme` | `paper` | `minimal`, `paper`, `playful`; legacy `glass` loads as Minimal. |
| `appearance` | `system` | `system`, `light`, `dark`; controls Minimal's mode. |
| `font` | `theme` | `theme`, `grotesk`, `mono`, `handwritten`, `kalam`; global content font. |
| `paper` | `plain` | `plain`, `dots`, `grid`, `ruled`; default canvas pattern. |
| `shape` | `ticket` | `square`, `ticket`, `torn`; default shape subject to theme drawing rules. |
| `visible_notes` | `4` | Accepts 4 or 5; rendering/loading is currently fixed at four. |
| `inactivity_seconds` | `120` | Saves accept 0, 60, 120, 300; docked timeout only, 0 means Off. |
| `reduced_motion` | `false` | Boolean; suppresses application animations alongside system settings. |
| `hide_in_fullscreen` | `true` | File-only Boolean; transient Top versus Overlay layer policy. |
| `hide_in_dnd` | `true` | File-only Boolean; Omarchy DND visibility policy. |
| legacy `font_size` | absent | Optional unsigned integer accepted on input, ignored, omitted on save. |

- Missing config creates/saves defaults. Missing keys in an existing file use these defaults. Unknown keys, invalid enums/types and invalid validated values are rejected.
- On load, legacy custom timeouts in 30–1800 seconds other than 60/120/300 normalize to 120 in memory. Loading alone does not rewrite an existing file unless it contains retired choices. Other unsupported durations reject the config.
- Valid configs containing retired `white_paper`, `tab_layout` or `marker_style` entries are saved atomically without those entries. Other preferences retain their accepted values.
- Invalid/unreadable startup config logs an error and uses defaults for that run, preserves the existing file, blocks config writes and skips autostart reconciliation.
- Saving writes `config.toml.tmp`, syncs the file and renames it over the destination. The code does not sync the parent directory.
- Per-note style is TOML in SQLite, with `text_size=medium`, `text_weight=regular`, optional `paper` and `shape` absent by default. Size accepts small/medium/large; weight regular/bold; pattern/shape use the enum values above. Missing fields default; unknown fields reject. Legacy per-note `font` is read, ignored and omitted on save.

## Actions and shortcuts

Sources: [cli.rs](../src/cli.rs), [main.rs](../src/main.rs), [settings.rs](../src/settings.rs).

GApplication ID/session-bus name is `dev.stickies.Stickies`. Four parameterless `gio::SimpleAction`s are exported through `org.gtk.Actions` at `/dev/stickies/Stickies`.

| Action / CLI flag | Behavior | Settings example |
|---|---|---|
| `new-note` / `--new-note` | Create a blank Yellow note, dock it and focus its title. | `bind = SUPER, N, exec, stickies --new-note` |
| `search` / `--search` | Open/focus edge Search. If already open, keep its query. | `bind = SUPER, slash, exec, stickies --search` |
| `toggle-strip` / `--toggle-strip` | Reveal/focus a dormant strip; otherwise cancel Reposition, save/close docked content, close Search and collapse. | `bind = SUPER, T, exec, stickies --toggle-strip` |
| `show-all-notes` / `--show-all-notes` | Open or present the single All Notes window. | `bind = SUPER, A, exec, stickies --show-all-notes` |

- With a primary instance running, GApplication forwards the invocation over D-Bus and the secondary exits. Without one, it initializes the UI and then performs the requested action. Plain launch initializes a dormant strip; plain repeated launch does nothing when application windows already exist.
- At most one distinct action flag is accepted per invocation; repeating the same flag is accepted. Unknown flags and combined distinct actions fail. Actions that would replace a docked editor retain it on a failed text/color save.
- UI assets initialize in the primary activation path. No in-app global key capture, automatic Hyprland config editing or conflict detection exists. Bind lines are examples to copy and adapt; they do not reserve keys by themselves.
- Local keys include individual-note layered Escape, edge-state Escape, Search Up/Down/Enter and list-editing Enter/Backspace as described above. There are no registered global accelerators.

To inspect and activate the exported actions while Stickies runs:

```sh
gdbus call --session --dest dev.stickies.Stickies \
  --object-path /dev/stickies/Stickies --method org.gtk.Actions.List
gdbus call --session --dest dev.stickies.Stickies \
  --object-path /dev/stickies/Stickies --method org.gtk.Actions.Activate \
  new-note '[]' '{}'
```

Replace `new-note` with another action name. The empty parameter array matches their parameterless registration. These commands document the exported interface; this documentation pass does not constitute a runtime check.

## Data and saving

Sources: [storage.rs](../src/storage.rs), [note_session.rs](../src/note_session.rs), [main.rs](../src/main.rs), [migrations](../migrations).

- Notes are local SQLite data at GLib's user data directory plus `stickies/notes.db` (normally `$XDG_DATA_HOME/stickies/notes.db`). Database opening creates the parent directory and applies transactional migrations through schema version 4; a newer schema is rejected.
- Stored note fields are ID, title, body, color, pinned, `sort_order`, created/updated timestamps and nullable archived/deleted timestamps. Styles live in a separate `note_styles` table as TOML. No example notes are inserted at startup.
- Both creation paths insert a blank-body Yellow note immediately. Search creation can supply a title. Text/color edits save after 750 ms of quiet time through one shared session; further edits reset the debounce. Per-note style and ordering-pin changes write immediately.
- Title/body/color saves update the timestamp and refresh ordering/edge data. Failed text/color saves retain dirty buffers and display an error. Close, switch, dock replacement and initiating archive/delete operations flush first and stop if that flush fails. Failed style/pin writes retain the previous preference and report the error.
- An active note marked newly created and still whitespace-only in both title and body is soft-deleted when its last editing view leaves. Other shared views keep it alive. Archived notes, existing blank notes, title-only notes and search-created nonblank titles remain. Save/cleanup failures retain the view for retry.
- Checklists persist as textual markers. No child-anchor objects are serialized. Schema 4 adds `notes.formatting`, a JSON array of bold/italic/underline/strike spans using Unicode character offsets in the plain body. Body and formatting save in one SQL update. SQL search uses literal `instr` matching with GLib Unicode case folding of title, body and query through a connection-local SQLite function. Folding includes case expansions such as `Straße`/`STRASSE`; it preserves accents and adds no normalization. Ordering still uses SQLite `NOCASE`.
- Storage uses a synchronous connection on the UI thread. The application does not configure WAL or a busy timeout and has no sync transport, accounts or application-level encryption.
- SIGTERM, SIGHUP and SIGINT use GLib Unix signal sources to request normal application shutdown. Shutdown flushes every active shared note session before releasing its views, logs each failed flush with the note ID and error to stderr, and exits even after a failed write. GTK session registration is enabled; session-end notifications that GTK receives use the same shutdown path. SIGKILL, crashes and session termination without a delivered notification or catchable signal cannot flush pending text.

## Statements not confirmed from code

Unconfirmed product statements: none. Each behavior above is derived from the referenced source. Source inspection establishes implementation, not compositor behavior, performance budgets, installed login success, packaging completeness or test outcomes. Those acceptance claims are intentionally absent.

## Docs that were wrong

These are disagreements with the current implementation, including historical claims that were true for an earlier checkout and drafts that were never built. They are preserved in place as references, not treated as current requirements.

| Earlier document / statement | Current code |
|---|---|
| [UI plan](ui-implementation-plan.md), preserved behavior: an open docked note hides tabs; [October 1 reference](design-references/2026-10-01/stickies-uiux-spec.md) §3.4: docking collapses the rail and switching requires closing first. | Tabs stay visible in Docked; clicking another saves and switches directly. `edge.rs`, `main.rs`. |
| October 1 reference §3.4: Search crossfades/replaces the tab list, and click-away dismisses Search/docked content. Earlier [history](HISTORY.md) edge review also expects Search click-away dismissal. | Search is beside retained tabs. Focus loss/click-away/pointer departure have no dismissal handler; explicit controls/actions/Escape dismiss content. `edge.rs`. |
| October 1 reference §3.4: docked card uses tag-color fill regardless of theme. | Docked and separate notes use the same theme renderer; only Playful uses tag-color fill. `appearance.rs`. |
| [Roadmap](implementation.md) current UI scope and early history: four-note vertical strip on every edge; reference tabs protrude about 34 px and are 64 px long. | Left/Right are vertical 24 × 80 px; Top/Bottom are horizontal and measured. `edge.rs`. |
| October 1 reference §3.4: fixed four resting dots and rectangular utility stack. | Only the bar marker remains; utility styling is an opaque capsule. `edge.rs`, `edge-strip.css`. |
| October 1 reference §3.4: content-sized layer surface; Reposition recommended only in customize mode, with drop potentially committing. | Two screen-sized surfaces; Dormant marker drag also works. Reposition requires Done to commit and Cancel/Escape discards preview. `edge.rs`. |
| UI plan edge pass and October 7 history: remove all edge motion; [October 8 audit](audit-2026-10-08.md) P0-1: reveal/collapse are hard cuts. | Custom reveal/collapse/panel/hover motion exists, gated by reduced/system motion settings. `edge.rs`, `motion.rs`. |
| October 8 audit P0-3 and its module discussion: edge state is spread across booleans in `main.rs`. | `edge.rs` owns the five-state enum and transition effects; motion has its own module. |
| Older history/reference behavior: all individual windows time out, with keys/clicks/scroll as activity; October 8 audit P1-1 reports this. | Docked-only timeout; pointer-enter/motion count; Off/1/2/5-minute Settings choice. `editor.rs`, `settings.rs`. |
| Older history's focused-panel Escape verification and October 8 audit P1-2: Escape in a tool popover closes the note. | First Escape closes tools; next closes the individual note. `editor.rs`. |
| October 1 reference §3.1: pin orders/keeps a window on top via `hyprctl`, and Close no-ops/prompts while pinned. | Individual pin is temporary Always open; All Notes pin is persisted ordering. No `hyprctl` pin dispatch; Close remains enabled. `editor.rs`, `manager.rs`. |
| October 1 reference §3.1 retains a 40 px full-height rail/card-growth rule despite its floating-popover correction. | Individual tools use a reserved 44 px footer with separate Color/Text popovers and flat More; opening them never expands the note. `editor.rs`, `note_controls.rs`. |
| October 1 reference typography: handwritten content in every theme, 20 px title/18 px body. UI plan still mentions provisional 18/21/24 px bodies. | Theme-default Minimal uses Inter; bodies are 14/17/20 px, titles depend on theme. `appearance.rs`. |
| October 1 reference §3.1: checklist markers only imitate checkboxes, interactive state is future work. | Active views embed clickable GTK check buttons and persist marker toggles. Completed styling now follows the selected font metrics and theme ink. `note_session.rs`. |
| October 1 reference §3.2: All Notes is 1100 × 620, with 292 px list/320 px Settings, and uses ListView/SingleSelection. | 720 × 540, 240 px list, 260 px drawer; Gtk.ListBox with explicit selection handling. `manager.rs`. |
| October 1 reference §3.2: selected-row/editor switching crossfades. | The preview footer is Color/Text/Checklist/Pin/Pop out/More. Rows change CSS state; note pages do not configure a selection crossfade. `manager.rs`, `note_controls.rs`, `all-notes.css`. |
| October 1 reference §3.3, roadmap and older history: current Settings is Appearance only or not started. | Settings has Appearance, Edge strip positioning, Behavior controls in the existing drawer. Stacked tabs and the bar marker are unconditional. `settings.rs`, `manager.rs`. |
| October 1 reference draft Settings: enable strip, login switch, default color and About; key-combo badges imply ready shortcuts. | Launch at login is now exposed. Strip enablement, default color and About remain absent; compositor commands are documented in README. A draft is not a shipped control. `settings.rs`. |
| October 8 audit P1-3 and older history: only `connect_activate`, no actions/flags to reach a running instance. | Four SimpleActions plus parsed forwarding CLI flags. `cli.rs`, `main.rs`. |
| [CONTEXT.md](CONTEXT.md), Close: closing never changes membership/deletes a note. | Exception: a new note still blank when its last editing view leaves is soft-deleted. `note_session.rs`. |
| Roadmap product scope: monitor follows cursor; sync is a required remaining feature. | Geometry reads monitor zero; no cursor-following or Drive transport exists. These are future scope, not current product behavior. `edge.rs`, application/storage modules. |
| October 8 audit P2: bundled font failure is fatal and Virgil is WOFF2. | Five TTF registrations are attempted independently; font failures warn. `ui_assets.rs`, resource manifest. |
| October 8 audit P2: icon helpers lack accessible labels and Always open uses a hand-declared unsafe accessibility update. | Helpers and Always open use the safe GTK accessible-property API. `ui_assets.rs`, `editor.rs`. |
| [Code audit](code-audit.md) finding 1 describes strong More/color callback captures in the individual editor. | Inspected editor menu callbacks now use weak references. This source description is stale; a fresh lifetime probe is still open before declaring the finding resolved. `editor.rs`. |
| October 8 audit says font licenses are bundled without qualification. | The resource manifest embeds Virgil's notice; Inter/JetBrains Mono notices exist in the repo but are not embedded there. Package notice completeness remains open. `assets/stickies.gresource.xml`. |
