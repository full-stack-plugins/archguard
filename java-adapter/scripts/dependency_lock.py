"""固定实际 Maven 依赖原件；仅从明确本地仓库读取，不解析网络或安装工具。"""
import argparse
import hashlib
import json
from pathlib import Path

ARTIFACTS = (
    "com/tngtech/archunit/archunit/1.5.1/archunit-1.5.1",
    "org/slf4j/slf4j-api/2.0.19/slf4j-api-2.0.19",
    "com/fasterxml/jackson/core/jackson-databind/2.20.1/jackson-databind-2.20.1",
    "com/fasterxml/jackson/core/jackson-core/2.20.1/jackson-core-2.20.1",
    "com/fasterxml/jackson/core/jackson-annotations/2.20/jackson-annotations-2.20",
    "junit/junit/4.13.2/junit-4.13.2",
    "org/hamcrest/hamcrest-core/1.3/hamcrest-core-1.3",
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    locked = {
        f"{artifact}.{extension}": "sha256:" + hashlib.sha256(
            (args.repository / f"{artifact}.{extension}").read_bytes()
        ).hexdigest()
        for artifact in ARTIFACTS
        for extension in ("jar", "pom")
    }
    destination = Path(__file__).resolve().parents[1] / "dependencies.lock.json"
    contents = json.dumps({"schemaVersion": 1, "artifacts": locked}, indent=2) + "\n"
    if args.check:
        if destination.read_text(encoding="utf-8") != contents:
            raise SystemExit("dependency lock differs from actual artifacts")
        print(f"verified {len(locked)} immutable dependency artifacts")
    else:
        destination.write_text(contents, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
