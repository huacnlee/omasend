#!/usr/bin/env python3
"""Package a prebuilt release binary using the install scripts' archive contract."""
import argparse
import hashlib
from pathlib import Path
import plistlib
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

TARGETS = {
    "aarch64-apple-darwin": "macos",
    "x86_64-apple-darwin": "macos",
    "x86_64-unknown-linux-gnu": "linux",
    "x86_64-pc-windows-msvc": "windows",
}


def package(root, binary, output, version, target):
    platform = TARGETS[target]
    output.mkdir(parents=True, exist_ok=True)
    extension = "zip" if platform == "windows" else "tar.gz"
    archive = output / f"omasend-{version}-{target}.{extension}"
    with tempfile.TemporaryDirectory(prefix="omasend-package-") as temporary:
        stage = Path(temporary)
        if platform == "macos":
            app = stage / "OmaSend.app"
            contents = app / "Contents"
            (contents / "MacOS").mkdir(parents=True)
            (contents / "Resources").mkdir()
            shutil.copy2(binary, contents / "MacOS/omasend")
            (contents / "MacOS/omasend").chmod(0o755)
            shutil.copy2(root / "assets/omasend.icns", contents / "Resources/omasend.icns")
            # Modern macOS wraps legacy-only icons in a pale compatibility tile.
            # Compile the Icon Composer asset while retaining our ICNS for macOS 13–15.
            with tempfile.TemporaryDirectory(prefix="omasend-icon-") as icon_temporary:
                icon_output = Path(icon_temporary)
                subprocess.run([
                    "xcrun", "actool", str(root / "assets/OmaSend.icon"),
                    "--compile", str(icon_output), "--output-format", "human-readable-text",
                    "--notices", "--warnings", "--errors",
                    "--output-partial-info-plist", str(icon_output / "partial.plist"),
                    "--app-icon", "OmaSend", "--include-all-app-icons",
                    "--enable-on-demand-resources", "NO", "--development-region", "en",
                    "--target-device", "mac", "--minimum-deployment-target", "13.0",
                    "--platform", "macosx",
                ], check=True)
                shutil.copy2(icon_output / "Assets.car", contents / "Resources/Assets.car")
            with (contents / "Info.plist").open("wb") as stream:
                plistlib.dump({
                    "CFBundleIdentifier": "org.omasend.OmaSend",
                    "CFBundleName": "Omasend",
                    "CFBundleDisplayName": "Omasend",
                    "CFBundleExecutable": "omasend",
                    "CFBundleIconFile": "omasend.icns",
                    "CFBundleIconName": "OmaSend",
                    "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": version,
                    "CFBundleVersion": version.split("-")[0],
                    "LSMinimumSystemVersion": "13.0",
                    "LSApplicationCategoryType": "public.app-category.utilities",
                    "NSHighResolutionCapable": True,
                    "NSLocalNetworkUsageDescription": "Omasend discovers nearby devices and transfers files on your local network.",
                    "CFBundleLocalizations": ["en", "zh_CN"],
                }, stream)
            # Ad-hoc signing preserves executable integrity on Apple Silicon.
            # Developer ID signing and notarization need release-owner credentials.
            subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
            subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
        elif platform == "linux":
            shutil.copy2(binary, stage / "omasend")
            (stage / "omasend").chmod(0o755)
            for name in ("packaging/omasend.desktop", "assets/omasend.png"):
                destination = stage / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(root / name, destination)
        else:
            shutil.copy2(binary, stage / "omasend.exe")
        for name in ("LICENSE", "README.md"):
            if (root / name).is_file():
                shutil.copy2(root / name, stage / name)
        if platform == "windows":
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as bundle:
                for path in sorted(stage.rglob("*")):
                    if path.is_file():
                        bundle.write(path, path.relative_to(stage))
        else:
            with tarfile.open(archive, "w:gz", format=tarfile.PAX_FORMAT) as bundle:
                for path in sorted(stage.iterdir()):
                    bundle.add(path, arcname=path.name)
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    # Release checksums are merged and read by Unix installers: keep LF even
    # when this sidecar is produced by the Windows matrix job.
    archive.with_name(archive.name + ".sha256").write_text(
        f"{digest}  {archive.name}\n", encoding="ascii", newline="\n"
    )
    print(archive)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=TARGETS)
    parser.add_argument("--version", required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=Path("dist"))
    args = parser.parse_args()
    if not args.binary.is_file():
        parser.error(f"binary not found: {args.binary}")
    import re
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", args.version):
        parser.error("version must be a SemVer release version without a leading v")
    package(Path(__file__).resolve().parent.parent, args.binary.resolve(), args.output.resolve(), args.version, args.target)
