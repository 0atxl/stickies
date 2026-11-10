#!/usr/bin/env python3
"""Fetch verified original notices for the known MSYS2 GStreamer notice gaps."""

import argparse
import hashlib
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import urllib.request


RECIPE_REVISION = "dd1d7b24b44e2f6c2fd2bc33abfc810307bd10b9"
# Reviewed MSYS2 package versions and the first source hash in their PKGBUILDs.
# Stop on updates until the new upstream notices and recipe have been reviewed.
SOURCES = {
    "gstreamer": ("gstreamer", "1.28.9-1", "6dbf744f583131a34845cc641e803843766a9ba80ce00378a9043fd6d8d5fb38"),
    "gst-plugins-base": ("gst-plugins-base", "1.28.9-1", "c1fda668b520e6aef763c0eb962c032c9fe52bce322e8b00c69b4e9e847bddc1"),
    "gst-plugins-bad-libs": ("gst-plugins-bad", "1.28.9-1", "4a4e0b7bf84f868378c48f143a7ac3a98923b488a5b74f1a35ba46bc3c43f6b1"),
}


def download(url):
    with urllib.request.urlopen(url, timeout=60) as response:
        return response.read()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--msys-root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists; choose a fresh directory")
    pacman = args.msys_root.resolve().parent / "usr/bin/pacman.exe"
    # Check every version before downloading or writing a supplement.
    for suffix, (_, version, _) in SOURCES.items():
        package = f"mingw-w64-ucrt-x86_64-{suffix}"
        installed = subprocess.check_output([str(pacman), "-Q", package], text=True).strip()
        if installed != f"{package} {version}":
            parser.error(f"unreviewed notice version: {installed}; expected {version}")

    args.output.mkdir(parents=True)
    for suffix, (source, version, expected_hash) in SOURCES.items():
        package = f"mingw-w64-ucrt-x86_64-{suffix}"
        upstream_version = version.rsplit("-", 1)[0]
        archive_name = f"{source}-{upstream_version}.tar.xz"
        url = f"https://gstreamer.freedesktop.org/src/{source}/{archive_name}"
        recipe_url = (
            "https://raw.githubusercontent.com/msys2/MINGW-packages/"
            f"{RECIPE_REVISION}/mingw-w64-{source}/PKGBUILD"
        )
        data = download(url)
        if hashlib.sha256(data).hexdigest() != expected_hash:
            raise RuntimeError(f"Source checksum mismatch for {package}")
        folder = args.output / package
        folder.mkdir()
        archive_path = folder / archive_name
        archive_path.write_bytes(data)
        (folder / "MSYS2-PKGBUILD").write_bytes(download(recipe_url))
        # Retain the full source for per-file notices, plus readable notice files.
        # Copy bytes ourselves rather than extracting archive paths or links.
        notice_count = 0
        with tarfile.open(archive_path) as archive:
            for member in archive:
                path = PurePosixPath(member.name)
                if not member.isfile() or not path.name.lower().startswith(
                    ("copying", "license", "licence", "notice", "copyright", "authors")
                ):
                    continue
                if path.is_absolute() or ".." in path.parts or ":" in member.name or "\\" in member.name:
                    raise RuntimeError(f"Unsafe notice path: {member.name}")
                destination = folder / "original-notices" / Path(*path.parts)
                destination.parent.mkdir(parents=True, exist_ok=True)
                with archive.extractfile(member) as notice:
                    destination.write_bytes(notice.read())
                notice_count += 1
        if notice_count == 0:
            raise RuntimeError(f"No original notices found for {package}")
        (folder / "PROVENANCE.txt").write_text(
            f"MSYS2 package: {package} {version}\nRecipe: {recipe_url}\n"
            f"Original upstream source: {url}\nSHA-256: {expected_hash}\n"
            "Original notice bytes and complete upstream source are preserved.\n"
            "The source includes per-file notices; it is not the complete patched MSYS2 build tree.\n",
            encoding="utf-8",
        )
        print(f"Prepared {package} {version}: {notice_count} original notice files")


if __name__ == "__main__":
    main()
