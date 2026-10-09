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
pin = json.loads((source / 'vendor/guardengine/source-hashes.json').read_text())
for name, expected in pin['files'].items():
    if hashlib.sha256((source / 'vendor/guardengine' / name).read_bytes()).hexdigest() != expected:
        raise SystemExit(f'fixed GuardEngine source mismatch: {name}')
with tempfile.TemporaryDirectory(prefix='archguard-package-') as directory:
    root = pathlib.Path(directory) / 'core'
    def ignore(path, names):
        excluded = {'.git', 'target', '__pycache__', '.bin'}
        if pathlib.Path(path) == source / 'integrations':
            excluded.add('git-spec')
        return excluded.intersection(names)
    shutil.copytree(source, root, ignore=ignore)
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
    checksums = []
    for file in sorted(root.rglob('*')):
        if file.is_file() and file.name != 'SHA256SUMS':
            checksums.append(f'{hashlib.sha256(file.read_bytes()).hexdigest()}  {file.relative_to(root)}')
    (root / 'SHA256SUMS').write_text('\n'.join(checksums) + '\n')
    with tarfile.open(output, 'w:gz') as archive:
        archive.add(root, arcname='.')
print(output)
