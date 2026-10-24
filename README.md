# Stickies

Stickies is a local scratchpad for Windows and Linux Wayland desktops with Layer Shell v4+ and compatible native libraries. It lives at the screen edge for thoughts that are too small to justify opening a full notes application.

The interaction is simple. Recent or pinned notes sit at the screen edge. Hovering reveals title-only tabs, clicking a title opens the full note, and closing returns it to the edge.

## Project goals

- Native Rust and GTK4 interface with Wayland Layer Shell integration.
- Local SQLite storage, offline editing, and no telemetry. No account is required for local use.
- Fast mouse and keyboard access without a normal tiled application window.
- Plain-text notes and checklists with optional inline formatting, undo/redo and Copy text.
- Target Wayland broadly. Hyprland is the development reference; scoped isolated Sway checks and owner desktop confirmations are recorded separately. KDE Plasma Wayland and other Layer Shell desktops are intended targets whose support must be verified. These results do not certify the final build across desktops.

The edge implementation requires Wayland with `zwlr_layer_shell_v1` version 4 or newer for on-demand keyboard focus. Startup checks support before creating the edge windows and prints a diagnostic if it is unavailable or too old. GNOME Wayland without Layer Shell cannot run that edge implementation; no fallback is implemented. This is a protocol limitation, not a Hyprland-only product scope. X11 and macOS are outside this release. The separate Windows port and its installation-free ZIP are described in [Windows packaging](packaging/windows.md). The owner verified the CI Windows bundle with disposable data and without development tools on PATH; broader platform limits remain documented.

## Status

The owner accepts the current UI/UX. UI design is complete; work now focuses on reliability, Wayland compatibility and finishing the app. Recorded scoped checks passed editing/saving, history, representative input, timeout, catchable shutdown and Search. The owner confirmed resolution recovery and accepted fixed-size Search without no-match jitter. These results do not establish broad compositor compatibility; remaining gaps are tracked in the verification record below.

All Notes and individual notes share live editing, per-note fonts/canvas/color, plain-text lists, separate formatting metadata and title/body undo history. Paper, Playful and Minimal Light/Dark are implemented. The fixed footer changes to B/I/U/S during editable body selection; there is no Find in note or hover preview. Fonts are Virgil, Inter, JetBrains Mono and Kalam. Existing font/canvas preferences migrate into note styles to preserve saved appearance.

