"""为既有工具/专用缓存生成真实字节码测试配置；不联网、不安装、不启动进程。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import xml.etree.ElementTree as ET

from dependency_lock import ARTIFACTS


def plain(path, directory=False):
    """拒绝链接/reparse 与特殊文件，返回已有绝对规范路径。"""
    if not path.is_absolute():
        raise ValueError(f"absolute path required: {path}")
    for ancestor in (path, *path.parents):
        meta = ancestor.lstat()
        if stat.S_ISLNK(meta.st_mode) or getattr(meta, "st_file_attributes", 0) & 0x400:
            raise ValueError(f"linked/reparse path rejected: {ancestor}")
    meta = path.stat()
    if not (stat.S_ISDIR(meta.st_mode) if directory else stat.S_ISREG(meta.st_mode)):
        raise ValueError(f"ordinary {'directory' if directory else 'file'} required: {path}")
    return path.resolve(strict=True)


def inventory(root, maximum):
    """按有界原生路径清单读取全部普通文件，不跟随目录链接。"""
    files = []
    count = 0
    for directory, directories, names in os.walk(root, followlinks=False):
        count += 1 + len(names)
        if count > maximum or len(Path(directory).relative_to(root).parts) > 64:
            raise ValueError("tool/cache traversal budget exceeded")
        for name in directories:
            plain(Path(directory) / name, True)
        for name in names:
            files.append(plain(Path(directory) / name))
    return sorted(files)


def digest(path, budget=512 * 1024 * 1024):
    """计算有界真实原件摘要，不从文件名或旧配置复用摘要。"""
    result = hashlib.sha256()
    total = 0
    with plain(path).open("rb") as stream:
        while chunk := stream.read(65536):
            total += len(chunk)
            if total > budget:
                raise ValueError(f"tool file exceeds budget: {path}")
            result.update(chunk)
    return "sha256:" + result.hexdigest()


def verify_test_reports(directory):
    """可选 CI 断言：真实 Surefire 报告至少十九测试、无失败或跳过。"""
    directory = plain(directory, True)
    reports = sorted(directory.glob("TEST-*.xml"))
    if not reports:
        raise ValueError("actual Surefire XML reports required")
    tests = 0
    for report in reports:
        root = ET.fromstring(plain(report).read_bytes())
        if root.tag != "testsuite":
            raise ValueError("unsupported Surefire report")
        tests += int(root.attrib["tests"])
        if any(int(root.attrib.get(key, "0")) for key in ("failures", "errors", "skipped")):
            raise ValueError("Java tests failed, errored or skipped")
    if tests < 19:
        raise ValueError(f"at least 19 actual Java tests required; observed {tests}")
    return tests


def prepare(args):
    """仅绑定显式已存在目录；返回独立本地声明，绝不颁发批准或信任。"""
    java_home = plain(args.java_home, True)
    maven = plain(args.maven_home, True)
    repository = plain(args.repository, True)
    home = plain(args.home, True)
    temporary = plain(args.temporary_directory, True)
    adapter = plain(args.adapter_jar)
    java = plain(java_home / "bin" / ("java.exe" if os.name == "nt" else "java"))
    plain(maven / "bin/m2.conf")
    if len(list((maven / "boot").glob("plexus-classworlds-*.jar"))) != 1:
        raise ValueError("exactly one Maven classworlds bootstrap required")
    plain(maven / "lib", True)

    # 锁文件由 Java 作者维护；准备器只能验证十四原件，不能重写锁以迎合当前缓存。
    lock = json.loads((Path(__file__).resolve().parents[1] / "dependencies.lock.json").read_text(encoding="utf-8"))
    observed = {f"{artifact}.{extension}": digest(repository / f"{artifact}.{extension}")
                for artifact in ARTIFACTS for extension in ("jar", "pom")}
    if lock != {"schemaVersion": 1, "artifacts": observed}:
        raise ValueError("all 14 dependency lock artifacts must match")
    classpath = [adapter, *(plain(repository / f"{artifact}.jar") for artifact in ARTIFACTS[:5])]
    pinned = {java, *classpath}
    # 附加固定已知 JDK 执行原件，不把 Java 程序摘要冒充整个 JDK 镜像证明。
    runtime = [java_home / "release", java_home / "lib/modules",
               java_home / ("bin/server/jvm.dll" if os.name == "nt" else "lib/server/libjvm.so")]
    pinned.update(plain(path) for path in runtime)
    maven_files = inventory(maven, 20000)
    if any(path.stat().st_size > 64 * 1024 * 1024 for path in maven_files):
        raise ValueError("Maven distribution file exceeds 64 MiB")
    if sum(path.stat().st_size for path in maven_files) > 512 * 1024 * 1024:
        raise ValueError("Maven distribution exceeds 512 MiB")
    pinned.update(maven_files)
    pinned.update(path for path in inventory(repository, 50000) if path.suffix in (".jar", ".pom"))
    if len(pinned) > 8192:
        raise ValueError("tool pin count exceeds adapter budget")
    environment = {"GIT_CEILING_DIRECTORIES": "{{subject_parent}}", "TEMP": str(temporary), "TMP": str(temporary), "TMPDIR": str(temporary)}
    if os.name == "nt":
        system_root = os.environ.get("SystemRoot")
        if not system_root:
            raise ValueError("explicit existing SystemRoot required on Windows")
        environment["SystemRoot"] = str(plain(Path(system_root), True))
    config = {
        "schemaVersion": 1, "repositoryId": "local-demo", "taskId": "REQ-A01",
        "worktreeId": "local-worktree", "verifierId": "archunit-java",
        "javaExecutable": str(java), "classpath": [str(path) for path in classpath],
        "mavenHome": str(maven), "repository": str(repository), "home": str(home),
        "temporaryDirectory": str(temporary),
        "toolArtifacts": [{"path": str(path), "sha256": digest(path)} for path in sorted(pinned)],
        "modules": [{"path": "", "expectedTypes": []},
                    {"path": "infra", "expectedTypes": ["demo.infra.SqlStore"]},
                    {"path": "domain", "expectedTypes": ["demo.domain.Order"]}],
        "environment": environment,
        "budget": {"timeoutMs": 120000, "maxOutputBytes": 1048576, "maxConcurrency": 1},
    }
    if len((json.dumps(config, indent=2) + "\n").encode("utf-8")) > 1024 * 1024:
        raise ValueError("configuration exceeds Rust input budget")
    return config


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("java-home", "maven-home", "repository", "adapter-jar", "home", "temporary-directory", "output"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--test-reports", type=Path)
    args = parser.parse_args()
    try:
        if not args.output.is_absolute():
            raise ValueError("absolute output required")
        plain(args.output.parent, True)
        if args.test_reports:
            print(f"verified {verify_test_reports(args.test_reports)} actual Java tests without skips")
        config = prepare(args)
        # 独占创建，不能覆盖旧配置；被检查的工具、缓存、源码全部只读。
        with args.output.open("x", encoding="utf-8", newline="\n") as stream:
            stream.write(json.dumps(config, indent=2) + "\n")
        print(f"prepared {len(config['toolArtifacts'])} actual tool pins; 14 locked artifacts verified; local_unverified")
    except (OSError, ValueError, KeyError, ET.ParseError) as error:
        parser.exit(2, f"configuration preparation failed: {error}\n")


if __name__ == "__main__":
    main()
