# Arch packaging preparation

Linux and Windows share `main`. Windows uses the [Windows ZIP guide](windows.md); the Arch procedure below remains Linux-specific.

## Standard and MCP builds

The default `standard` variant builds only `stickies`, without the optional MCP dependency tree, companion, IPC listener or Settings row. The `mcp` variant enables Cargo's `mcp` feature and includes `stickies-mcp`; access defaults to Off. Both keep Playful/Clean defaults and schema 5/config compatibility. The standard build accepts a saved `mcp_access` value but cannot serve MCP requests. Switching variants preserves notes and explicit saved settings. Install one variant at a time.

Prepare standard Arch inputs with `python3 packaging/prepare-release.py --variant standard --output /path/to/new-standard-directory`. Select `--variant mcp` for the companion build. Standard packages are named `stickies`; MCP packages are named `stickies-mcp`, provide `stickies` and conflict with it because they own the same application paths. Each source archive, recipe and notice inventory records its selected variant. Only enabled dependency notices enter the installed consolidated notice document.

The Packages workflow builds both variants independently on Arch and Windows. Its four artifact names include `standard` or `mcp` and the source commit. Arch artifacts contain one package plus matching sources/checksums. Windows standard ZIPs retain the `stickies-windows-x86_64` prefix; MCP ZIPs use `stickies-mcp-windows-x86_64`. CI runs the existing compilation, lint, tests and release build for both feature configurations. These are build artifacts, not published releases.

## Public release channels

The selected release is `v0.10.2`, with package version `0.10.2`. GitHub hosts Windows and Arch x86_64 downloads with matching sources, inventories and checksums. AUR `stickies` builds the source archive; `stickies-bin` installs the same prebuilt Arch package. crates.io distributes the application source for Cargo installation with native build prerequisites. The repository README contains user installation steps.

After downloading and verifying the release candidate's Arch Actions artifact, prepare the two AUR submissions with:

```bash
python scripts/prepare-aur.py --artifacts /path/to/arch-artifact --tag v0.10.2 --output /path/to/new-aur-directory
```

The helper verifies the artifact checksums and source recipe hash, writes GitHub release URLs and generates `.SRCINFO` with `makepkg --printsrcinfo`. It never builds or publishes. Verify the source recipe and binary repackaging before submission, then check the public asset URLs after GitHub publication and before pushing AUR. Keep AUR recipes and `.SRCINFO` together; do not upload binaries to AUR.

## Automated build artifacts

The [Packages workflow](../.github/workflows/packages.yml) builds Arch x86_64 and Windows x86_64 from the same workflow commit on pushes to `main`, pull requests targeting `main`, and manual dispatch. In GitHub, open Actions, select Packages, select a successful run, then download the artifact for your OS. A pull-request build uses GitHub's proposed merge commit. Artifact names include the full commit SHA, and each artifact contains its matching application source and checksums. Both platform jobs must succeed before treating a run as a complete candidate.

The Arch job uses a fresh `archlinux:base-devel` container, installs dependencies, then runs `makepkg --nocheck --nosign --log` as an unprivileged user through [ci-package-arch.sh](../scripts/ci-package-arch.sh). It collects the package, exact source archive, generated PKGBUILD, source/preparation records, actual BUILDINFO/PKGINFO, installed build-package inventory and verified SHA256SUMS. The Windows job uses the existing PowerShell packager with automated original-notice preparation.

Artifacts are retained for 30 days and are development downloads, not a published or signed release. The workflow has read-only repository permissions and does not create releases or tags. Native package repositories and compiler updates can change build inputs; inventories record their versions, but reproducibility is not claimed. The existing CI workflow remains separate and still runs its checks and tests. The Packages workflow itself runs no tests or app sessions. UI behavior, installation, upgrade/removal and clean-machine launch still need verification of the downloaded artifacts.

