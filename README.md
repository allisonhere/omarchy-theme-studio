# omarchy-theme-studio

A terminal UI for designing [Omarchy](https://omarchy.org) desktop themes. Edit a flat color
palette with a live mock-desktop preview — Waybar, Hyprland windows, the launcher, a terminal
sample, and a notification popup — then export a real, ready-to-apply Omarchy theme directory.

## Features

- **Mock desktop preview** — see every color reflected instantly across a faux Hyprland desktop
- **Flat palette model** — 17 named colors (background, foreground, accent, borders, Waybar,
  launcher, terminal, notification) edited one at a time
- **Dual-mode color picker** — RGB sliders plus an HSL field picker with live HEX/RGB/HSL values
- **Yank / paste / undo** — copy a color, paste it elsewhere, or undo the last change (`y` / `p` / `u`)
- **Theme loader** — fuzzy search and filter across built-in presets and your saved themes, with live preview
- **Omarchy-native export** — writes the real flat files Omarchy consumes (see below)
- **Gated apply** — if `omarchy-theme-set` is on your PATH, press `a` to apply, but only after
  an explicit confirmation. Nothing is ever applied automatically, and existing config is never edited.
- **In-app self-update** — checks GitHub releases and can replace the binary in place on Linux x86_64 (`U`)
- **Help overlay** — press `?` for a full keybinding reference

## What it exports

Saving a theme writes `~/.config/omarchy/themes/<name>/` containing:

| File | Purpose |
| --- | --- |
| `colors.toml` | Canonical Omarchy palette — `omarchy-theme-set` reads this to drive every app |
| `hyprland.conf` | `col.active_border` / `col.inactive_border` (Hyprland `rgb()` form) |
| `hyprlock.conf` | Lock screen colors (`source`d by `~/.config/hypr/hyprlock.conf`) |
| `waybar.css` | Waybar background, foreground, active workspace |
| `walker.css` | Launcher (walker) colors |
| `ghostty.conf` | Terminal background / foreground / cursor / selection |
| `README.md` | A palette reference for the theme |
| `palette.json` | Round-trip source so the studio can reload the theme for editing |

To apply a theme, copy its directory into `~/.config/omarchy/themes/` (the studio already writes
there) and run `omarchy-theme-set <name>` — or press `a` in the studio.

Theme files are plain `@define-color` / `key = value` sets — the selectors live in walker's and
Waybar's own layout, which reference fixed color names. A couple of palette fields therefore drive
the live preview but don't have a dedicated slot in Omarchy's current file formats:
the **launcher selected foreground** (walker exposes a single `selected-text` highlight, fed from
*launcher selected background*) and the **Waybar active workspace** (Omarchy colors that from
`colors.toml`'s `accent`). Everything else maps directly.

## Installation

### Prebuilt binary (Linux x86_64)

```sh
curl -fsSL https://raw.githubusercontent.com/allisonhere/omarchy-theme-studio/main/install-binary.sh | sh
```

### Build from source (all platforms)

Requires `git` and `cargo` ([rustup.rs](https://rustup.rs)).

```sh
curl -fsSL https://raw.githubusercontent.com/allisonhere/omarchy-theme-studio/main/install-source.sh | sh
```

Both scripts install to `~/.local/bin` by default. Override with `INSTALL_DIR`:

```sh
curl -fsSL ... | INSTALL_DIR=/usr/local/bin sh
```

### Manual install

```sh
git clone https://github.com/allisonhere/omarchy-theme-studio.git
cd omarchy-theme-studio
cargo build --release
cp target/release/omarchy-theme-studio ~/.local/bin/
```

## Usage

```
omarchy-theme-studio
```

The app opens a full-terminal mock desktop. Navigate the palette fields and edit their colors.

### Keybindings

| Key | Action |
|-----|--------|
| `1`–`6` | Jump to a palette group (Desktop, Windows, Waybar, Launcher, Terminal, Notification) |
| `←` `→` / `Tab` `⇧Tab` | Previous / next group |
| `↑/k` `↓/j` | Move between fields within the current group |
| `/` | Find a field by fuzzy search (type, `↑↓`, `Enter`) |
| `c` / `Enter` | Open color picker for the selected field |
| `y` | Yank (copy) current color |
| `p` | Paste yanked color |
| `u` | Undo last color change |
| `s` | Export theme to `~/.config/omarchy/themes/` |
| `l` | Open theme loader |
| `a` | Apply via `omarchy-theme-set` (asks for confirmation; only if installed) |
| `U` | Install the latest released binary when an update is available on Linux x86_64 |
| `?` | Toggle help overlay |
| `q` / `Esc` | Quit |

**Color picker:**

| Key | Action |
|-----|--------|
| `Tab` / `Shift + Tab` | Move focus between picker controls |
| `m` | Switch RGB sliders / HSL field |
| `↑ ↓ ← →` | Nudge the focused control |
| `Shift` / `Alt` with arrows | Coarse / fine nudging |
| `#` | Jump straight to hex editing |
| `Enter` | Edit the focused value field or keep |
| `Esc` | Cancel |
| `mouse drag` | Drag in the HSL field or lightness slider |

**Theme loader:**

| Key | Action |
|-----|--------|
| `type` | Search — fuzzy filter by name |
| `↑ ↓` | Navigate themes |
| `Enter` | Load selected theme into the editor |
| `d` | Filter: built-in presets only |
| `s` | Filter: saved themes only |
| `r` | Rename selected saved theme |
| `x` | Delete selected saved theme |
| `Esc` | Clear search / cancel |

## Palette fields

| Group | Fields |
|-------|--------|
| Desktop | background, foreground, accent, accent 2 |
| Windows | active border, inactive border |
| Waybar | background, foreground, active workspace |
| Launcher | background, foreground, selected background, selected foreground |
| Terminal | background, foreground |
| Notification | background, border |

## Applying to a VM (develop on the host, see it on the guest)

Run the studio on your fast host and have `a` apply on an Omarchy VM over SSH. Set
`OTS_APPLY_CMD` to the bundled wrapper, which rsyncs the exported theme to the VM and runs
`omarchy-theme-set` there:

```sh
export OTS_VM=user@192.168.122.50            # the VM, over SSH (key auth!)
export OTS_APPLY_CMD="$PWD/scripts/apply-to-vm.sh"
omarchy-theme-studio                          # press a → confirm → it lands on the VM
```

`OTS_APPLY_CMD` overrides the local `omarchy-theme-set` with any `<cmd> <name>` command, so
the studio works the same whether it's applying locally or to a remote machine. SSH must be
**non-interactive** (key auth / ssh-agent) — the wrapper uses `BatchMode` so a missing key
fails fast instead of hanging the TUI. You can also run the wrapper by hand:
`scripts/apply-to-vm.sh <theme-name>`.

See [`docs/vm-development.md`](docs/vm-development.md) for the full host→VM setup (SSH
enablement, key auth, the PATH gotcha, verifying, reverting, and a troubleshooting log).

## Safety

- **Non-destructive:** saving only ever writes inside `~/.config/omarchy/themes/<name>/`.
- **No automatic apply:** the studio never switches your live theme on its own. The `a` action
  runs `omarchy-theme-set` only after you confirm, and only when that command exists.

## Requirements

- A terminal with true color support
- Rust stable (only needed if building from source)
- Omarchy (for applying themes); the studio runs and exports fine without it
