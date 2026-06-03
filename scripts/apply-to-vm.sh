#!/usr/bin/env sh
# Sync a locally-exported Omarchy theme to a VM and apply it there.
#
# Designed to be used as the studio's apply command so pressing `a` in
# omarchy-theme-studio (on the host) applies the theme on the VM:
#
#   export OTS_VM=user@192.168.122.50          # the Omarchy VM over SSH
#   export OTS_APPLY_CMD="$PWD/scripts/apply-to-vm.sh"
#   omarchy-theme-studio
#
# It can also be run by hand:  scripts/apply-to-vm.sh <theme-name>
#
# Requirements:
#   - SSH key auth (or ssh-agent) so rsync/ssh are NON-INTERACTIVE. A password
#     prompt would hang the TUI.
#   - `omarchy-theme-set` present on the VM (it is, on Omarchy).
set -eu

name="${1:-}"
if [ -z "$name" ]; then
    echo "usage: apply-to-vm.sh <theme-name>" >&2
    exit 1
fi

: "${OTS_VM:?set OTS_VM=user@vm-host (e.g. allie@192.168.122.50)}"

themes_dir="${OTS_THEMES_DIR:-$HOME/.config/omarchy/themes}"
src="$themes_dir/$name"
if [ ! -d "$src" ]; then
    echo "theme dir not found: $src" >&2
    exit 1
fi

# Non-interactive SSH: fail fast instead of prompting and hanging the UI.
SSH_OPTS="-o BatchMode=yes -o ConnectTimeout=10"

# 1. Push the theme directory into the VM's user theme store.
rsync -az --delete -e "ssh $SSH_OPTS" \
    "$src/" "$OTS_VM:.config/omarchy/themes/$name/"

# 2. Apply it on the VM. Omarchy's commands live in ~/.local/share/omarchy/bin,
# which isn't on the non-interactive SSH PATH, so add it explicitly. The
# omarchy-theme-set script bootstraps the rest of its own PATH once found.
# `$name` is normalized (alnum/-/_) by the studio.
# omarchy-theme-set restarts waybar et al. but NOT walker, so refresh it too
# (best-effort) — otherwise the launcher keeps its old CSS until next login.
# shellcheck disable=SC2029
ssh $SSH_OPTS "$OTS_VM" \
    "export PATH=\"\$HOME/.local/share/omarchy/bin:\$HOME/.local/bin:\$PATH\"; \
     omarchy-theme-set '$name'; rc=\$?; \
     omarchy-restart-walker >/dev/null 2>&1 || true; \
     exit \$rc"

echo "applied '$name' on $OTS_VM"
