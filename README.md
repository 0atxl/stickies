# Stickies

Stickies is a fast, local scratchpad that lives on the edge of a Linux desktop. It is built for thoughts that are too small to justify opening a full notes application.

The interaction is simple. Recent or pinned notes sit at the screen edge. Hovering reveals title-only tabs, clicking a title opens the full note, and closing returns it to the edge.

## Project goals

- Native Rust and GTK4 interface with Wayland Layer Shell integration.
- Local SQLite storage, offline editing, and no telemetry. No account is required for local use.
- Fast mouse and keyboard access without a normal tiled application window.
- Plain-text notes and checklists, with Copy text for use elsewhere.
- Hyprland is the reference environment. KDE Plasma Wayland, Sway, and other Wayland compositors that support Layer Shell are planned Linux targets.

GNOME and X11 are outside the Linux release scope. Windows and macOS may receive separate ports later.

## Status

Stickies is in early development. All Notes now has a compact list, an editable note, and an embedded Appearance drawer with immediate changes. Main and floating editors share the same live note. Search, pinning, archive, restore, deletion with Undo, and Copy text are available. Paper, Playful, and Minimal Light/Dark are implemented. All Notes and individual notes await final visual approval; the edge-strip redesign remains unfinished.

Notes save locally after 750 ms of quiet time. Failed saves block closing or replacing an editor. Always open disables the default two-minute inactivity timeout; pinning controls note order. Checklist markers, bullets, numbered lists, and completed-line strike-through work in the plain-text editor.

The UI is awaiting manual design review and compositor checks. Minimal has fully opaque light and dark modes with no compositor blur dependency. Saved Glass preferences remain compatible. DND integration currently watches Omarchy shell state. Other desktop compatibility is unverified.
New databases start empty. Use `+` at the edge to create your first note. Existing databases retain their notes, including any examples created by earlier prototypes.

The handwritten font is Virgil, with Inter and JetBrains Mono as the other choices. Reference images guide layout and styling without replacing those font decisions.

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

## License

Stickies is licensed under the [Mozilla Public License 2.0](LICENSE).
