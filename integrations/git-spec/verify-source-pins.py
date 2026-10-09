#!/usr/bin/env python3
"""Check exact local sources before running optional integration tests."""
import hashlib
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent
for name, pin in json.loads((root / 'source-pins.json').read_text()).items():
    source = root.parent.parent.parent / name
    actual = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    if actual != pin['commit']:
        raise SystemExit(f'{name}: expected {pin["commit"]}, found {actual}')
    for file, expected in pin['files'].items():
        if hashlib.sha256((source / file).read_bytes()).hexdigest() != expected:
            raise SystemExit(f'{name}: modified source {file}')
    print(f'{name}: {actual}, {len(pin["files"])} source files verified')
