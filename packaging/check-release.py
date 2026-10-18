#!/usr/bin/env python3
"""Inspect a generated release archive and run its packaged native executable."""
import argparse
import hashlib
from pathlib import Path
import plistlib
import subprocess
import tarfile
import tempfile
import zipfile


def check(archive, version, run):
    expected = archive.with_name(archive.name + ".sha256").read_text().split()
    with archive.open("rb") as stream:
        actual = hashlib.file_digest(stream, "sha256").hexdigest()
    if expected != [actual, archive.name]:
        raise SystemExit("Release checksum does not match its archive")
    with tempfile.TemporaryDirectory(prefix="omasend-release-check-") as temporary:
        root = Path(temporary)
        if archive.suffix == ".zip":
            with zipfile.ZipFile(archive) as bundle:
                bundle.extractall(root)
            executable = root / "omasend.exe"
        else:
            with tarfile.open(archive) as bundle:
                bundle.extractall(root, filter="data")
            if "apple-darwin" in archive.name:
                app = root / "OmaSend.app"
                executable = app / "Contents/MacOS/omasend"
                with (app / "Contents/Info.plist").open("rb") as stream:
                    info = plistlib.load(stream)
                if (info.get("CFBundleExecutable"), info.get("CFBundleShortVersionString")) != ("omasend", version):
                    raise SystemExit("Bundle executable or version does not match the release")
                if info.get("LSMinimumSystemVersion") != "14.0":
                    raise SystemExit("AppKit release requires macOS 14 or later")
                subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
            else:
                executable = root / "omasend"
                for asset in ("packaging/omasend.desktop", "assets/omasend.png"):
                    if not (root / asset).is_file():
                        raise SystemExit(f"Release archive is missing {asset}")
        if not executable.is_file():
            raise SystemExit("Release archive is missing its executable")
        if archive.suffix != ".zip" and not executable.stat().st_mode & 0o111:
            raise SystemExit("Packaged binary is not executable")
        if run:
            result = subprocess.run([str(executable), "--version"], check=True,
                                    text=True, capture_output=True, timeout=30)
            if result.stdout.strip() != f"omasend {version}":
                raise SystemExit(f"Unexpected packaged version: {result.stdout!r}")
    print(f"Verified {archive.name}" + (" (native executable ran)" if run else " (cross-compiled archive)"))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--no-run", action="store_true", help="Only inspect a cross-compiled archive")
    args = parser.parse_args()
    check(args.archive.resolve(), args.version, not args.no_run)