New configurations default to Playful with Clean edges. Existing saved theme and edge choices are preserved. MCP access defaults to Off. The optional `stickies-mcp` companion lets a local assistant find, read, create and update notes when access is enabled in Settings. See the [MCP setup and write contract](packaging/README.md#model-context-protocol-mcp-companion). Focused Linux checks passed; Windows MCP compilation and runtime remain unverified.

Notes save after 750 ms of quiet time. Failed saves retain the editor. Only docked notes time out, with Off/1/2/5-minute choices and an Always open override; All Notes pin controls ordering. Blank new drafts stay out of lists/search until their first successful nonblank save. Existing blank notes remain. New All Notes windows start on Active without a selected note. Archive/restore, soft deletion with Undo and plain Copy text remain available.

The strip supports all four edges, four/five visible notes, Classic/Stacked tabs and Pills/Bar markers. Current design, dimensions, menus and motion are described in the local specification. They are settled, not an invitation to add controls or redesign them.

The single strip retains its selected monitor while available. Both layer windows use that monitor's logical coordinates; removal selects the first valid remaining GDK monitor. With no outputs, the windows stay hidden until an output returns. The owner confirmed resolution placement and accepted fixed-size Search without no-match jitter. Temporary diagnostics have been removed. Multi-monitor behavior remains unverified. Existing layout minimums remain: a docked note needs 300 × 280 logical pixels plus its adjacent strip and gap. Smaller outputs can leave controls outside the visible area. Separate note placement remains compositor-owned.

## Remaining work

Remaining verification includes complete checklist caret/wrap and emoji keyboard coverage, accessibility, fractional/high scaling, installed login/session-end, multi-monitor recovery and resource/lifetime observations. The owner's resolution pass does not establish hotplug or mixed-scale behavior. Automatic cursor-following is deferred. Verify Wayland environments in stages before claiming support, then package Arch/AUR and GitHub artifacts with an application icon, metadata, licences and safe installation/upgrades/removal.

Google Drive sync remains a future requirement outside the first release, with its long-term scope needing a separate decision. No sync integration currently exists. Extra Settings additions and manual drag ordering are removed from finishing work. Additional package formats are deferred. Dedicated Markdown support, export and application-level note encryption are outside the planned scope.

Current project documentation: [specification](agent-docs/SPEC.md), [milestone](agent-docs/current-milestone.md), [roadmap, completion conditions and follow-ups](agent-docs/implementation.md), and [verification checklist](agent-docs/manual-testing.md). These documents are tracked in Git.

## Requirements

- A Wayland compositor with `zwlr_layer_shell_v1` version 4 or newer. Protocol availability alone is not verified Stickies support.
- Rust and Cargo 1.92 or newer for the current locked dependencies. A minimum-toolchain build remains unverified.
- GTK4 4.10 or newer, libadwaita 1.7 or newer, Pango 1.56 or newer, PangoCairo, and gtk4-layer-shell 1.0 or newer development libraries, with mutually compatible transitive dependencies.
- Fontconfig and `glib-compile-resources`. Bundled fonts and their licenses are in `assets/fonts`.
- SQLite development library.
- `pkg-config`.

Confirm the native libraries are available:

```bash
pkg-config --modversion gtk4
pkg-config --modversion libadwaita-1
pkg-config --modversion pango pangocairo
pkg-config --modversion gtk4-layer-shell-0
pkg-config --modversion sqlite3
```

## Install v0.10.2

Package version is `0.10.2`; the GitHub release tag is `v0.10.2`.

- [GitHub downloads](https://github.com/0atxl/stickies/releases/tag/v0.10.2): Windows x86_64 ZIP and Arch Linux x86_64 package, matching sources, notices and checksums.
- [AUR stickies](https://aur.archlinux.org/packages/stickies): build from source.
- [AUR stickies-bin](https://aur.archlinux.org/packages/stickies-bin): install the prebuilt Arch download.
- [crates.io](https://crates.io/crates/stickies): install from source with Cargo and the native dependencies below.

On Windows, extract the whole ZIP and launch `bin\stickies.exe`. Rust and MSYS2 are not required for the ZIP. Keep all extracted runtime folders together. Notes and settings normally use `%LOCALAPPDATA%\stickies`, independently of the bundle folder. Quit before replacing the bundle. See [Windows packaging](packaging/windows.md) for storage and login behavior.

On an up-to-date Arch Linux x86_64 system, install the downloaded native package:

```bash
sudo pacman -U ./stickies-0.10.2-1-x86_64.pkg.tar.zst
```

The Arch binary does not target Ubuntu or Kali. The Linux edge requires a Wayland compositor with Layer Shell v4+. Package removal preserves user notes and settings. Disable Launch at login before removing the application if you also want its optional user startup entry removed.

For Cargo on Arch, install the build prerequisites, then compile the published source:

```bash
sudo pacman -S --needed base-devel rust pkgconf gtk4 libadwaita gtk4-layer-shell sqlite librsvg
cargo install stickies --version 0.10.2 --locked
```

Cargo installs only the executable. To add the Linux desktop launcher and icon, download the matching `stickies-0.10.2-src.tar.gz` from the release, extract it, and run these commands from its source directory:

```bash
install -Dm644 data/dev.stickies.Stickies.desktop "$HOME/.local/share/applications/dev.stickies.Stickies.desktop"
install -Dm644 data/dev.stickies.Stickies.png "$HOME/.local/share/icons/dev.stickies.Stickies.png"
```

Your graphical session must have Cargo's binary directory, normally `~/.cargo/bin`, on PATH for the launcher to find `stickies`. Cargo builds on Windows require the GNU target and MSYS2 native build environment described in the Windows guide; the ZIP is the ready-to-run Windows distribution.

Release files are unsigned. Verify `SHA256SUMS` before installation; checksums alone do not authenticate the publisher. Matching source and original notices accompany the binaries. See the [release notes](https://github.com/0atxl/stickies/releases/tag/v0.10.2) for known limitations.

The [Packages workflow](.github/workflows/packages.yml) also produces development artifacts from pushes, pull requests and manual runs. Those Actions downloads expire after 30 days. Public releases are separate, persistent downloads.

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

Global shortcuts belong to the compositor and are documented here rather than shown in Settings. Other Wayland compositors can bind the same CLI flags using their own configuration syntax. For current Omarchy Lua configurations, choose unused bindings:

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
