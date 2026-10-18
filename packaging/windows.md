# Windows ZIP packaging

The Windows port keeps the accepted GTK interface, shared note sessions and anchor-based checklist repair. It is separate from the Linux Wayland release. Windows clean-machine compatibility, mixed-DPI placement, focus, single-instance failure handling and session-end saving remain unverified.

## Installation-free launch and storage

Extract the whole binary ZIP and launch `bin\stickies.exe`. Keep `bin`, `lib`, `share`, `licenses` and the accompanying metadata together. Runtime resources and helpers resolve relative to the extracted bundle. Launching from another working directory and an extraction path containing spaces or Unicode still needs Windows verification. No MSYS2 installation or developer tools should be required by recipients.

Installation-free does not mean that data travels with the ZIP. Notes use GLib's user data directory plus `stickies\notes.db`; settings use its user configuration directory plus `stickies\config.toml`. Both normally live under `%LOCALAPPDATA%\stickies`. `XDG_DATA_HOME` and `XDG_CONFIG_HOME` override those base directories when supplied before launch. Bundled fonts are cached below GLib's user cache directory. These user directories must be writable; the extraction directory need not be writable. Moving or deleting the extracted bundle leaves user notes and preferences in place. There is no adjacent-data portable mode.

Launch at login is optional and uses the current user's Windows Run registry key with a quoted executable path. Disable it before moving/removing the bundle, or re-enable it from the new location. Saved note bodies, formatting offsets and schema migrations remain the same as on Linux.

The tray offers strip toggling, New Note, Quick Search, All Notes and Quit. Quit flushes all shared sessions and stays open with a visible, editor-owned error dialog if a save fails. Explorer taskbar recreation re-registers the existing icon. These fixes have no Windows runtime acceptance yet.

On Windows, the existing hide-in-DND preference responds to explicit Presentation Mode only. `SHQueryUserNotificationState` does not expose the modern Windows DND switch here. First-login/upgrade quiet time no longer hides the strip. Fullscreen policy retains the existing polling heuristic and needs runtime verification.

## Build prerequisites

Use 64-bit Windows PowerShell 5.1 or PowerShell 7, Git, Rust/Cargo 1.92+ with the `x86_64-pc-windows-gnu` target already available, and an existing MSYS2 UCRT64 toolchain. Required tools include GCC, objdump, pkg-config, glib-compile-resources and the matching MSYS2 pacman package database. Native requirements include GTK 4.10+, libadwaita 1.7+, **Pango 1.56+**, PangoCairo, SQLite and SVG-capable GdkPixbuf, with compatible transitive libraries and icon/schema assets. Pango 1.56 is required by `pango_font_map_add_font_file` on both platforms.

The script never installs dependencies or downloads Cargo packages. Populate the exact Cargo.lock cache separately when authorized. Its `--frozen` build enforces the lockfile and offline operation. The GNU Windows target and linker are explicit. Native DLLs remain tied to the installed MSYS2 versions, recorded in the output; Cargo.lock does not pin those libraries.

Pass the UCRT64 prefix explicitly, or put that prefix's pkg-config.exe on PATH. There is no machine-specific fallback. A prefix containing spaces needs a working space-free 8.3 alias for pkg-config flags; otherwise the script stops with an explanation. This build-tool limitation does not require recipients to extract the application into a space-free path.

## Packaging procedure

Run only after packaging execution is authorized. For example, from any working directory:

```powershell
& 'C:\work\Stickies\scripts\package-windows.ps1' -MsysRoot 'C:\msys64\ucrt64' -OutputDirectory 'C:\artifacts\Stickies Windows'
```

The output directory must not exist. Previous outputs are never removed. The script snapshots explicit application inputs, including untracked `src\*.rs` files, and builds that snapshot into a fresh build directory. Agent material, existing artifacts and user storage are excluded. Review the snapshot before sharing.

Every required helper, loader, schema and icon tree must exist. The script checks objdump's exit status and x86_64 PE format, recursively resolves imports, and rejects unresolved imports except declared Windows system libraries/API contracts. It normalizes loader-cache module paths to bundle-relative paths and requires an SVG loader. Static dependency closure does not verify optional runtime loading, graphics drivers or clean-machine operation.

The bundle includes the application MPL text, original font notices, enabled Windows Rust normal/build dependency notices, and original notices from every MSYS2 package owning copied native files. Installed package metadata records native versions, licences and upstream URLs. A missing notice stops packaging rather than silently omitting it or inventing replacement text.

Some MSYS2 split packages do not install their own notices. Supply original notices for the exact installed versions under a separate directory, with one child named after each full MSYS2 package name, then pass `-NativeNoticeRoot 'C:\release-inputs\native-notices'`. Include original copyright/exception texts and a short source/provenance record in each child. Review these inputs before distribution; the script copies them without downloading anything. The error identifies a package whose notices are still missing.

## Files to distribute together

- `stickies-windows-x86_64.zip`, containing the executable/runtime, readable notices, this guide, `SOURCE-INFO.txt`, `SOURCE-MANIFEST.sha256`, `SOURCE-ARCHIVE.sha256`, `BUILD-INFO.txt` and `BUNDLE-MANIFEST.sha256`.
- `stickies-windows-x86_64-source.zip`, containing the exact application snapshot built, its manifest, licence, build script and this guide.
- `SHA256SUMS`, covering both archives. The binary bundle also records its matching source archive's checksum.

The source manifest hashes application inputs and excludes itself and generated source instructions. The bundle manifest hashes bundle contents and excludes itself. Build information records the target, command, Rust/compiler/binutils versions, relevant Rust flags and bundled native package versions. The Git baseline is recorded as context, not a claim that uncommitted changes are absent. These records do not establish bit-for-bit reproducibility.

Keep matching application source available alongside the binary download. Cargo.lock identifies exact Rust registry sources; the native package inventory identifies the MSYS2 versions whose matching sources can be obtained through the MSYS2 package/source archive catalogues. Preserve the original notices and any applicable source-distribution requirements when sharing native libraries.

The existing repository-root ZIP predates these fixes. This change does not regenerate or approve it. Sharing a new build remains blocked on a separately authorized packaging run, review of the generated notices/manifests, and focused Windows verification.
