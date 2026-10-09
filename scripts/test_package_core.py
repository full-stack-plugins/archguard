#!/usr/bin/env python3
"""Regression tests for source-package input containment and the fixed GE closure."""
import pathlib
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

SOURCE = pathlib.Path(__file__).resolve().parent.parent


class PackageBoundary(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.root = self.base / 'source'
        (self.root / 'scripts').mkdir(parents=True)
        shutil.copy(SOURCE / 'scripts/package-core.py', self.root / 'scripts/package-core.py')
        shutil.copytree(SOURCE / 'vendor/guardengine', self.root / 'vendor/guardengine')
        shutil.copy(SOURCE / 'Cargo.toml', self.root / 'Cargo.toml')
        self.archive = self.base / 'core.tar.gz'

    def package(self):
        return subprocess.run([sys.executable, str(self.root / 'scripts/package-core.py'),
                               str(self.archive)], capture_output=True, text=True)

    def rejected(self, message):
        result = self.package()
        self.assertNotEqual(result.returncode, 0, 'unsafe input was packaged')
        self.assertIn(message, result.stderr)
        self.assertFalse(self.archive.exists())

    def test_external_file_symlink_is_rejected_before_archive_creation(self):
        external = self.base / 'sibling.rs'
        external.write_text('external sibling source')
        (self.root / 'external.rs').symlink_to(external)
        self.rejected('symlink not allowed')

    def test_external_directory_symlink_is_rejected_before_archive_creation(self):
        external = self.base / 'sibling'
        external.mkdir()
        (external / 'lib.rs').write_text('external sibling source')
        (self.root / 'external').symlink_to(external, target_is_directory=True)
        self.rejected('symlink not allowed')

    def test_unlisted_guardengine_build_script_is_rejected(self):
        (self.root / 'vendor/guardengine/build.rs').write_text('fn main() {}')
        self.rejected('fixed GuardEngine file closure mismatch')

    def test_unlisted_guardengine_source_is_rejected(self):
        (self.root / 'vendor/guardengine/src/unlisted.rs').write_text('// extra source')
        self.rejected('fixed GuardEngine file closure mismatch')

    def test_changed_guardengine_lockfile_is_rejected(self):
        with (self.root / 'vendor/guardengine/Cargo.lock').open('a') as lock:
            lock.write('\n# changed after source pin\n')
        self.rejected('fixed GuardEngine source mismatch: Cargo.lock')

    def test_missing_guardengine_source_is_rejected(self):
        (self.root / 'vendor/guardengine/src/analyzer.rs').unlink()
        self.rejected('fixed GuardEngine file closure mismatch')

    def test_unchanged_fixed_source_packages_with_internal_dependency(self):
        result = self.package()
        self.assertEqual(result.returncode, 0, result.stderr)
        with tarfile.open(self.archive) as archive:
            manifest = archive.extractfile('./Cargo.toml').read().decode()
            self.assertIn('path = "vendor/guardengine"', manifest)
            self.assertTrue(all(not member.issym() and not member.islnk()
                                for member in archive.getmembers()))


if __name__ == '__main__':
    unittest.main()
