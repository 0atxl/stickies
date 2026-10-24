#!/usr/bin/env bash
# Run as an unprivileged user in a fresh Arch builder with dependencies installed.
set -euo pipefail
cd "$(dirname "$0")/.."

export CARGO_HOME="/home/builder/.cargo"
SOURCE_DATE_EPOCH=$(git show -s --format=%ct HEAD)
export SOURCE_DATE_EPOCH
variant="${1:-standard}"
case "$variant" in standard|mcp) ;; *) echo "Unknown variant: $variant" >&2; exit 1 ;; esac
build_dir="$PWD/target/ci-arch-$variant"
artifact_dir="$PWD/dist/arch/$variant"
if [[ -e "$build_dir" || -e "$artifact_dir" ]]; then
    echo 'CI output already exists; use a fresh checkout.' >&2
    exit 1
fi

cargo fetch --locked --target x86_64-unknown-linux-gnu
python3 packaging/prepare-release.py --variant "$variant" --output "$build_dir"
cd "$build_dir"
makepkg --nocheck --nosign --log

shopt -s nullglob
packages=(stickies-*-x86_64.pkg.tar.zst)
sources=(stickies-*-src.tar.gz)
if (( ${#packages[@]} != 1 || ${#sources[@]} != 1 )); then
    echo 'Expected exactly one package and one matching source archive.' >&2
    exit 1
fi
mkdir -p "$artifact_dir"
cp "${packages[0]}" "${sources[0]}" PKGBUILD SOURCE-INFO.txt PREPARATION.txt "$artifact_dir/"
bsdtar -xOf "${packages[0]}" .BUILDINFO > "$artifact_dir/BUILDINFO"
bsdtar -xOf "${packages[0]}" .PKGINFO > "$artifact_dir/PKGINFO"
pacman -Q > "$artifact_dir/BUILD-PACKAGES.txt"
cd "$artifact_dir"
sha256sum ./* > SHA256SUMS
sha256sum --check SHA256SUMS
