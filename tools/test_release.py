"""Check version increments and detection of platform/lockfile/plist drift."""
from pathlib import Path
import shutil
import re
import tempfile
import unittest
from unittest.mock import patch

import release


class ReleaseVersions(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='dragon-release-')
        root = Path(self.temp.name)
        originals = [*release.MANIFESTS, *release.LOCKS, release.PLIST,
                     release.ROOT / 'crates/pet-desktop/src/updates.rs',
                     release.ROOT / 'macos/crates/pet-desktop/src/updates.rs']
        for source in originals:
            destination = root / source.relative_to(release.ROOT)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
        self.replacements = patch.multiple(
            release, ROOT=root, MANIFESTS=[root / 'Cargo.toml', root / 'macos/Cargo.toml'],
            LOCKS=[root / 'Cargo.lock', root / 'macos/Cargo.lock'], PLIST=root / 'macos/Info.plist')
        self.replacements.start()

    def tearDown(self):
        self.replacements.stop()
        self.temp.cleanup()

    def test_increment_and_synchronize_all_metadata(self):
        release.sync('2.9.12')
        self.assertEqual(release.next_version('patch'), '2.9.13')
        self.assertEqual(release.next_version('minor'), '2.10.0')
        self.assertEqual(release.next_version('major'), '3.0.0')
        release.sync(release.next_version('minor'))
        self.assertEqual(release.check(), '2.10.0')

    def test_reject_platform_and_lockfile_drift(self):
        release.sync('2.0.0')
        for path in [release.MANIFESTS[1], release.LOCKS[1], release.PLIST]:
            original = path.read_text(encoding='utf-8')
            changed = re.sub(r'(name = "pet-core"\nversion = ")2.0.0', r'\g<1>2.0.1', original) if path.suffix == '.lock' else original.replace('2.0.0', '2.0.1', 1)
            path.write_text(changed, encoding='utf-8')
            with self.assertRaises(AssertionError):
                release.check()
            path.write_text(original, encoding='utf-8')


if __name__ == '__main__':
    unittest.main()
