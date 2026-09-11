#!/usr/bin/env python3
"""Exercise the Arch package recipe without invoking makepkg or compiling Rust."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
PKGBUILD = ROOT / "PKGBUILD"


class PkgbuildTests(unittest.TestCase):
    def test_builds_and_installs_desktop_application(self):
        with tempfile.TemporaryDirectory(prefix="omasend-pkgbuild-test-") as temporary:
            work = Path(temporary)
            fake_bin = work / "bin"
            fake_bin.mkdir()
            install = shutil.which("ginstall") or shutil.which("install")
            self.assertIsNotNone(install)
            (fake_bin / "install").symlink_to(install)
            cargo_log = work / "cargo.log"
            cargo = fake_bin / "cargo"
            cargo.write_text(
                "#!/bin/sh\n"
                "printf '%s\\n' \"$*\" > \"$CARGO_LOG\"\n"
                "mkdir -p \"$CARGO_TARGET_DIR/release\"\n"
                "printf '#!/bin/sh\\n' > \"$CARGO_TARGET_DIR/release/omasend\"\n"
                "chmod 755 \"$CARGO_TARGET_DIR/release/omasend\"\n"
            )
            cargo.chmod(0o755)

            pkgdir = work / "pkg"
            srcdir = work / "src"
            pkgdir.mkdir()
            srcdir.mkdir()
            command = (
                'set -eu; source "$RECIPE"; '
                '[ "$pkgname" = omasend ]; '
                '[[ " ${arch[*]} " == *" x86_64 "* ]]; '
                '[[ " ${arch[*]} " == *" aarch64 "* ]]; '
                "build; package"
            )
            env = dict(
                os.environ,
                PATH=f"{fake_bin}:{os.environ['PATH']}",
                CARGO_LOG=str(cargo_log),
                RECIPE=str(PKGBUILD),
                startdir=str(ROOT),
                srcdir=str(srcdir),
                pkgdir=str(pkgdir),
            )
            result = subprocess.run(
                ["bash", "-c", command], env=env, capture_output=True, text=True
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(cargo_log.read_text().strip(), "build --release --locked")
            self.assertTrue(os.access(pkgdir / "usr/bin/omasend", os.X_OK))
            self.assertTrue((pkgdir / "usr/share/applications/omasend.desktop").is_file())
            self.assertTrue(
                (pkgdir / "usr/share/icons/hicolor/1024x1024/apps/omasend.png").is_file()
            )
            license_file = pkgdir / "usr/share/licenses/omasend/LICENSE"
            self.assertIn("MIT License", license_file.read_text())


if __name__ == "__main__":
    unittest.main()
