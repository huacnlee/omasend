#!/bin/sh
# Build the CLI in its checkout so nshm-only apps link the same nightly crates.
set -eu
cd "$(dirname "$0")/.."
revision=f577c51901a2857956f8ba0580da4db9ce3a3311
source_dir="$PWD/.nsh/toolchain/$revision"
if [ ! -d "$source_dir/.git" ]; then
  git init -q "$source_dir"
fi
if ! git -C "$source_dir" cat-file -e "$revision^{commit}" 2>/dev/null; then
  git -C "$source_dir" fetch --depth 1 https://github.com/longbridge/nsh.git "$revision"
fi
git -C "$source_dir" checkout --quiet --detach "$revision"
(cd "$source_dir" && cargo build --locked --release -p nsh-cli --target-dir target)
"$source_dir/target/release/nsh" --version
mkdir -p .nsh/bin
ln -sfn "$source_dir/target/release/nsh" .nsh/bin/nsh
