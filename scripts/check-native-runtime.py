#!/usr/bin/env python3
"""Keep the Rust migration on NSH's native backends without a JS runtime."""
import json
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
result = subprocess.run(
    ["cargo", "metadata", "--locked", "--all-features", "--format-version", "1"],
    cwd=root,
    check=True,
    capture_output=True,
    text=True,
)
metadata = json.loads(result.stdout)
names = {package["name"] for package in metadata["packages"]}
forbidden = sorted(
    name for name in names
    if "gpui" in name.lower()
    or name == "nsh-js"
    or "quickjs" in name.lower()
)
if forbidden:
    sys.exit("Forbidden runtime dependencies: " + ", ".join(forbidden))
local_nsh = sorted(
    package["name"] for package in metadata["packages"]
    if package["name"].startswith("nsh-") and package.get("source") is None
)
if local_nsh:
    sys.exit("NSH must use upstream dependencies, not local copies: " + ", ".join(local_nsh))
required = {"nsh-platform-macos", "nsh-platform-linux", "nsh-platform-windows", "nsh-platform-ios"}
missing = sorted(required - names)
if missing:
    sys.exit("Missing native backends: " + ", ".join(missing))
print("NSH Rust runtime: AppKit / GTK / Win32 / UIKit; no GPUI or JavaScript runtime")
