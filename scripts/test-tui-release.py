#!/usr/bin/env python3
"""Check standalone TUI archive names, contents and checksums on every target."""
import importlib.util
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parent.parent


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


build = load("build_release", "packaging/build-release.py")
verify = load("check_release", "packaging/check-release.py")


class TerminalReleaseTests(unittest.TestCase):
    def test_archives_contain_a_terminal_binary_without_desktop_assets(self):
        targets = ["x86_64-pc-windows-msvc"] if os.name == "nt" else build.TARGETS
        for target in targets:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                binary = root / "binary"
                binary.write_bytes(b"terminal executable")
                with patch.object(build.subprocess, "run") as commands:
                    build.package(ROOT, binary, root / "dist", "9.0.0", target, "tui")
                    extension = "zip" if "windows" in target else "tar.gz"
                    archive = root / "dist" / f"omasend-tui-9.0.0-{target}.{extension}"
                    if extension == "zip":
                        with zipfile.ZipFile(archive) as bundle:
                            names = bundle.namelist()
                    else:
                        with tarfile.open(archive) as bundle:
                            names = bundle.getnames()
                    executable = "omasend-tui.exe" if extension == "zip" else "omasend-tui"
                    self.assertEqual(set(names), {executable, "LICENSE", "README.md"})
                    verify.check(archive, "9.0.0", False)
                    if "apple-darwin" in target:
                        self.assertTrue(any(call.args[0][0] == "codesign" for call in commands.call_args_list))
                    else:
                        commands.assert_not_called()
                archive.write_bytes(archive.read_bytes() + b"tampered")
                with self.assertRaisesRegex(SystemExit, "checksum"):
                    verify.check(archive, "9.0.0", False)


if __name__ == "__main__":
    unittest.main()
