#!/bin/sh
set -eu

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust and Cargo are required. Install them from https://rustup.rs, then rerun this script." >&2
    exit 1
fi

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
install_root=${CARGO_HOME:-"$HOME/.cargo"}

cargo install --locked --path "$repo_dir" --root "$install_root"

echo "Installed ring to $install_root/bin/ring"
case ":$PATH:" in
    *":$install_root/bin:"*) echo "Run: ring" ;;
    *) printf 'Add this to your shell config, then restart your shell:\nexport PATH="%s/bin:$PATH"\n' "$install_root" ;;
esac
