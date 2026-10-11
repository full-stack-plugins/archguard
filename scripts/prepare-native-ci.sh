#!/usr/bin/env bash
# Reproduce the existing Linux x86_64 provider pins; never rewrite them from PATH.
set -euo pipefail
repo_root=$(cd "$(dirname "$0")/.." && pwd)
tool_root=${1:?usage: prepare-native-ci.sh ABSOLUTE_TOOL_DIRECTORY}
[[ "$tool_root" = /* ]] || { echo 'tool directory must be absolute' >&2; exit 2; }
[[ $(uname -s) = Linux && $(uname -m) = x86_64 ]] || { echo 'only Linux x86_64 is qualified' >&2; exit 2; }
mkdir -p "$tool_root"
download_root=$(mktemp -d)
trap 'rm -rf "$download_root"' EXIT
fetch() {
  local url=$1 sha=$2 dest=$3
  curl --fail --location --retry 3 --proto '=https' --tlsv1.2 "$url" --output "$dest"
  echo "$sha  $dest" | sha256sum --check --status
}
fetch 'https://deb.debian.org/debian/pool/main/o/openjdk-21/openjdk-21-jdk-headless_21.0.12.1+1-1~deb13u1_amd64.deb' \
  f3abafb6c644b03df042824e707cd211ea33254761d7f7b75be3f8dc0df97c7a "$download_root/jdk.deb"
fetch 'https://deb.debian.org/debian/pool/main/o/openjdk-21/openjdk-21-jre-headless_21.0.12.1+1-1~deb13u1_amd64.deb' \
  e95f36193e45464ac758e5436f940bf3eaecc77de67188275a42104f58aa7674 "$download_root/jre.deb"
dpkg-deb --extract "$download_root/jdk.deb" "$tool_root/jdk"
dpkg-deb --extract "$download_root/jre.deb" "$tool_root/jdk"
# Debian configuration links normally target /etc. Keep extracted packages
# self-contained without reading or modifying the runner's system configuration.
python3 - "$tool_root/jdk" <<'PYCONFIG'
import os
from pathlib import Path
import sys
root = Path(sys.argv[1]).resolve()
home = root / 'usr/lib/jvm/java-21-openjdk-amd64'
for link in home.rglob('*'):
    if link.is_symlink():
        target = link.readlink()
        if str(target).startswith('/etc/java-21-openjdk/'):
            packaged = root / target.relative_to('/')
            if not packaged.is_file():
                raise SystemExit(f'missing packaged configuration: {target}')
            link.unlink()
            link.symlink_to(os.path.relpath(packaged, link.parent))
PYCONFIG
fetch 'https://nodejs.org/dist/v24.19.0/node-v24.19.0-linux-x64.tar.xz' \
  14b342e71204f811bde6153be8e04b62aef63c236fef92b55f9c83154b409647 "$download_root/node.tar.xz"
tar -xJf "$download_root/node.tar.xz" -C "$tool_root"
export ARCHGUARD_TEST_JDK="$tool_root/jdk/usr/lib/jvm/java-21-openjdk-amd64"
export ARCHGUARD_TEST_NODE="$tool_root/node-v24.19.0-linux-x64/bin/node"
export PATH="$(dirname "$ARCHGUARD_TEST_NODE"):$PATH"
npm ci --cache "$tool_root/npm-cache" --ignore-scripts --no-audit --no-fund --prefix "$repo_root/fixtures/languages/typescript"
python3 "$repo_root/scripts/verify-native-tools.py"
if [[ -n ${GITHUB_ENV:-} ]]; then
  printf 'ARCHGUARD_TEST_JDK=%s\nARCHGUARD_TEST_NODE=%s\n' "$ARCHGUARD_TEST_JDK" "$ARCHGUARD_TEST_NODE" >> "$GITHUB_ENV"
fi
