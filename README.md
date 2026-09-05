# Stickies

Stickies is a fast, local scratchpad that lives on the edge of a Linux desktop. It is built for thoughts that are too small to justify opening a full notes application.

The planned interaction is simple. Recent or pinned notes sit at the screen edge. Hovering reveals title-only tabs, clicking a title opens the full note, and closing returns it to the edge.

## Project goals

- Native Rust and GTK4 interface with Wayland Layer Shell integration.
- Local SQLite storage with no account, backend, or telemetry.
- Fast mouse and keyboard access without a normal tiled application window.
- Markdown and plain-text export so notes remain portable.
- Hyprland is the reference environment. KDE Plasma Wayland, Sway, and other Wayland compositors that support Layer Shell are planned Linux targets.

GNOME and X11 are outside the Linux release scope. Windows and macOS may receive separate ports later.

## Status

Stickies is in early development. The current prototype has a right-edge Layer Shell marker, a five-note edge deck, editable notes, session-only Keep open behavior, and local SQLite persistence. Users can create notes, edit titles and bodies, archive notes, and soft delete them with confirmation. A plain All Notes window searches active and archived notes, sorts them by recent edit or title, and provides pin, archive, restore, and delete controls. Edits save after 750 ms of quiet time and load again when the app restarts. The application is single-instance and has an initial XDG configuration, desktop entry, opt-in launch-at-login backend, and Linux CI.

New databases start empty. Use `+` at the edge to create your first note. Existing databases retain their notes, including any examples created by earlier prototypes.

Planned work includes the final UI, the remaining desktop behavior, multi-monitor support, Markdown export, and packaging.

## Requirements

- A Wayland compositor with Layer Shell support. Hyprland is the reference environment.
- Rust and Cargo.
- GTK4 and gtk4-layer-shell development libraries.
- SQLite development library.
- `pkg-config`.

Confirm the native libraries are available:

```bash
pkg-config --modversion gtk4
pkg-config --modversion gtk4-layer-shell-0
pkg-config --modversion sqlite3
```

## Build and run

```bash
cargo build --locked
cargo run --locked
```

While the prototype is running, Hyprland should list a layer named `stickies`:

```bash
hyprctl layers
```

Press `Ctrl+C` to stop it.

## License

Stickies is licensed under the [Mozilla Public License 2.0](LICENSE).
