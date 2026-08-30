# Stickies

Stickies is a fast, local scratchpad that lives on the edge of a Linux desktop. It is built for thoughts that are too small to justify opening a full notes application.

The planned interaction is simple. Recent or pinned notes sit as thin indicators at the screen edge. Hovering fans them out, clicking opens a note, and edits save automatically. Closing the note returns it to the edge.

## Project goals

- Native Rust and GTK4 interface with Wayland Layer Shell integration.
- Local SQLite storage with no account, backend, or telemetry.
- Fast mouse and keyboard access without a normal tiled application window.
- Markdown and plain-text export so notes remain portable.
- Hyprland support first, followed by other Wayland compositors that support Layer Shell.

GNOME and X11 support are outside the initial release plan.

## Status

Stickies is in early development. The current prototype creates a small right-edge Layer Shell marker on Hyprland. Note cards, editing, and persistence are not implemented yet.

Planned work includes the edge interaction, application state, SQLite persistence, an all-notes window, desktop integration, multi-monitor behavior, Markdown export, and packaging.

## Requirements

- A Wayland compositor with Layer Shell support. Hyprland is the current target.
- Rust and Cargo.
- GTK4 and gtk4-layer-shell development libraries.
- `pkg-config`.

Confirm the native libraries are available:

```bash
pkg-config --modversion gtk4
pkg-config --modversion gtk4-layer-shell-0
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
