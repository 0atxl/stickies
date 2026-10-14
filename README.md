# Stickies

Stickies is a fast, local scratchpad that lives on the edge of a Linux desktop. It is built for thoughts that are too small to justify opening a full notes application.

The interaction is simple. Recent or pinned notes sit at the screen edge. Hovering reveals title-only tabs, clicking a title opens the full note, and closing returns it to the edge.

## Project goals

- Native Rust and GTK4 interface with Wayland Layer Shell integration.
- Local SQLite storage, offline editing, and no telemetry. No account is required for local use.
- Fast mouse and keyboard access without a normal tiled application window.
- Plain-text notes and checklists with optional inline formatting, undo/redo and Copy text.
- Hyprland is the reference environment. KDE Plasma Wayland, Sway, and other Wayland compositors that support Layer Shell are planned Linux targets.

GNOME and X11 are outside the Linux release scope. Windows and macOS may receive separate ports later.

## Status

Stickies is in early development. All Notes now has a compact list, an editable note, and an embedded Appearance drawer with immediate changes. Main and floating editors share the same live note. Search, pinning, archive, restore, deletion with Undo, and Copy text are available. Paper, Playful, and Minimal Light/Dark are implemented. All Notes and individual notes await final visual approval; the edge-strip redesign remains unfinished.

Notes save locally after 750 ms of quiet time. Failed saves block closing or replacing an editor. Always open disables the default two-minute inactivity timeout; pinning controls note order. Checklist markers, bullets, numbered lists, and completed-line strike-through work in the plain-text editor.

The UI is awaiting manual design review and compositor checks. Minimal has fully opaque light and dark modes with no compositor blur dependency. Saved Glass preferences remain compatible. DND integration currently watches Omarchy shell state. Other desktop compatibility is unverified.
New databases start empty. Use `+` at the edge to create your first note. Existing databases retain their notes, including any examples created by earlier prototypes.

Font choices are Virgil, Inter, JetBrains Mono and Kalam, selected per note. New notes use the theme font and a plain canvas. Existing global font and canvas preferences migrate into saved note styles on startup so existing notes keep their appearance. No database schema change is needed.

Individual notes have a fixed, opaque footer capsule with Color, Text, Checklist, Always open and More. Separate notes also have a small drag grip in the top margin that uses native window movement. All Notes has Color, Text, Checklist, ordering Pin, Pop out and More. Color opens a 16-color grid in eight columns and two rows. Text opens four Aa samples in the actual fonts, with full-name tooltips, plus S/M/L. The whole-note Regular/Bold control is removed; existing saved bold styles retain their appearance. Color, font and canvas choices change the current note in either editor; they are absent from app-wide Settings. The old header tools launcher and inline tools panel are removed.

Individual More contains four canvas samples without a heading, bullet/numbered list icons, Copy and Delete. Tooltips name the samples and list icons. Docked notes also offer Open away from edge. Archive is available only in All Notes. Its More menu contains the same canvas samples, Copy, Archive/Restore and Delete. Both editors share menu colors, corners, spacing and hover/selected styling. Delete has a separator above it. Individual More opens toward the screen interior, from the same footer row. Footer popovers allow switching directly to another control with one click. Archived preview controls are disabled except Copy and Restore. Checklist toggles checklist mode on the current line; the embedded checkbox changes completion.

Selecting editable body text replaces the normal footer controls with Bold/Italic/Underline/Strikethrough in the same capsule. Its outer size and position stay fixed, with no transition or floating selection menu. Formatting clicks retain the selection; clearing selection or pressing Escape restores the normal controls. Archived notes never switch to formatting controls. Underline and strike strokes scale with the note font for readable handwriting decorations. Ctrl+B, Ctrl+I and Ctrl+U use the same formatting handler. Ctrl+Z undoes; Ctrl+Shift+Z and Ctrl+Y redo. Formatting remains separate from the plain body and survives reopening. Copy text is plain text. Search inside a note remains removed.

