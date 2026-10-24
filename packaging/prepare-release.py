#!/usr/bin/env python3
"""Prepare Arch source/checksum inputs without building, installing or publishing."""

import argparse
import gzip
import hashlib
import io
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tomllib
import json


ROOT = Path(__file__).resolve().parent.parent
TARGET = "x86_64-unknown-linux-gnu"
SUPPLEMENTAL = {
    ("gtk4-layer-shell", "0.8.1"): "packaging/licenses/gtk4-layer-shell-MIT.txt",
    ("gtk4-layer-shell-sys", "0.6.1"): "packaging/licenses/gtk4-layer-shell-MIT.txt",
    ("rmcp", "2.2.0"): "packaging/licenses/rmcp-Apache-2.0.txt",
    ("rmcp-macros", "2.2.0"): "packaging/licenses/rmcp-Apache-2.0.txt",
}


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/packaging")
    parser.add_argument("--variant", choices=("standard", "mcp"), default="standard")
    args = parser.parse_args()
    features = ["--features", "mcp"] if args.variant == "mcp" else []
    package_name = "stickies-mcp" if args.variant == "mcp" else "stickies"
    output = args.output.resolve()
    if output.exists():
        parser.error("output already exists; choose a new directory to preserve its contents")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[.+][A-Za-z0-9.]+)?", version):
        parser.error("manifest version needs an explicit Arch pkgver mapping")
    revision = run("git", "rev-parse", "HEAD")
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", run("git", "show", "-s", "--format=%ct", "HEAD")))

    # Copy application inputs, including uncommitted new modules/migrations, and named packaging files.
    paths = run("git", "ls-files", "--cached", "--others", "--exclude-standard", "--", "Cargo.toml", "Cargo.lock", "build.rs",
                "LICENSE", "README.md", "src", "assets", "migrations", "data").splitlines()
    paths += ["packaging/arch/PKGBUILD", "packaging/prepare-release.py",
              "packaging/README.md", "packaging/licenses/gtk4-layer-shell-MIT.txt",
              "packaging/licenses/rmcp-Apache-2.0.txt"]
    icon = "data/dev.stickies.Stickies.png"
    if (ROOT / icon).is_file() and icon not in paths:
        paths.append(icon)
    files = {}
    for name in sorted(set(paths)):
        path = ROOT / name
        if path.is_symlink() or not path.is_file():
            parser.error(f"expected a regular source file: {name}")
        files[name] = path.read_bytes()

    # Cargo metadata supplies source paths; cargo tree selects enabled Linux
    # normal/build dependencies, excluding inactive optional/platform packages.
    metadata = json.loads(run("cargo", "metadata", "--frozen", "--format-version", "1",
                              "--filter-platform", TARGET, *features))
    tree = run("cargo", "tree", "--frozen", "--target", TARGET, "-e", "normal,build",
               "--prefix", "none", "--format", "{p}", *features)
    selected = {(line.split()[0], line.split()[1].removeprefix("v"))
                for line in tree.splitlines()}
    index = ["Rust normal/build dependency notices for " + TARGET,
             "Includes build tools as well as libraries; not every entry is linked into the binary.",
             "Versions and registry checksums are pinned in Cargo.lock.", ""]
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        key = package["name"], package["version"]
        if key not in selected or package["source"] is None:
            continue
        if not package["source"].startswith("registry+"):
            parser.error(f"dependency source needs notice review: {key}")
        crate = Path(package["manifest_path"]).parent
        notices = {p for p in crate.iterdir() if p.is_file() and p.name.lower().startswith(
            ("license", "licence", "copying", "notice", "copyright"))}
        if package.get("license_file"):
            notices.add(crate / package["license_file"])
        folder = f"packaging/notices/{key[0]}-{key[1]}"
        if key in SUPPLEMENTAL:
            files[f"{folder}/LICENSE"] = files[SUPPLEMENTAL[key]]
        elif not notices:
            parser.error(f"missing original dependency notice: {key}")
        for notice in sorted(notices):
            relative = notice.resolve().relative_to(crate.resolve())
            files[f"{folder}/{relative.as_posix()}"] = notice.read_bytes()
        index += [f"{key[0]} {key[1]}", f"  Licence: {package['license']}",
                  f"  Repository: {package.get('repository')}",
                  f"  Authors: {', '.join(package.get('authors', []))}", ""]
    files["packaging/notices/INDEX.txt"] = ("\n".join(index) + "\n").encode()

    # Install one attributed document rather than a directory per dependency.
    # Keep the original notice files in the matching source archive.
    notices = {}
    for name, data in sorted(files.items()):
        if name.startswith("packaging/notices/") or (
            name.startswith("assets/fonts/") and name.endswith("-LICENSE.txt")
        ):
            notices.setdefault(data, []).append(name)
    combined = bytearray(
        b"Third-party notices for Stickies (Linux)\n"
        b"Each section lists all original paths sharing the following text.\n"
    )
    for data, names in notices.items():
        combined.extend(("\n" + "=" * 72 + "\n" + "\n".join(names) + "\n\n").encode())
        combined.extend(data)
        combined.extend(b"\n")
    files["packaging/THIRD-PARTY-NOTICES.txt"] = bytes(combined)

    archive_name = f"{package_name}-{version}-src.tar.gz"
    icon_status = f"present; SHA-256 {digest(files[icon])}" if icon in files else "MISSING; package construction is blocked"
    source_info = (
        "Stickies packaging preparation, not a published release\n"
        f"Manifest version: {version}\nBuild variant: {args.variant}\nCargo features: {'mcp' if features else 'none'}\nGit baseline: {revision}\n"
        "Snapshot: current working-tree bytes of the selected inputs, not necessarily the Git baseline\n"
        f"SOURCE_DATE_EPOCH: {epoch}\nApproved application icon: {icon_status}\n"
        f"Matching source archive: {archive_name}\n"
        "Repository: https://github.com/0atxl/stickies\n"
        "Distribute this exact source archive and PKGBUILD alongside the Arch binary package.\n"
        "Tell recipients to obtain matching source from the same GitHub release assets.\n"
        "No release tag or published download URL has been selected by this preparation.\n"
        "Application licence: MPL-2.0. Embedded fonts retain their original OFL-1.1 notices.\n"
    )
    files["SOURCE-INFO.txt"] = source_info.encode()
    files["SOURCE-MANIFEST.sha256"] = "".join(
        f"{digest(data)}  {name}\n" for name, data in sorted(files.items())
    ).encode()

    # Deterministic source container metadata. The binary still depends on the
    # recorded compiler, native packages, makepkg configuration and flags.
    output.mkdir(parents=True)
    archive = output / archive_name
    with archive.open("wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=epoch) as compressed:
        with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as tar:
            for name, data in sorted(files.items()):
                entry = tarfile.TarInfo(f"stickies-{version}/{name}")
                entry.size = len(data)
                entry.mtime = epoch
                entry.mode = 0o644
                tar.addfile(entry, io.BytesIO(data))
    recipe = files["packaging/arch/PKGBUILD"].decode().replace("@VERSION@", version)
    recipe = recipe.replace("@SOURCE_SHA256@", digest(archive.read_bytes()))
    recipe = recipe.replace("@PACKAGE_NAME@", package_name)
    relationships = 'provides=("stickies=$pkgver")\nconflicts=("stickies")' if features else ""
    recipe = recipe.replace("@PACKAGE_RELATIONSHIPS@", relationships)
    recipe = recipe.replace("@SOURCE_ARCHIVE@", archive_name)
    recipe = recipe.replace("@MCP_ENABLED@", "1" if features else "0")
    recipe = recipe.replace("@CARGO_FEATURES@", " ".join(features))
    (output / "PKGBUILD").write_text(recipe)
    (output / "SOURCE-INFO.txt").write_text(source_info)
    environment = ["Preparation environment only; no package build or runtime verification is implied.",
                   run("rustc", "--version", "--verbose"), run("cargo", "--version"),
                   "Native pkg-config versions, in order: gtk4, libadwaita-1, gtk4-layer-shell-0, sqlite3, fontconfig",
                   run("pkg-config", "--modversion", "gtk4", "libadwaita-1", "gtk4-layer-shell-0", "sqlite3", "fontconfig")]
    (output / "PREPARATION.txt").write_text("\n".join(environment) + "\n")
    (output / "SHA256SUMS").write_text("".join(
        f"{digest(path.read_bytes())}  {path.name}\n"
        for path in sorted(output.iterdir()) if path.is_file()
    ))
    print(f"Prepared source, recipe and checksums at {output}")
    print(f"Approved application icon: {icon_status}")
    print("No binary package was built, installed, tested or published.")


if __name__ == "__main__":
    main()
