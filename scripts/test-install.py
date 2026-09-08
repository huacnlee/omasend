#!/usr/bin/env python3
"""Exercise installers using local release fixtures; never modify the host install."""
import hashlib
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parent.parent / "install.sh"


class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="omasend-installer-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.dest = self.root / "install with spaces"
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}", FIXTURES=str(self.root))
        self.mock("uname", '#!/bin/sh\nif [ "$1" = -s ]; then echo "${MOCK_OS:-Linux}"; else echo "${MOCK_ARCH:-x86_64}"; fi\n')
        self.mock("curl", '''#!/usr/bin/env python3
import os, pathlib, shutil, sys
args = sys.argv[1:]
url = next(x for x in args if x.startswith('https://'))
if url.endswith('/latest'):
    print('https://github.com/huacnlee/omasend/releases/tag/v0.1.0', end='')
else:
    shutil.copyfile(pathlib.Path(os.environ['FIXTURES']) / url.rsplit('/', 1)[1], args[args.index('--output') + 1])
''')

    def mock(self, name, source):
        path = self.bin / name
        path.write_text(source)
        path.chmod(0o755)

    def release(self, target="x86_64-unknown-linux-gnu", unsafe=False):
        archive = self.root / f"omasend-0.1.0-{target}.tar.gz"
        files = {"omasend": b"#!/bin/sh\n", "packaging/omasend.desktop": b"[Desktop Entry]\nExec=omasend\n", "assets/omasend.png": b"icon"}
        if "apple" in target:
            files = {"OmaSend.app/Contents/MacOS/omasend": b"#!/bin/sh\n", "OmaSend.app/Contents/Info.plist": b"plist"}
        if unsafe:
            files["../escaped"] = b"bad"
        with tarfile.open(archive, "w:gz") as tar:
            for name, data in files.items():
                info = tarfile.TarInfo(name)
                info.size = len(data)
                info.mode = 0o755
                tar.addfile(info, io.BytesIO(data))
        (self.root / "SHA256SUMS").write_text(f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}\n")
        return archive

    def run_install(self, *args):
        return subprocess.run([os.environ.get("INSTALL_TEST_SHELL", "sh"), str(SCRIPT), "--dir", str(self.dest), *args], env=self.env, capture_output=True, text=True)

    def test_pipe_to_sh_install(self):
        self.release()
        self.env["OMASEND_INSTALL_DIR"] = str(self.dest)
        result = subprocess.run([os.environ.get("INSTALL_TEST_SHELL", "sh")], input=SCRIPT.read_text(), env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.dest / "bin/omasend").is_file())

    def test_linux_latest_install_and_upgrade(self):
        self.release()
        for _ in range(2):
            result = self.run_install()
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.dest / "bin/omasend").is_file())
        self.assertTrue((self.dest / "share/applications/omasend.desktop").is_file())
        self.assertTrue((self.dest / "share/icons/hicolor/1024x1024/apps/omasend.png").is_file())

    def test_mac_bundle_install_and_upgrade(self):
        self.env.update(MOCK_OS="Darwin", MOCK_ARCH="arm64")
        self.mock("ditto", '#!/bin/sh\ncp -R "$1" "$2"\n')
        self.release("aarch64-apple-darwin")
        for _ in range(2):
            result = self.run_install("--version", "v0.1.0")
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.dest / "OmaSend.app/Contents/MacOS/omasend").is_file())
        self.assertEqual(len(list(self.dest.iterdir())), 1)

    def test_bad_checksum_does_not_install(self):
        archive = self.release()
        archive.write_bytes(b"tampered")
        result = self.run_install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Checksum verification failed", result.stderr)
        self.assertFalse(self.dest.exists())

    def test_mac_failed_replacement_restores_previous_bundle(self):
        self.env.update(MOCK_OS="Darwin", MOCK_ARCH="arm64")
        self.mock("ditto", '#!/bin/sh\ncp -R "$1" "$2"\n')
        self.mock("mv", '#!/bin/sh\ncase "$1" in */.omasend-install.*/OmaSend.app) exit 1 ;; esac\nexec /bin/mv "$@"\n')
        self.release("aarch64-apple-darwin")
        previous = self.dest / "OmaSend.app/Contents/MacOS/omasend"
        previous.parent.mkdir(parents=True)
        previous.write_text("previous version")
        result = self.run_install("--version", "0.1.0")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Could not replace", result.stderr)
        self.assertEqual(previous.read_text(), "previous version")
        self.assertEqual(len(list(self.dest.iterdir())), 1)

    def test_unsafe_archive_does_not_install(self):
        self.release(unsafe=True)
        result = self.run_install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsafe path", result.stderr)
        self.assertFalse(self.dest.exists())

    def test_unsupported_architecture(self):
        self.env["MOCK_ARCH"] = "aarch64"
        result = self.run_install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("x86_64 only", result.stderr)

    def test_invalid_version_does_not_download(self):
        result = self.run_install("--version", "../bad")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Version must be", result.stderr)


if __name__ == "__main__":
    unittest.main()
