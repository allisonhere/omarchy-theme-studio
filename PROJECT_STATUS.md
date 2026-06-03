# omarchy-theme-studio

A terminal UI for designing Omarchy desktop themes (Rust, `ratatui` + `crossterm`).

## Project Location

```
/home/allie/Projects/omarchy-theme-studio/
```

## Architecture

- `src/theme/mod.rs` — `RgbColor`, the flat `ThemePalette` (17 colors), the `Theme { name, palette }`
  wrapper, and the `PaletteField` editing-target enum.
- `src/theme/presets.rs` — built-in starter palettes (tokyo-night, nord, gruvbox, rose-pine).
- `src/config/mod.rs` — `ConfigManager`: exports a theme directory under
  `~/.config/omarchy/themes/<name>/` (colors.toml, hyprland.conf, waybar.css, walker.css,
  ghostty.conf, README.md, palette.json), lists/loads/renames/deletes, and the gated apply via
  `omarchy-theme-set`.
- `src/ui/state.rs` — `App` state machine, palette-field selection, undo/yank/paste, loader, save/apply flows.
- `src/ui/render.rs` — mock Hyprland desktop preview + color-picker / loader / help / confirm overlays.
- `src/ui/events.rs` — key/mouse handling and the terminal event loop.
- `src/ui/color_picker.rs` — reusable RGB/HSL/HEX color editor.
- `src/update.rs` — in-app self-update from GitHub releases.

## Status

- v1 complete: edit palette, live mock-desktop preview, export Omarchy-native theme dir, gated apply.
- Non-destructive: only writes inside the theme's own directory; never auto-applies.

## Running

```bash
cd /home/allie/Projects/omarchy-theme-studio
cargo run
```

Saved themes land in `~/.config/omarchy/themes/<name>/`.

## Out of scope for v1

- Generating every Omarchy app file (btop/neovim/vscode/mako/gtk/etc.) — `colors.toml` plus
  Omarchy's template engine covers the wider ecosystem.
- Background images, `light.mode` detection, and a `preview.png` generator.
