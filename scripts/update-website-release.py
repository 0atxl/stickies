#!/usr/bin/env python3
"""Update the static website from GitHub's latest stable release JSON."""

import argparse
from collections import Counter
from html import escape
import json
from pathlib import Path
import re


ROOT = Path(__file__).resolve().parent.parent
REPOSITORY = 'https://github.com/0atxl/stickies'


def version_parts(tag):
    if not re.fullmatch(r'v[0-9]+\.[0-9]+(?:\.[0-9]+)?', tag):
        raise ValueError('Expected a stable numeric tag, for example v0.10.1')
    parts = tuple(int(part) for part in tag[1:].split('.'))
    return parts + (0,) * (3 - len(parts))


def update(release):
    if release.get('draft') is not False or release.get('prerelease') is not False:
        raise ValueError('Only published stable releases can update the website')
    tag = release['tag_name']
    parts = version_parts(tag)
    version = '.'.join(str(part) for part in parts)
    release_url = f'{REPOSITORY}/releases/tag/{tag}'
    if release['html_url'] != release_url or not release.get('published_at'):
        raise ValueError('Expected a published release from 0atxl/stickies')

    def asset(pattern):
        matches = [item for item in release['assets'] if re.fullmatch(pattern, item['name'])]
        if len(matches) != 1:
            raise ValueError(f'Expected exactly one release asset matching {pattern}')
        item = matches[0]
        if item.get('state') != 'uploaded' or item.get('size', 0) <= 0:
            raise ValueError(f'Release asset is not ready: {item["name"]}')
        if item['browser_download_url'] != f'{REPOSITORY}/releases/download/{tag}/{item["name"]}':
            raise ValueError(f'Unexpected download URL for {item["name"]}')
        return item

    windows = asset(r'stickies-windows-x86_64\.zip')
    arch = asset(rf'stickies-{re.escape(version)}-[0-9]+-x86_64\.pkg\.tar\.zst')
    checksums = asset(r'SHA256SUMS')
    hrefs = {
        'release': release_url,
        'windows': windows['browser_download_url'],
        'arch': arch['browser_download_url'],
        'checksums': checksums['browser_download_url'],
        'source': f'{REPOSITORY}/tree/{tag}#requirements',
    }
    path = ROOT / 'website' / 'index.html'
    original = path.read_text(encoding='utf-8')
    baseline = re.findall(r'\bdata-demo-since="([^"]+)"', original)
    if len(baseline) != 1:
        raise ValueError('Expected one explicit demo appearance baseline')
    # This baseline is an editorial decision, never inferred from a new release.
    matches_demo = baseline[0] != 'unreleased' and parts >= version_parts(baseline[0])
    description = 'Simplified demo of Playful Clean.' if matches_demo else 'Preview of the upcoming appearance.'
    texts = {
        'tag': tag,
        'version': version,
        'arch-command': f'sudo pacman -U ./{arch["name"]}',
        'disclosure': f'{description} Downloads are {tag}.',
    }
    seen_hrefs = Counter()
    seen_texts = Counter()

    def replace_href(match):
        element = match[0]
        key = match[1]
        seen_hrefs[key] += 1
        element, count = re.subn(
            r'(?<![\w-])href="[^"]*"',
            lambda _: f'href="{escape(hrefs[key], quote=True)}"', element,
        )
        if count != 1:
            raise ValueError(f'Expected one href for {key}')
        return element

    def replace_text(match):
        key = match['key']
        seen_texts[key] += 1
        return match['open'] + escape(texts[key]) + match['close']

    updated = re.sub(r'<a\b[^>]*\bdata-release-href="([^"]+)"[^>]*>', replace_href, original)
    updated = re.sub(
        r'(?P<open><(?P<tag>span|code|p)\b[^>]*\bdata-release-text="(?P<key>[^"]+)"[^>]*>)'
        r'[^<]*(?P<close></(?P=tag)>)', replace_text, updated,
    )
    if seen_hrefs != Counter(release=3, windows=1, arch=1, checksums=1, source=1):
        raise ValueError('Website release link markers changed; review the updater before writing')
    if seen_texts != Counter({'tag': 2, 'version': 1, 'arch-command': 1, 'disclosure': 1}):
        raise ValueError('Website release text markers changed; review the updater before writing')
    # Validate the full update before replacing the page, leaving it intact on failure.
    if updated != original:
        path.write_text(updated, encoding='utf-8')
        print(f'Updated website/index.html to {tag}')
    else:
        print(f'Website already uses {tag}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release-json', type=Path, required=True,
                        help='JSON returned by GET /repos/0atxl/stickies/releases/latest')
    args = parser.parse_args()
    try:
        update(json.loads(args.release_json.read_text(encoding='utf-8')))
    except (KeyError, TypeError, ValueError, OSError) as error:
        parser.exit(1, f'Website release update stopped: {error}\n')


if __name__ == '__main__':
    main()
