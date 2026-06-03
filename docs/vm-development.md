# Developing against an Omarchy VM

This document describes the workflow used to develop **omarchy-theme-studio** on a fast
host while testing themes on a separate Omarchy VM (run under virt-manager / libvirt).
You edit and build on the host, press `a` in the studio, and the theme is synced to the
VM and applied there — you watch the result live in the virt-manager console.

```
┌─────────────── HOST (dev machine) ───────────────┐        ┌────── Omarchy VM ──────┐
│  omarchy-theme-studio (TUI)                       │        │                        │
│    s → export ~/.config/omarchy/themes/<name>/    │        │  ~/.config/omarchy/    │
│    a → OTS_APPLY_CMD = scripts/apply-to-vm.sh     │        │    themes/<name>/      │
│            │                                      │        │                        │
│            ├─ rsync theme dir ───────────────────────────► │  (over SSH / NAT)      │
│            └─ ssh omarchy-theme-set <name> ──────────────► │  switches live theme   │
└───────────────────────────────────────────────────┘        └────────────────────────┘
```

The key idea: the studio's apply command is overridable via the `OTS_APPLY_CMD`
environment variable. Set it to the bundled SSH wrapper and "apply" transparently
targets the VM instead of the local machine. Unset, it applies locally as normal.

---

## Prerequisites

**Host**
- This repo built: `cargo build --release`
- `ssh` and `rsync` installed
- libvirt/virt-manager with the VM on the **default (NAT)** network — the host reaches
  the guest directly at `192.168.122.x`

**VM (Omarchy guest)**
- Omarchy installed (its theme commands live in `~/.local/share/omarchy/bin/`)
- `sshd` enabled (see below)

---

## One-time setup

### 1. Enable SSH on the VM

In the **virt-manager console** of the guest:

```sh
sudo pacman -S --needed openssh        # no-op if already installed
sudo systemctl enable --now sshd
ss -tlnp | grep ':22'                  # confirm sshd is listening on *:22
ip -4 addr | grep 'inet '              # note the 192.168.122.x address
```

If a firewall is active (most Omarchy installs have none), allow SSH:
`sudo ufw allow 22/tcp` (ufw) or the firewalld equivalent.

> **Gotcha:** if `ssh`/`ssh-copy-id` from the host *hangs* (rather than returning
> "connection refused"), port 22 is being **dropped** — sshd isn't running yet or a
> firewall is blocking it. The VM being pingable is not enough; check `ss -tlnp` above.

### 2. Key-based auth from the host

The studio shells out to `ssh`/`rsync` from inside a raw-mode TUI, so they **must be
non-interactive** — a password prompt would hang the UI. Use key auth:

```sh
[ -f ~/.ssh/id_ed25519 ] || ssh-keygen -t ed25519
ssh-copy-id user@192.168.122.138       # one-time password entry

# verify it is now non-interactive (must print OK with NO prompt):
ssh -o BatchMode=yes user@192.168.122.138 'echo OK'
```

### 3. Confirm `omarchy-theme-set` is reachable

Omarchy's commands live in `~/.local/share/omarchy/bin`, which is only added to PATH by
your **interactive** shell — a non-interactive `ssh host 'cmd'` won't see it. The wrapper
handles this by prepending that directory; verify it resolves:

```sh
ssh -o BatchMode=yes user@192.168.122.138 \
  'export PATH="$HOME/.local/share/omarchy/bin:$PATH"; command -v omarchy-theme-set'
# → /home/<user>/.local/share/omarchy/bin/omarchy-theme-set
```

---

## Daily workflow

Set the two environment variables (put them in your shell rc or a `direnv` `.envrc`):

```sh
cd /path/to/omarchy-theme-studio
export OTS_VM=user@192.168.122.138                 # the VM over SSH
export OTS_APPLY_CMD="$PWD/scripts/apply-to-vm.sh"  # route apply to the VM
./target/release/omarchy-theme-studio
```

Then, in the studio:

1. Edit a palette field (`↑↓` to move, `c`/`Enter` to open the color picker).
2. `s` — export the theme to `~/.config/omarchy/themes/<name>/` on the host.
3. `a` → `y` — sync to the VM and apply. Watch the virt-manager console.

