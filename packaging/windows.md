# Windows ZIP packaging

The Windows port keeps the accepted GTK interface, shared note sessions and anchor-based checklist repair. Linux and Windows share the main source branch and produce separate platform packages. Windows clean-machine compatibility, mixed-DPI placement, general focus behavior, single-instance failure handling and session-end saving remain unverified.

On October 22 the owner reported that the latest Windows build works after the combined detached-resize and Open away correction, and also accepted the latest Linux build. This is owner verification in the development environments, not clean-machine or full release acceptance.

## Installation-free launch and storage

Extract the whole binary ZIP and launch `bin\stickies.exe`. Keep `bin`, `lib`, `share`, `licenses` and the accompanying metadata together. Runtime resources and helpers resolve relative to the extracted bundle. Launching from another working directory and an extraction path containing spaces or Unicode still needs Windows verification. No MSYS2 installation or developer tools should be required by recipients.

Installation-free does not mean that data travels with the ZIP. Notes use GLib's user data directory plus `stickies\notes.db`; settings use its user configuration directory plus `stickies\config.toml`. Both normally live under `%LOCALAPPDATA%\stickies`. `XDG_DATA_HOME` and `XDG_CONFIG_HOME` override those base directories when supplied before launch. Bundled fonts are cached below GLib's user cache directory. These user directories must be writable; the extraction directory need not be writable. Moving or deleting the extracted bundle leaves user notes and preferences in place. There is no adjacent-data portable mode.

Launch at login is optional and uses the current user's Windows Run registry key with a quoted executable path. Disable it before moving/removing the bundle, or re-enable it from the new location. Saved note bodies, formatting offsets and schema migrations remain the same as on Linux.

The tray offers strip toggling, New Note, Quick Search, All Notes and Quit. Settings also offers Quit Stickies on Linux and Windows through the same application action. Quit flushes all shared sessions and stays open with a visible, editor-owned error dialog if a save fails. Closing All Notes with × does not quit the app. Explorer taskbar recreation re-registers the existing icon. Failed-save retention and Explorer recovery still lack explicit Windows runtime results.

The edge hosts stay out of the taskbar while the strip, docked note or Search is showing. All Notes and detached notes keep normal taskbar behavior. A Windows subclass preserves the edge hosts' tool-window styles across GTK style updates and removes itself when the native window is destroyed. The tray remains available for strip toggling, opening All Notes and Quit. The taskbar fix is included in the owner-accepted Windows build.

Successfully opening a docked note away collapses the strip to its marker through the existing animation on Windows and Linux. It does not depend on a pointer-leave event from the removed editor. A failed save retains the docked editor and expanded strip. Opening a different note away from All Notes preserves an unrelated docked editor.

Individual notes on both platforms resize between 300 × 280 and 640 × 720 logical pixels, with a smaller maximum on small monitors. Detached notes use their existing grip for bounded app-managed resizing. Windows detached notes also enforce these limits through native size constraints, including maximization; GTK resizing stays enabled so fixed-size hints do not block the grip. Linux keeps compositor resizing disabled. On Windows, individual-note footer popups are raised after mapping without activating or moving their note; edge geometry updates preserve stacking order. The owner accepted the latest fixes on both platforms; small-monitor and mixed-DPI coverage remain unestablished.

On Windows, the existing hide-in-DND preference responds to explicit Presentation Mode only. `SHQueryUserNotificationState` does not expose the modern Windows DND switch here. First-login/upgrade quiet time no longer hides the strip. Fullscreen policy retains the existing polling heuristic and needs runtime verification.

## Build prerequisites

For routine builds, use the repository's [Packages workflow](../.github/workflows/packages.yml). It installs MSYS2 UCRT64 and the Rust GNU target on a Windows runner, fetches Cargo.lock dependencies, then invokes the same packaging script described below. Download the Windows artifact from a successful run in GitHub Actions. It contains both ZIPs and SHA256SUMS; preserve them together. This removes the need to transfer patches and compile manually on the Azure VM for each committed change. Runtime checks still require a Windows desktop.

Use 64-bit Windows PowerShell 5.1 or PowerShell 7, Git, Rust/Cargo 1.92+ with the `x86_64-pc-windows-gnu` target already available, and an existing MSYS2 UCRT64 toolchain. Required tools include GCC, objdump, pkg-config, glib-compile-resources and the matching MSYS2 pacman package database. Native requirements include GTK 4.10+, libadwaita 1.7+, **Pango 1.56+**, PangoCairo, SQLite and SVG-capable GdkPixbuf, with compatible transitive libraries and icon/schema assets. Pango 1.56 is required by `pango_font_map_add_font_file` on both platforms.

The script never installs dependencies or downloads Cargo packages. Populate the exact Cargo.lock cache separately when authorized. Its `--frozen` build enforces the lockfile and offline operation. The GNU Windows target and linker are explicit. Native DLLs remain tied to the installed MSYS2 versions, recorded in the output; Cargo.lock does not pin those libraries.