Both platform jobs passed in [run 39740913431](https://github.com/0atxl/stickies/actions/runs/39740913431) at commit `8f7622b`. This establishes hosted package construction, not installed-app behavior. A commit containing `[skip ci]` skips push-triggered workflows; after the workflow is on `main`, an explicitly authorized manual Packages run can build that commit without running the separate test workflow.

Stickies is a Linux Wayland app requiring `zwlr_layer_shell_v1` v4+ and compatible native libraries. This pass prepares an Arch Linux x86_64 package recipe and source inputs. The owner-approved application icon is included. The downloaded CI candidate passed focused installation, launcher, same-version reinstall and removal checks in a disposable Arch container; actual login and older-version upgrades remain unverified.

The GitHub binary download will be labelled **Arch Linux x86_64 only**, using the same `.pkg.tar.zst` produced by the PKGBUILD. It depends on Arch's native libraries. It is not a portable Linux binary, and no Kali or Ubuntu binary compatibility is claimed. Other package formats are outside this pass.

## Desktop evidence and intended compatibility

- The recorded Search repair passed scoped checks in isolated Sway 1.12 with one 1280 × 800 scale-1 output. That run predates the final monitor/layout changes.
- The owner confirmed normal-desktop Search, resolution recovery and fixed-size Search acceptance. Their compositor/version and display setup were not specified.
- Hyprland is the development reference. The isolated Hyprland attempt could not start without `/dev/dri`, so it establishes no additional app pass.
- Sway, KDE Plasma Wayland and other Layer Shell desktops remain intended compatibility targets, with support limited to actual recorded checks. Multi-monitor behavior remains unverified. GNOME Wayland without Layer Shell has no implemented fallback. X11 is outside this release.

## Dependencies

Use an up-to-date Arch x86_64 build environment with `base-devel`, `rust>=1:1.92`, `pkgconf` and `desktop-file-utils`. Python 3.11+ is needed only for the source-preparation script. The highest declared Rust requirement in the locked dependencies is 1.92; this is source metadata, not a verified minimum-toolchain build. Version constraints include Arch's package epochs, such as `1:`, separately from upstream versions.

The recipe names the direct native runtime libraries as Arch packages: `cairo`, `fontconfig`, `gdk-pixbuf2`, `glib2`, `glibc`, `graphene`, `gtk4>=1:4.10`, `gtk4-layer-shell>=1.0`, `libadwaita>=1:1.7`, `libgcc`, `pango>=1.56` and `sqlite`. `librsvg` supplies SVG support for the embedded UI icons. Their dependencies supply Wayland libraries and the rest of the native stack. `glib2` supplies `glib-compile-resources`, which `build.rs` invokes. Arch includes development files with these native packages; no system font packages are needed for the embedded fonts.

Pango 1.56+ is required by the shared bundled-font registration API; PangoCairo is part of the Pango native stack.

These are source/feature requirements, not a promise that a binary built with newer libraries runs against every older version. Adwaita and GTK must also satisfy their own transitive requirements. The recipe also depends on `dbus`; the desktop must supply a running session bus and Layer Shell v4+. A compiler does not need an active Wayland session. Do not make a particular compositor or Omarchy a package dependency.

## Installed files and licences

| File | Package path |
|---|---|
| Executable | `/usr/bin/stickies` |
| MCP companion, MCP variant only | `/usr/bin/stickies-mcp` |
| Existing desktop entry | `/usr/share/applications/dev.stickies.Stickies.desktop` |
| Approved PNG application icon | `/usr/share/pixmaps/dev.stickies.Stickies.png` |
| Application MPL-2.0 | `/usr/share/licenses/stickies/LICENSE` |
| Font and Rust dependency notices with inventory | `/usr/share/licenses/stickies/THIRD-PARTY-NOTICES.txt` |
| Source identity and this guide | `/usr/share/doc/stickies/` |

The owner supplied and approved `Frosted Layered Sticky Notes.png` and confirmed it is their own design, with no third-party attribution identified. `data/dev.stickies.Stickies.png` preserves that 1254 × 1254 transparent PNG byte-for-byte, SHA-256 `edf8cd958d73fa9a79e909dd5ff8d1d447657eaf27852c6d91a6e079d75e3f2c`. No conversion, redraw or substitute artwork is used. Keep the desktop entry's existing icon identifier. The original raster uses the standard unthemed [pixmaps lookup](https://specifications.freedesktop.org/icon-theme/latest/), avoiding an invented SVG or a mismatched fixed-size theme directory. Fonts, CSS, UI icons and migrations are already embedded; do not install separate copies of the fonts or alter application resource loading.

Original Rust dependency notice files are copied from Cargo's cached sources for the enabled Linux normal/build dependency tree. The matching source archive retains those originals. Installed Arch packages use one `THIRD-PARTY-NOTICES.txt` document for font/dependency notices, grouping identical texts with all original paths and retaining package attribution; the application MPL text remains separate. This reduces the package's file/directory count without removing notice content. The inventory includes build tools, so some entries are not linked into the executable. The two locked gtk4-layer-shell bindings omit their MIT file from their crate archives. `packaging/licenses/gtk4-layer-shell-MIT.txt` is the identical original notice verified at upstream commits [7043a44](https://github.com/pentamassiv/gtk4-layer-shell-gir/blob/7043a44adc4e31868e6f30a4f8c8279168417a94/LICENSE) for 0.8.1 and [c52b9b1](https://github.com/pentamassiv/gtk4-layer-shell-gir/blob/c52b9b12e6b1afa38dcb496ec821c55bb298da78/LICENSE) for sys 0.6.1. The script applies that supplement only to those versions and stops on other missing notices. Native shared libraries are provided by Arch, not bundled here.

## Model Context Protocol (MCP) companion

The MCP variant includes a companion binary, `stickies-mcp`, installed at `/usr/bin/stickies-mcp`. It communicates with MCP clients using JSON-RPC over standard I/O.

### Access control

MCP access defaults to Off. Enable it in Stickies Settings under the MCP access control:
- **Off**: All MCP queries and mutations are rejected.
- **Read only**: Clients can query and read published active notes. Mutations are rejected.
- **Read/write**: Clients can query, read, create and update active notes without additional per-operation confirmation prompts.

Changes to this setting take effect immediately. Revoking access blocks subsequent calls without restarting the app.

### Native client configuration

Configure your local MCP client by referencing the companion executable directly. Keep command paths and arguments separate. Do not edit client configuration files programmatically.

Example configuration for Claude Desktop on Linux (`~/.config/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "stickies": {
      "command": "/usr/bin/stickies-mcp"
    }
  }
}
```

### Operational requirements and limitations

- **Running app required**: Stickies must already be running in the user session. The companion never launches the GUI, quits the GUI, opens the SQLite database directly, or writes configuration files. If Stickies is closed, starting or shutting down, the companion returns a structured tool error.
- **Upgrade procedure**: Always close Stickies before upgrading. The GUI and companion share an internal IPC contract version and must be updated together.
- **Data scope**: MCP access includes published active notes and pending editor text. Drafts, archived notes and deleted notes are excluded.
- **Local authorization**: Linux checks the D-Bus caller UID. Windows uses a pipe named from the process token user SID, a protected DACL granting only that user access, and remote-client rejection. This permits other processes running as the same user.
- **Security**: Note contents are treated as untrusted text. No shell commands or links in notes are executed. Titles, bodies and search queries are never logged to stderr.
- **Platform scope**: Native local clients are supported. WSL, containers, SSH forwarding and remote clients are outside v1 compatibility.

### Note workflow and write contract

The four tools are `notes_find`, `notes_get`, `notes_create` and `notes_update`. A client can create an edge note containing ordinary reminder text or `- [ ] ` checklist lines. The user edits it or checks items in Stickies. Before later changes, the client reads the current body and revision, retains completed `- [x] ` markers and supplies the intended replacement. There is no task model, monitoring, sync or timed reminder engine.

`notes_create` defaults omitted title/body to empty strings and formatting to `[]`. `notes_update` preserves omitted title. Body and formatting must be supplied together; `[]` explicitly clears formatting. Omit both to retain current text, checkbox states and formatting. Formatting uses Unicode scalar offsets. Titles are limited to 500 UTF-8 bytes. Serialized title, body and formatting together are limited to 1,000,000 bytes, including JSON escaping, on reads and writes. Requests and responses have a 2 MiB framing limit. Formatting-load errors fail the call.

A revision hashes a structured JSON byte array of title, body and canonical formatting with SHA-256. It includes pending session text and checkbox states. Formatting uses the same serialization as the editor. A stale revision rejects the entire write, leaving pending human edits intact. The client must reread before retrying with a new revision and operation ID. Whole-body replacement does not merge concurrent changes automatically.

Each mutation saves its receipt in the same SQLite transaction as the note. Retrying identical arguments with the same operation ID returns the original note ID and revision, even after subsequent note edits or an app restart. Reusing that ID with different arguments fails, including omitted versus explicitly empty fields. Receipts remain in the local database without automatic expiry. A timeout or lost connection does not prove that a write failed; retry identical arguments with the same ID. Existing pre-repair MCP receipts/revisions use an incompatible fingerprint scheme. Do not replay those operation IDs after upgrading; reread notes and use fresh IDs.

Successful changes refresh All Notes and edge tabs without opening an editor or requesting focus. Open editors share checklist widgets and one Undo step per MCP update. Failed writes leave session contents, dirty state and pending autosaves intact. IPC waits are bounded to ten seconds; Windows shutdown cancels pipe waits and joins the worker. Settings are checked on the GUI thread for every dispatched request, including receipt replays.

Schema 5 adds `mutation_receipts`. The note and checklist formats remain unchanged. Close Stickies before upgrading and update the GUI and companion together. The unreleased source still reports version 0.10.2; it is not the already published schema-4 v0.10.2 package.

Both packagers include the selected variant: the standard build has the GUI only, and the MCP build has both executables. Both source snapshots retain the modules/migrations. They retain the consolidated notice layout from main. The exact `rmcp` and `rmcp-macros` 2.2.0 crate archives omit LICENSE. Both packagers use `packaging/licenses/rmcp-Apache-2.0.txt`, copied from [the repository LICENSE at their recorded source commit](https://github.com/modelcontextprotocol/rust-sdk/blob/519577601db3823616dbd7c4eb84ed569d8e17d4/LICENSE). The text includes its licensing-transition statement. Other missing notices still stop packaging. The matching Windows source ZIP and Cargo source list include the supplement.

Linux formatting, compilation/linking and Clippy passed for both binaries. Focused isolated Linux/Hyprland checks passed for create/read/update, checklist completion/formatting, shared views, Undo/Redo, persistence, retries, stale revisions and permission/save failures. The checkbox-focus replacement crash was repaired and its focused regressions passed. Native Windows compilation, PowerShell packaging execution and Windows MCP runtime remain unverified. The Windows implementation is retained. Historical release evidence below predates this MCP build.

## Prepare source and checksums locally

`packaging/arch/PKGBUILD` is an input template. Run the script from this checkout to produce the usable recipe with a real source checksum:

```sh
python3 packaging/prepare-release.py
```

The script uses cached Cargo metadata only. On a new builder, first fetch the locked dependencies with `cargo fetch --locked --target x86_64-unknown-linux-gnu` when network use is authorized. Preparation itself neither downloads nor compiles anything. An existing output directory is refused; use `--output /path/to/a/new/directory` to preserve older inputs.

The default output, `target/packaging/`, contains the source archive, generated `PKGBUILD`, `SOURCE-INFO.txt`, `PREPARATION.txt` and `SHA256SUMS`. The source version comes from Cargo.toml, currently `0.10.2`; it does not select a release version/tag. The archive contains current bytes of tracked and untracked application inputs under the selected source paths, the named packaging files and generated notices. `SOURCE-INFO.txt` records the Git baseline and explicitly identifies a working-tree snapshot. `SOURCE-MANIFEST.sha256` inside the archive identifies those bytes. Unrelated local files, build outputs, agent material and saved notes are excluded.

Source archive paths, order, timestamps, ownership and modes are normalized. `SOURCE_DATE_EPOCH` defaults to the Git baseline's commit time and is recorded. Reuse it when recreating the same source snapshot. Archive normalization and Cargo.lock do not establish bit-for-bit binary reproducibility.

Static inspection commands, without building or installing a package:

```sh
bash -n packaging/arch/PKGBUILD
bash -n target/packaging/PKGBUILD
desktop-file-validate data/dev.stickies.Stickies.desktop
cd target/packaging
sha256sum --check SHA256SUMS
```

## Later package build and GitHub assets

The source inputs include the approved PNG, and `prepare()` still rejects a missing icon. A separately authorized clean Arch builder can run `makepkg --nocheck` beside the generated PKGBUILD and archive. There is no `check()` function. Do not use `--syncdeps` or `--install` during preparation. `prepare()` validates the desktop file and fetches locked dependencies; `build()` uses `cargo build --release --frozen --target x86_64-unknown-linux-gnu` in a fresh build directory. It never reuses the checkout's old executable.

For an equivalent source compilation, extract the exact archive, enter its root, fetch its locked dependencies, then use that same Cargo build command. Export the `SOURCE_DATE_EPOCH` value recorded in `SOURCE-INFO.txt` before either build. Record Rust/Cargo versions, target, native package versions, makepkg configuration and compiler flags. Retain makepkg's actual `.BUILDINFO`; `PREPARATION.txt` records only the source-preparation environment. Proving binary reproducibility requires a later independent build with matching inputs and comparison.

Once release identity and distribution are separately approved, GitHub assets should contain:

- The Arch x86_64 `.pkg.tar.zst` with the explicit Arch-only download label.
- The exact source archive and generated PKGBUILD used to build it, including all notices.
- The actual `.BUILDINFO`, and SHA-256 checksums for every distributed file.

Keep those files together so recipients can obtain the matching MPL source. Finalize `SHA256SUMS` only after the package and `.BUILDINFO` exist, excluding the checksum file itself. Verify it before publication. No binary checksum, release/tag, signature or published artifact is claimed by local source preparation. AUR submission and other package formats remain outside this pass.

With exactly one package and source archive in a fresh output directory, the later artifact commands are:

```sh
bsdtar -xOf stickies-*-x86_64.pkg.tar.zst .BUILDINFO > BUILDINFO
sha256sum PKGBUILD SOURCE-INFO.txt PREPARATION.txt BUILDINFO \
    stickies-*-src.tar.gz stickies-*-x86_64.pkg.tar.zst > SHA256SUMS
sha256sum --check SHA256SUMS
```

## Upgrade, removal and optional autostart

The package owns only the system files above. There are no install/remove hooks, configuration files under `/etc`, database migrations run by the package, or writes to home directories. Standard Arch desktop-entry cache hooks can handle the launcher cache.

Upgrades replace package-owned files. Notes at `$XDG_DATA_HOME/stickies/notes.db`, preferences at `$XDG_CONFIG_HOME/stickies/config.toml` and cached fonts at `$XDG_CACHE_HOME/stickies/fonts` remain user-owned. The usual defaults are `~/.local/share`, `~/.config` and `~/.cache`. Quit Stickies before an upgrade so the running process and installed binary agree. This MCP build migrates schema 4 to 5 at GUI startup, including when MCP access is Off. Older executables, including released v0.10.2, reject schema 5. Back up the database with Stickies closed before the first run. Downgrading needs a compatible pre-upgrade backup; do not lower `user_version` by hand.

Launch at login stays optional and app-managed. On valid startup, Stickies creates or removes `$XDG_CONFIG_HOME/autostart/dev.stickies.Stickies.desktop` according to `launch_at_login`. Do not install `/etc/xdg/autostart` entries or a new service. An enabled user entry survives package removal. Disable Launch at login before removing the package, or explicitly remove that single entry afterward if desired. Its existing `TryExec=stickies` guards an absent executable. Reinstalling retains the stored preference. Package removal must leave notes, preferences, caches and other users' autostart entries alone.

## Remaining verification

The application artwork is approved and included; release identity is v0.10.2. Focused downloaded-candidate installation, launch, same-version reinstall and removal checks passed. Desktop icon lookup, actual login and older-version upgrade coverage remain incomplete. Keep the separate reliability/Wayland gaps in the current project record; packaging does not resolve them. The [October 19 source maintenance pass](../agent-docs/manual-testing.md#october-19-gtk-deprecation-maintenance) reports zero application warnings on Linux, down from the integrated 35. Existing packages predate those replacements; no package was regenerated or verified in that pass.

The smallest later, separately authorized installation check is:

1. Build in a clean Arch x86_64 environment and inspect dependency closure, installed file list, notices, checksums and actual `.BUILDINFO`.
2. In a disposable supported Wayland session, install and verify the launcher/icon, CLI actions and optional login launch without development-machine rules.
3. Upgrade with disposable notes/preferences and confirm retention. Remove the package and confirm user data and unrelated autostart entries survive; exercise only the documented optional-entry cleanup.