Title and body edits use the note's shared, session-local undo history. Continuous typing or deletion groups into one step, up to 100 Unicode characters. A pause of at least one second, a typed newline, changing between insertion and deletion, or moving the cursor/selection starts another step. Paste, cut, drop, formatting, list-prefix and checkbox changes, and list continuation each have their own step. Undo/redo restores text and formatting together and returns the cursor to the changed range. A new edit after undo clears redo.

Archive and Delete first save the shared note. A failed save retains its editor. Archive closes individual views without a toast. Delete closes them and shows one replaceable six-second "Note deleted" / Undo toast beside the current strip position, on the existing strip surface. The toast uses the docked-note Overlay/DND policy, accepts input only within its own bounds and fades over 120 ms unless motion is reduced. Undo restores the previous collection and pin state. All Notes keeps its existing deletion toast.

The edge defaults to overlapping title tabs and a 4 × 40 px bar marker. Settings also offers Classic tabs with a small gap and a Pills marker colored by the visible notes. Classic tabs pull 6 px toward the screen center without scaling or moving their neighbors. Their single background extends off screen to keep them attached without a shadow seam. Stacked retains the Dock-style hover wave. Settings starts with theme previews, followed by compact rows for Mode, Edge, Position, Visible notes, Tabs, Marker and Auto-close. Reposition remains under Position. Redundant Appearance/Edge strip/Behavior headings are removed. Settings has compact choice rows, a distinct Reposition button and the Reduce motion, Hover to reveal and Launch at login switches at the bottom. The note-count choice applies immediately and survives restart. The bar fades into the tabs and utilities on one shared reveal clock, with a short slide from the screen edge. In Stacked, a Dock-style hover wave enlarges the nearest title by up to 24%, with a smaller response in nearby titles. The stack makes room without losing its overlap, and every tab stays attached to the screen edge on Left, Right, Top and Bottom. Pointer tracking updates the destination without restarting the animation. There are no hover previews or tooltips. Clicking a title opens the docked editor; switching notes crossfades the sheet, moving it only when needed to keep its arrow aligned with the selected tab. New individual windows start at 320 × 376 px. Reduced motion makes reveal, collapse and hover changes immediate.

## Remaining work

1. Finalize the UI and finish phase 5A/5B desktop checks, including installed launch after login.
2. Add multi-monitor behavior: the monitor containing the cursor has the edge strip.
3. Add simple, optional-to-enable Google Drive sync. The integration is not chosen yet; offline editing and preservation of conflicting edits remain required.
4. Validate other Layer Shell compositors and distributions.
5. Package and release, starting with Arch/AUR and GitHub artifacts, with other Linux package formats following.

Dedicated Markdown support, file export, and application-level note encryption are outside the planned scope. Existing plain-text list controls and Copy text remain. Google Drive sync is planned, not currently available.

## Requirements

- A Wayland compositor with Layer Shell support. Hyprland is the reference environment.
- Rust and Cargo.
- GTK4, libadwaita 1.7 or newer, and gtk4-layer-shell development libraries.
- Fontconfig and `glib-compile-resources`. Bundled fonts and their licenses are in `assets/fonts`.
- SQLite development library.
- `pkg-config`.

Confirm the native libraries are available:

```bash
pkg-config --modversion gtk4
pkg-config --modversion libadwaita-1
pkg-config --modversion gtk4-layer-shell-0
pkg-config --modversion sqlite3
```

## Build and run

```bash
cargo build --locked
cargo run --locked
```

While Stickies is running, Hyprland should list a layer named `stickies`:

```bash
hyprctl layers
```

Press `Ctrl+C` to stop it.

## File-only visibility preferences

`$XDG_CONFIG_HOME/stickies/config.toml`, normally `~/.config/stickies/config.toml`, retains these keys without Settings controls. Both default to `true`:

```toml
hide_in_fullscreen = true
hide_in_dnd = true
```

`hide_in_fullscreen` places the strip and transient search panel on the Top layer, below fullscreen windows. Set it to `false` to use Overlay. Docked notes use Overlay in either case. `hide_in_dnd` hides the strip and transient panel while Omarchy Do Not Disturb is active, preserving an open docked note. Set it to `false` to keep the strip visible. Restart Stickies after editing the file.