`OTS_APPLY_CMD` overrides the local `omarchy-theme-set` with any `<cmd> <name>` command;
the studio passes the theme name as a positional argument, so the same confirmed `a`
action works locally or against the VM with zero code differences.

### The wrapper

`scripts/apply-to-vm.sh <name>` does two things:

1. `rsync -az --delete` the host theme dir to `<OTS_VM>:~/.config/omarchy/themes/<name>/`.
2. `ssh` in and run `omarchy-theme-set <name>` (with the omarchy bin dir on PATH), then
   best-effort `omarchy-restart-walker`.

It uses `ssh -o BatchMode=yes` so a missing key **fails fast** instead of hanging the TUI.
Run it by hand for a quick test: `OTS_VM=user@ip scripts/apply-to-vm.sh <name>`.

### Headless export (no TUI)

Useful for scripting or seeding themes:

```sh
./target/release/omarchy-theme-studio export <name> [preset]
# presets: tokyo-night, nord, gruvbox, rose-pine
```

---

## Verifying an apply

```sh
ssh -o BatchMode=yes user@192.168.122.138 'cat ~/.config/omarchy/current/theme.name'
# → <name>
```

## Reverting the VM

```sh
ssh -o BatchMode=yes user@192.168.122.138 \
  'export PATH="$HOME/.local/share/omarchy/bin:$PATH"; omarchy-theme-set aether'
```

(or just apply any other theme).

---

## Troubleshooting log (issues actually hit)

| Symptom | Cause | Fix |
| --- | --- | --- |
| `ssh-copy-id` hangs | sshd not running / firewall dropping port 22 (VM pings fine) | `sudo systemctl enable --now sshd`; open port 22 |
| `which: no omarchy-theme-set` over SSH | omarchy bin dir not on non-interactive PATH | wrapper prepends `~/.local/share/omarchy/bin` |
| Hyprland "Config error … line 13: source= globbing error: found no match" | Transient: Hyprland reloaded mid theme-swap; `current/theme/hyprland.conf` momentarily absent | Benign; clears on next `hyprctl reload`. Affects any theme switch. |
| Launcher selection shows no highlight | `walker.css` used wrong color names + selectors; walker references `selected-text` | Theme files are value-only `@define-color` sets — fixed `walker.css`/`waybar.css` to Omarchy's schema |
| Launcher unchanged after apply | `omarchy-theme-set` restarts Waybar but **not** walker | wrapper now runs `omarchy-restart-walker` |
| Lock screen is black & white | theme had no `hyprlock.conf`; `~/.config/hypr/hyprlock.conf` `source`s it | exporter now writes `hyprlock.conf`. Hard-`source`d files (`hyprland.conf`, `hyprlock.conf`) MUST be shipped — `omarchy-theme-set` does not template-generate them for user themes |

### How Omarchy theme files actually work (reference)

- `colors.toml` is the canonical palette. `omarchy-theme-set-templates` reads it and fills
  `{{ key }}` placeholders in `~/.local/share/omarchy/default/themed/*.tpl`, and derives a
  full semantic + 16-color palette from a small core (`background`, `foreground`, `accent`, …).
- App theme files (`walker.css`, `waybar.css`, `hyprland.conf`, `ghostty.conf`) are **value
  sets**; the *selectors/layout* live in each app's own config and reference fixed
  `@define-color` names. A theme that invents its own names or adds selectors gets ignored.
  Mirror a known-good installed theme (e.g. `~/.config/omarchy/themes/aether/`) when in doubt.
- `omarchy-theme-set <name>` resolves a theme purely by directory under
  `~/.config/omarchy/themes/<name>/`, copies it to a staging dir, runs the template step,
  then atomically symlinks `~/.config/omarchy/current/theme` to it.

---

## Networking notes (libvirt)

- The guest is reachable from the host over libvirt's **default NAT** bridge `virbr0`
  (host is `192.168.122.1`, guests get `192.168.122.x`).
- If the `default` network is down: `sudo virsh net-start default && sudo virsh net-autostart default`.
- `virsh` showing no domains usually means the VM is defined under `qemu:///system` and your
  user isn't in the `libvirt` group — it doesn't affect SSH reachability.
