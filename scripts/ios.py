#!/usr/bin/env python3
"""Build with NSH, add app permissions, sign, and optionally install on an iPhone."""
import argparse
import pathlib
import plistlib
import shutil
import subprocess
import tomllib
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run(args, cwd=ROOT, check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sign", required=True, help="Apple Development signing identity or SHA-1")
    parser.add_argument("--profile", required=True, type=pathlib.Path, help="Provisioning profile")
    parser.add_argument("--device", help="Install and launch on this device UDID")
    args = parser.parse_args()
    profile = args.profile.expanduser().resolve()
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    run("nsh", "package", "--target", "ios", "--icon", "assets/omasend.png",
        "--app-version", version, "--sign", args.sign, "--profile", str(profile))
    app = ROOT / ".nsh/out/Omasend.app"
    info_path = app / "Info.plist"
    with info_path.open("rb") as file:
        info = plistlib.load(file)
    info.update({
        "NSLocalNetworkUsageDescription": "Omasend discovers nearby devices and transfers files over your local network.",
        "UIFileSharingEnabled": True,
        "LSSupportsOpeningDocumentsInPlace": True,
    })
    with info_path.open("wb") as file:
        plistlib.dump(info, file)
    decoded = run("security", "cms", "-D", "-i", str(profile), capture_output=True).stdout
    entitlements = plistlib.loads(decoded)["Entitlements"]
    bundle_id = info["CFBundleIdentifier"]
    prefix = entitlements["application-identifier"].split(".", 1)[0]
    entitlements["application-identifier"] = f"{prefix}.{bundle_id}"
    entitlements["keychain-access-groups"] = [f"{prefix}.{bundle_id}"]
    entitlements_path = ROOT / ".nsh/out/Omasend.entitlements"
    with entitlements_path.open("wb") as file:
        plistlib.dump(entitlements, file)
    shutil.copyfile(profile, app / "embedded.mobileprovision")
    run("codesign", "--force", "--sign", args.sign, "--entitlements", str(entitlements_path), str(app))
    run("codesign", "--verify", "--strict", str(app))
    ipa = ROOT / ".nsh/out/Omasend.ipa"
    with zipfile.ZipFile(ipa, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(app.rglob("*")):
            archive.write(path, pathlib.Path("Payload") / path.relative_to(app.parent))
    print(f"Signed: {ipa}", flush=True)
    if args.device:
        run("xcrun", "devicectl", "device", "install", "app", "--device", args.device, str(app))
        run("xcrun", "devicectl", "device", "process", "launch", "--device", args.device, bundle_id)


if __name__ == "__main__":
    main()