Old `white_paper` entries are removed when a valid config loads. `tab_layout` accepts `classic` or `stacked`, defaulting to `stacked`; older `adjacent`/`overlapping` values also load. `marker_style` accepts `dots` or `bar`, defaulting to `bar`. The existing `dots` value now renders pills. Global `font` and `paper` keys are read once to preserve existing note styles, then removed from the file. Other preferences and stored note styles are preserved.

## Hyprland shortcuts and CLI actions

Stickies accepts one action flag per invocation:

| Flag and exported action | Behavior |
|---|---|
| `--new-note` / `new-note` | Create a note at the edge and focus its title. |
| `--search` / `search` | Open edge Search and focus it. If Search is already open, keep the current query. |
| `--toggle-strip` / `toggle-strip` | Reveal dormant tabs, or close edge content and collapse to the marker. Closing still requires a successful save. Reposition is cancelled when collapsing. |
| `--show-all-notes` / `show-all-notes` | Open All Notes or focus its existing window. |

When Stickies is running, these flags use [GApplication's command-line forwarding](https://docs.gtk.org/gio/method.Application.add_main_option.html) over the session D-Bus. The second process sends the request to the primary instance and exits. With no running instance, Stickies starts and performs the requested action. Running without a flag retains the normal startup behavior. `stickies --help` lists the flags; unknown flags or multiple different action flags return an error.

Global shortcuts belong to the compositor and are documented here rather than shown in Settings. For current Omarchy Lua configurations, choose unused bindings:

```lua
o.bind("SUPER + N", "New note", "stickies --new-note")
o.bind("SUPER + slash", "Search notes", "stickies --search")
o.bind("SUPER + T", "Toggle strip", "stickies --toggle-strip")
o.bind("SUPER + A", "All Notes", "stickies --show-all-notes")
```

Older Hyprland configurations use `bind = SUPER, N, exec, stickies --new-note` and the equivalent action commands above.

To hide Hyprland's border on pop-out notes while keeping the All Notes border:

```lua
o.window({ class = "^dev[.]stickies[.]Stickies$", title = "^Stickies · Note$" }, { border_size = 0 })
```

Add that rule to your user Hyprland Lua config, then run `hyprctl reload` and `hyprctl configerrors`.

The `stickies` executable must be on Hyprland's PATH. For a workspace build, substitute the absolute path to `target/debug/stickies` or `target/release/stickies`. Hyprland owns the global keybindings; Stickies does not capture global keys or edit compositor configuration.

## Verify the exported D-Bus actions

Start the current Stickies build, then use another terminal on the same session bus. List its actions:

```bash
gdbus call --session \
  --dest dev.stickies.Stickies \
  --object-path /dev/stickies/Stickies \
  --method org.gtk.Actions.List
```

The result must contain `new-note`, `search`, `toggle-strip` and `show-all-notes`; their order may vary. Call the actions through GIO's [exported action interface](https://github.com/GNOME/glib/blob/main/gio/gactiongroupexporter.c):

```bash
gdbus call --session \
  --dest dev.stickies.Stickies --object-path /dev/stickies/Stickies \
  --method org.gtk.Actions.Activate "'new-note'" '[]' '{}'

gdbus call --session \
  --dest dev.stickies.Stickies --object-path /dev/stickies/Stickies \
  --method org.gtk.Actions.Activate "'search'" '[]' '{}'

gdbus call --session \
  --dest dev.stickies.Stickies --object-path /dev/stickies/Stickies \
  --method org.gtk.Actions.Activate "'toggle-strip'" '[]' '{}'

gdbus call --session \
  --dest dev.stickies.Stickies --object-path /dev/stickies/Stickies \
  --method org.gtk.Actions.Activate "'show-all-notes'" '[]' '{}'
```

Each call returns `()`. Observe the corresponding behavior from the table above; invoke `toggle-strip` again to check both directions. The empty parameter array is required because these actions take no arguments. Repeat using the four CLI flags and confirm the same running process handles them without another persistent instance. Close Stickies and launch with each flag separately to check startup with an action. A failed save must retain the docked note when New note, Search or Toggle strip would close it.

## License

Stickies is licensed under the [Mozilla Public License 2.0](LICENSE).
