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

Font choices are Virgil, Inter, JetBrains Mono and Kalam. Existing theme fonts remain the defaults.

All Notes tools reserve space beside the sheet, or below it in narrow windows. The edge always uses stacked tabs and a bar marker. Title tooltips are removed; the planned hover preview is not implemented yet.

Ctrl+Z and Ctrl+Shift+Z undo and redo while editing the body. Existing bold, italic, underline and strikethrough ranges remain stored separately from the plain text and survive reopening. Their UI and shortcuts are temporarily unavailable pending the selection bubble. Search inside a note is removed. Copy text remains plain text.

The individual nested More menu is temporarily removed. Bullet list, Numbered list, Pattern override, Copy text and docked Open away from edge are unavailable there until the flat menus arrive. Their underlying editing and pop-out handlers and stored per-note styles remain. All Notes tools and actions are unchanged apart from removal of the editing/search bar.

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

Old `white_paper`, `tab_layout` and `marker_style` entries are removed when a valid config loads. Other preferences and stored note styles are preserved.

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
