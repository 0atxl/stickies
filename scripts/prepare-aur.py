#!/usr/bin/env python3
"""Prepare AUR recipes from verified GitHub release artifacts; never publish."""
import argparse
import hashlib
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--tag', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'v[0-9]+(?:\.[0-9]+){1,2}', args.tag):
        parser.error('expected a numeric release tag such as v0.10')
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    tag_version = args.tag[1:].split('.')
    if '.'.join(tag_version + ['0'] * (3 - len(tag_version))) != version:
        parser.error('tag and manifest version must agree')
    if args.output.exists():
        parser.error('output already exists; choose a new directory')
    artifacts = args.artifacts.resolve()
    for line in (artifacts / 'SHA256SUMS').read_text().splitlines():
        digest, name = line.split(maxsplit=1)
        path = artifacts / name.removeprefix('*').removeprefix('./')
        if path.parent != artifacts or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            parser.error('invalid artifact checksum or filename')
    recipe = (artifacts / 'PKGBUILD').read_text()
    if f'pkgver={version}\n' not in recipe or 'pkgrel=1\n' not in recipe:
        parser.error('unexpected source recipe version or release number')
    base = f'https://github.com/0atxl/stickies/releases/download/{args.tag}'
    source_name = f'stickies-{version}-src.tar.gz'
    binary_name = f'stickies-{version}-1-x86_64.pkg.tar.zst'
    source_hash = hashlib.sha256((artifacts / source_name).read_bytes()).hexdigest()
    binary_hash = hashlib.sha256((artifacts / binary_name).read_bytes()).hexdigest()
    if f"sha256sums=('{source_hash}')" not in recipe:
        parser.error('source recipe hash does not match the archive')
    source_line = 'source=("stickies-${pkgver}-src.tar.gz")'
    if recipe.count(source_line) != 1:
        parser.error('unexpected source recipe layout')
    recipe = recipe.replace(source_line, f'source=("{base}/stickies-${{pkgver}}-src.tar.gz")')
    recipe = re.sub(r'\A# Source template.*?# with[^\n]*\n', '', recipe, flags=re.S)
    header = recipe.split('\nprepare()', 1)[0]
    header = header.replace('pkgname=stickies\n', 'pkgname=stickies-bin\n')
    header = re.sub(r'^makedepends=.*\n', '', header, flags=re.M)
    header = re.sub(r'^_target=.*\n', '', header, flags=re.M)
    header = re.sub(r'^source=.*$', f'source=("{base}/{binary_name}")', header, flags=re.M)
    header = re.sub(r'^sha256sums=.*$', f"sha256sums=('{binary_hash}')", header, flags=re.M)
    binary_recipe = header + '''\nprovides=("stickies=$pkgver")
conflicts=('stickies')

package() {
    cp -a "$srcdir/usr" "$pkgdir/"
}
'''
    for name, text in [('stickies', recipe), ('stickies-bin', binary_recipe)]:
        directory = args.output / name
        directory.mkdir(parents=True)
        (directory / 'PKGBUILD').write_text(text)
        subprocess.run(['bash', '-n', str(directory / 'PKGBUILD')], check=True)
        info = subprocess.check_output(['makepkg', '--printsrcinfo'], cwd=directory, text=True)
        (directory / '.SRCINFO').write_text(info)
        print(f'Prepared {directory}')
    print('No packages were built or published. Verify the public release URLs before pushing to AUR.')


if __name__ == '__main__':
    main()
