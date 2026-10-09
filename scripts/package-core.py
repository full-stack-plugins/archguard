#!/usr/bin/env python3
"""Build a self-contained source archive, excluding the optional Git/spec package."""
import hashlib
import json
import pathlib
import shutil
import sys
import tarfile
import tempfile

source = pathlib.Path(__file__).resolve().parent.parent
output = pathlib.Path(sys.argv[1]).resolve()
if output.is_relative_to(source):
    raise SystemExit('archive output must be outside the source checkout')
with tempfile.TemporaryDirectory(prefix='archguard-package-') as directory:
    root = pathlib.Path(directory) / 'core'
    def ignore(path, names):
        excluded = {'.git', 'target', '__pycache__', '.bin'}
        if pathlib.Path(path) == source / 'integrations':
            excluded.add('git-spec')
        for name in set(names) - excluded:
            if (pathlib.Path(path) / name).is_symlink():
                raise SystemExit(f'symlink not allowed in core distribution: {pathlib.Path(path) / name}')
        return excluded.intersection(names)
    shutil.copytree(source, root, ignore=ignore, symlinks=True)
    manifest = root / 'Cargo.toml'
    text = manifest.read_text()
    old = 'path = "../guardengine", version = "=0.1.0"'
    if old in text:
        manifest.write_text(text.replace(old, 'path = "vendor/guardengine", version = "=0.1.0"'))
    elif 'path = "vendor/guardengine", version = "=0.1.0"' not in text:
        raise SystemExit('unexpected GuardEngine source; refusing to package')
    for file in root.rglob('*'):
        if file.is_symlink():
            raise SystemExit(f'symlink not allowed in core distribution: {file.relative_to(root)}')
    engine = root / 'vendor/guardengine'
    pin = json.loads((engine / 'source-hashes.json').read_text())
    expected_files = set(pin['files']) | {'source-hashes.json'}
    actual_files = {str(file.relative_to(engine)) for file in engine.rglob('*') if file.is_file()}
    if actual_files != expected_files:
        raise SystemExit('fixed GuardEngine file closure mismatch: '
                         f'extra={sorted(actual_files - expected_files)}, '
                         f'missing={sorted(expected_files - actual_files)}')
    for name, expected in pin['files'].items():
        if hashlib.sha256((engine / name).read_bytes()).hexdigest() != expected:
            raise SystemExit(f'fixed GuardEngine source mismatch: {name}')
    checksums = []
    for file in sorted(root.rglob('*')):
        if file.is_file() and file.name != 'SHA256SUMS':
            checksums.append(f'{hashlib.sha256(file.read_bytes()).hexdigest()}  {file.relative_to(root)}')
    (root / 'SHA256SUMS').write_text('\n'.join(checksums) + '\n')
    with tarfile.open(output, 'w:gz') as archive:
        archive.add(root, arcname='.')
print(output)
