"""Manage the shared Rust release version. Run from any working directory."""
import argparse
from pathlib import Path
import re
import subprocess
import tomllib
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
MANIFESTS = [ROOT / 'Cargo.toml', ROOT / 'macos/Cargo.toml']
LOCKS = [ROOT / 'Cargo.lock', ROOT / 'macos/Cargo.lock']
PLIST = ROOT / 'macos/Info.plist'


def version():
    return tomllib.loads(MANIFESTS[0].read_text(encoding="utf-8"))['workspace']['package']['version']


def plist_versions(value):
    major, minor, patch = re.match(r'^(\d+)\.(\d+)\.(\d+)', value).groups()
    return {'CFBundleShortVersionString': f'{major}.{minor}.{patch}',
            'CFBundleVersion': f'{major}.{minor}.{patch}', 'LPPReleaseVersion': value}


def sync(value):
    if not re.fullmatch(r'\d+\.\d+\.\d+', value):
        raise ValueError('Releases use stable SemVer: major.minor.patch')
    for path in MANIFESTS:
        source = path.read_text(encoding="utf-8")
        source, count = re.subn(r'(?m)^version = "[^"]+"$', f'version = "{value}"', source, count=1)
        assert count == 1, path
        path.write_text(source, encoding="utf-8", newline='\n')
    for path in LOCKS:
        source = path.read_text(encoding="utf-8")
        source, count = re.subn(r'(name = "pet-(?:core|desktop)"\nversion = ")[^"]+',
                                lambda match: match[1] + value, source)
        assert count == 2, path
        path.write_text(source, encoding="utf-8", newline='\n')
    source = PLIST.read_text(encoding="utf-8")
    for key, entry in plist_versions(value).items():
        source, count = re.subn(r'(<key>' + key + r'</key>\s*<string>)[^<]+',
                                lambda match: match[1] + entry, source)
        assert count == 1, key
    PLIST.write_text(source, encoding="utf-8", newline='\n')


def check():
    expected = version()
    for path in MANIFESTS:
        assert tomllib.loads(path.read_text(encoding="utf-8"))['workspace']['package']['version'] == expected, path
    for path in LOCKS:
        packages = tomllib.loads(path.read_text(encoding="utf-8"))['package']
        own = [entry for entry in packages if entry['name'] in ('pet-core', 'pet-desktop')]
        assert len(own) == 2 and all(entry['version'] == expected for entry in own), path
    entries = list(ET.parse(PLIST).getroot().find('dict'))
    values = {entries[i].text: entries[i+1].text for i in range(0, len(entries), 2)}
    for key, entry in plist_versions(expected).items():
        assert values[key] == entry, f'{key}: {values[key]} != {entry}'
    for prefix in ('', 'macos/'):
        source = (ROOT / prefix / 'crates/pet-desktop/src/updates.rs').read_text(encoding='utf-8')
        assert 'pub const VERSION: &str = env!("CARGO_PKG_VERSION")' in source
    return expected


def next_version(part):
    values = list(map(int, re.match(r'^(\d+)\.(\d+)\.(\d+)', version()).groups()))
    index = {'major': 0, 'minor': 1, 'patch': 2}[part]
    values[index] += 1
    for lower in range(index+1, 3):
        values[lower] = 0
    return '.'.join(map(str, values))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('check')
    sub.add_parser('version')
    bump = sub.add_parser('bump')
    bump.add_argument('part', choices=['patch', 'minor', 'major'], default='patch', nargs='?')
    args = parser.parse_args()
    if args.command == 'bump':
        value = next_version(args.part)
        tags = subprocess.check_output(['git', 'tag', '--list', f'v{value}'], cwd=ROOT, text=True).strip()
        if tags:
            raise SystemExit(f'Tag v{value} already exists')
        sync(value)
        print(check())
    elif args.command == 'check':
        print(f'Shared release version OK: {check()}')
    else:
        print(version())


if __name__ == '__main__':
    main()
