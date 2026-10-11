#!/usr/bin/env python3
"""Validate selected installed tools against committed profiles; emit actual identity."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verify():
    jdk = Path(os.environ.get('ARCHGUARD_TEST_JDK') or os.environ['JAVA_HOME'])
    node = Path(os.environ.get('ARCHGUARD_TEST_NODE') or shutil.which('node') or '')
    compiler = ROOT / 'fixtures/languages/typescript/node_modules/typescript'
    java_pin = json.loads((ROOT / 'fixtures/languages/java/jdk-profile.json').read_text())
    ts_pin = json.loads((ROOT / 'fixtures/languages/typescript/tool-profile.json').read_text())
    actual_java = {name: digest(jdk / name) for name in java_pin['files']}
    actual_ts = {name: digest(compiler / name) for name in ts_pin['files']}
    actual_node = digest(node)
    if actual_java != java_pin['files'] or actual_ts != ts_pin['files'] or actual_node != ts_pin['node_sha256']:
        raise SystemExit('native tool bytes do not match the committed provider profiles')
    javac_version = subprocess.check_output([str(jdk / 'bin/javac'), '-version'], text=True).strip()
    node_version = subprocess.check_output([str(node), '--version'], text=True).strip()
    if javac_version != 'javac ' + java_pin['version'] or node_version != ts_pin['node_version']:
        raise SystemExit('native tool versions do not match the committed provider profiles')
    print(json.dumps({'javac': javac_version, 'node': node_version,
                      'jdk_files': actual_java, 'node_sha256': actual_node,
                      'typescript_files': actual_ts}, sort_keys=True))


if __name__ == '__main__':
    verify()