Pass the UCRT64 prefix explicitly, or put that prefix's pkg-config.exe on PATH. There is no machine-specific fallback. A prefix containing spaces needs a working space-free 8.3 alias for pkg-config flags; otherwise the script stops with an explanation. This build-tool limitation does not require recipients to extract the application into a space-free path.

## Packaging procedure

Run only after packaging execution is authorized. For example, from any working directory:

```powershell
& 'C:\work\Stickies\scripts\package-windows.ps1' -MsysRoot 'C:\msys64\ucrt64' -OutputDirectory 'C:\artifacts\Stickies Windows'
```

The output directory must not exist. Previous outputs are never removed. The script snapshots explicit application inputs, including untracked `src\*.rs` files, and builds that snapshot into a fresh build directory. Agent material, existing artifacts and user storage are excluded. Review the snapshot before sharing.

Before each ZIP, staged file and directory modification times outside ZIP's supported range of 1980-01-01 through 2107-12-31 23:59:58 are set to 2000-01-01. This handles native resources with old timestamps without changing their bytes or installed MSYS2 files. Build information records the policy; byte manifests and archive checksums still cover the packaged contents.

Every required helper, loader, schema and icon tree must exist. The script checks objdump's exit status and x86_64 PE format, recursively resolves imports, and rejects unresolved imports except declared Windows system libraries/API contracts. It normalizes loader-cache module paths to bundle-relative paths and requires an SVG loader. Static dependency closure does not verify optional runtime loading, graphics drivers or clean-machine operation.

The bundle includes the application MPL text, original font notices, enabled Windows Rust normal/build dependency notices, and original notices from every MSYS2 package owning copied native files. Installed package metadata records native versions, licences and upstream URLs. A missing notice stops packaging rather than silently omitting it or inventing replacement text.

MSYS2 generates `loaders.cache`, `gschemas.compiled` and each icon theme's `icon-theme.cache`, so these caches need no direct pacman owner. The script checks ownership and notices for the bundled loader DLLs, the copied Adwaita/hicolor theme files and all installed `.gschema.xml` and `.gschema.override` inputs instead. Only the root `icon-theme.cache` in each copied theme gets this exception. `BUILD-INFO.txt` lists the generated caches and hashes the schema inputs; `BUNDLE-MANIFEST.sha256` covers the bundled cache bytes. Missing schema sources, other unowned native files and unowned schema inputs still stop packaging.

Some MSYS2 split packages do not install their own notices. Supply original notices for the exact installed versions under a separate directory, with one child named after each full MSYS2 package name, then pass `-NativeNoticeRoot 'C:\release-inputs\native-notices'`. Include original copyright/exception texts and a short source/provenance record in each child. Review these inputs before distribution; the script copies them without downloading anything. The error identifies a package whose notices are still missing.

CI runs [prepare-windows-notices.py](../scripts/prepare-windows-notices.py) before packaging. It checks the installed versions of `gstreamer 1.28.8-1`, `gst-plugins-base 1.28.8-1` and `gst-plugins-bad-libs 1.28.8-2`, downloads their original upstream archives, and verifies the SHA-256 values from reviewed MSYS2 recipes at commit `98ee2b8de5c36785f977dc6ccbdef1a1b2043cdb`. It preserves readable notice files, complete upstream archives, recipes and provenance in the supplements. The complete archives retain per-file notices; they are not complete patched MSYS2 build trees. A version or hash mismatch stops the job before packaging. When MSYS2 updates these packages, review the new recipes/notices and update this helper's version/hash table. Other missing notices continue to stop the existing packager.

## Files to distribute together

- `stickies-windows-x86_64.zip`, containing the executable/runtime, readable notices, this guide, `SOURCE-INFO.txt`, `SOURCE-MANIFEST.sha256`, `SOURCE-ARCHIVE.sha256`, `BUILD-INFO.txt` and `BUNDLE-MANIFEST.sha256`.
- `stickies-windows-x86_64-source.zip`, containing the exact application snapshot built, its manifest, licence, build script and this guide.
- `SHA256SUMS`, covering both archives. The binary bundle also records its matching source archive's checksum.

The source manifest hashes application inputs and excludes itself and generated source instructions. The bundle manifest hashes bundle contents and excludes itself. Build information records the target, command, Rust/compiler/binutils versions, relevant Rust flags and bundled native package versions. The Git baseline is recorded as context, not a claim that uncommitted changes are absent. These records do not establish bit-for-bit reproducibility.

Keep matching application source available alongside the binary download. Cargo.lock identifies exact Rust registry sources; the native package inventory identifies the MSYS2 versions whose matching sources can be obtained through the MSYS2 package/source archive catalogues. Preserve the original notices and any applicable source-distribution requirements when sharing native libraries.

The existing repository-root ZIP predates these fixes. The owner has built newer ZIPs and accepted the latest behavior, but the old ZIP remains historical. Public release still requires CI-built artifacts from the chosen source commit, review of notices/manifests and verification of the actual release bundle on a clean Windows environment.
