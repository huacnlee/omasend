#!/bin/sh
# Install the published OmaSend app for the current user. No sudo required.
set -eu

version="${OMASEND_VERSION:-latest}"
destination="${OMASEND_INSTALL_DIR:-}"
usage() {
  printf '%s\n' 'Usage: install.sh [--version VERSION] [--dir DIRECTORY]' \
    'Defaults: latest release; ~/Applications on macOS, ~/.local on Linux.'
}
while [ "$#" -gt 0 ]; do
  case "$1" in
    --version|--dir)
      [ "$#" -ge 2 ] || { usage >&2; exit 2; }
      if [ "$1" = --version ]; then version="$2"; else destination="$2"; fi
      shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
done
die() { printf 'OmaSend: %s\n' "$*" >&2; exit 1; }
for tool in curl tar awk; do command -v "$tool" >/dev/null || die "Required command missing: $tool"; done
case "$(uname -s)" in
  Darwin) platform=macos; suffix=apple-darwin; destination="${destination:-$HOME/Applications}" ;;
  Linux) platform=linux; suffix=unknown-linux-gnu; destination="${destination:-$HOME/.local}" ;;
  *) die 'Use install.ps1 on Windows. This script supports macOS and Linux.' ;;
esac
case "$(uname -m)" in
  x86_64|amd64) arch=x86_64 ;;
  arm64|aarch64) arch=aarch64 ;;
  *) die 'Unsupported CPU architecture (requires x86_64 or arm64).' ;;
esac
if [ "$platform" = linux ] && [ "$arch" != x86_64 ]; then
  die 'Linux releases currently support x86_64 only. Build from source on ARM64.'
fi
download() { curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location "$@"; }
repo=https://github.com/huacnlee/omasend
if [ "$version" = latest ]; then
  release_url="$(download --output /dev/null --write-out '%{url_effective}' "$repo/releases/latest")"
  version="${release_url##*/}"
fi
version="${version#v}"
awk -v version="$version" 'BEGIN {exit !(version ~ /^[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$/)}' || die 'Version must be a release version such as 0.1.0 or v0.1.0.'
asset="omasend-$version-$arch-$suffix.tar.gz"
base="$repo/releases/download/v$version"
work="$(mktemp -d "${TMPDIR:-/tmp}/omasend-install.XXXXXXXX")"
trap 'rm -rf "$work"' 0
printf 'Downloading OmaSend %s for %s %s…\n' "$version" "$platform" "$arch"
download "$base/$asset" --output "$work/$asset"
download "$base/SHA256SUMS" --output "$work/SHA256SUMS"
expected="$(awk -v name="$asset" '$2 == name || $2 == "*" name { print $1 }' "$work/SHA256SUMS")"
[ "${#expected}" -eq 64 ] || die "Missing or invalid checksum for $asset"
case "$expected" in *[!a-fA-F0-9]*) die "Missing or invalid checksum for $asset" ;; esac
if command -v sha256sum >/dev/null; then
  actual="$(sha256sum "$work/$asset")"
elif command -v shasum >/dev/null; then
  actual="$(shasum -a 256 "$work/$asset")"
else die 'Required command missing: sha256sum or shasum'; fi
[ "${actual%% *}" = "$expected" ] || die 'Checksum verification failed; nothing was installed.'
mkdir "$work/unpacked"
# Only accept relative archive paths inside the temporary extraction directory.
tar -tzf "$work/$asset" > "$work/entries"
if awk '/^\// || /(^|\/)\.\.(\/|$)/ {bad=1} END {exit !bad}' "$work/entries"; then
  die 'Release archive contains an unsafe path.'
fi
tar -xzf "$work/$asset" -C "$work/unpacked"
if [ "$platform" = macos ]; then
  app="$work/unpacked/OmaSend.app"
  [ -x "$app/Contents/MacOS/omasend" ] && [ -f "$app/Contents/Info.plist" ] || die 'Release archive is missing OmaSend.app.'
  mkdir -p "$destination"
  # Replace the bundle as a unit; preserve the previous version if copying fails.
  staging="$(mktemp -d "$destination/.omasend-install.XXXXXXXX")"
  trap 'rm -rf "$work" "${staging:-}"' 0
  ditto "$app" "$staging/OmaSend.app"
  previous="$staging/previous.app"
  if [ -e "$destination/OmaSend.app" ]; then mv "$destination/OmaSend.app" "$previous"; fi
  if ! mv "$staging/OmaSend.app" "$destination/OmaSend.app"; then
    if [ -e "$previous" ]; then mv "$previous" "$destination/OmaSend.app"; fi
    die 'Could not replace OmaSend.app.'
  fi
  printf 'Installed %s/OmaSend.app\nOpen it from Finder.\n' "$destination"
else
  source_dir="$work/unpacked"
  for file in omasend packaging/omasend.desktop assets/omasend.png; do
    [ -f "$source_dir/$file" ] || die "Release archive is missing $file."
  done
  mkdir -p "$destination/bin" "$destination/share/applications" "$destination/share/icons/hicolor/1024x1024/apps"
  # Rename a new executable into place so a running instance keeps its old inode.
  staged_binary="$(mktemp "$destination/bin/.omasend.XXXXXXXX")"
  trap 'rm -rf "$work"; rm -f "${staged_binary:-}"' 0
  install -m755 "$source_dir/omasend" "$staged_binary"
  mv -f "$staged_binary" "$destination/bin/omasend"
  install -m644 "$source_dir/packaging/omasend.desktop" "$destination/share/applications/omasend.desktop"
  install -m644 "$source_dir/assets/omasend.png" "$destination/share/icons/hicolor/1024x1024/apps/omasend.png"
  if command -v update-desktop-database >/dev/null; then update-desktop-database "$destination/share/applications" >/dev/null 2>&1 || true; fi
  printf 'Installed %s/bin/omasend\nAdd %s/bin to PATH, then run omasend.\n' "$destination" "$destination"
  printf '%s\n' 'Linux requires graphics drivers and desktop libraries; install wl-clipboard for paste.'
fi
