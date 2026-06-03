use crate::theme::{RgbColor, Theme, ThemePalette};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Theme not found: {0}")]
    ThemeNotFound(String),
    #[error("Could not parse palette: {0}")]
    Parse(String),
    #[error("Apply failed: {0}")]
    Apply(String),
}

/// Reads and writes Omarchy themes under `~/.config/omarchy/themes/<name>/`.
/// Each theme directory holds the real Omarchy flat files plus a `palette.json`
/// that we use as the round-trip source of truth for editing.
pub struct ConfigManager {
    themes_dir: PathBuf,
}

const PALETTE_FILE: &str = "palette.json";

impl ConfigManager {
    pub fn new() -> Self {
        let themes_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("omarchy")
            .join("themes");
        Self { themes_dir }
    }

    #[cfg(test)]
    pub fn with_themes_dir(themes_dir: PathBuf) -> Self {
        Self { themes_dir }
    }

    fn ensure_themes_dir(&self) -> Result<(), ConfigError> {
        if !self.themes_dir.exists() {
            fs::create_dir_all(&self.themes_dir)?;
        }
        Ok(())
    }

    /// Theme directories we can load back into the editor — those containing a
    /// `palette.json` we wrote.
    pub fn list_themes(&self) -> Result<Vec<String>, ConfigError> {
        let mut themes = Vec::new();
        if self.themes_dir.exists() {
            for entry in fs::read_dir(&self.themes_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() && path.join(PALETTE_FILE).is_file() {
                    if let Some(name) = path.file_name() {
                        themes.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
        Ok(themes)
    }

    pub fn load_theme(&self, name: &str) -> Result<Theme, ConfigError> {
        let palette_path = self.themes_dir.join(name).join(PALETTE_FILE);
        if !palette_path.is_file() {
            return Err(ConfigError::ThemeNotFound(name.to_string()));
        }
        let content = fs::read_to_string(&palette_path)?;
        let palette: ThemePalette =
            serde_json::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;
        Ok(Theme::new(name, palette))
    }

    /// Write the full theme directory. Non-destructive: only touches files
    /// inside `<themes_dir>/<name>/`.
    pub fn export_theme(&self, theme: &Theme) -> Result<PathBuf, ConfigError> {
        self.ensure_themes_dir()?;
        let dir = self.themes_dir.join(&theme.name);
        fs::create_dir_all(&dir)?;
        let p = &theme.palette;

        fs::write(dir.join(PALETTE_FILE), palette_json(p)?)?;
        fs::write(dir.join("colors.toml"), colors_toml(p))?;
        fs::write(dir.join("hyprland.conf"), hyprland_conf(p))?;
        fs::write(dir.join("hyprlock.conf"), hyprlock_conf(p))?;
        fs::write(dir.join("waybar.css"), waybar_css(p))?;
        fs::write(dir.join("walker.css"), walker_css(p))?;
        fs::write(dir.join("ghostty.conf"), ghostty_conf(p))?;
        fs::write(dir.join("README.md"), readme_md(&theme.name, p))?;

        Ok(dir)
    }

    pub fn rename_theme(&self, old_name: &str, new_name: &str) -> Result<(), ConfigError> {
        let old_path = self.themes_dir.join(old_name);
        let new_path = self.themes_dir.join(new_name);
        fs::rename(&old_path, &new_path)?;
        // Refresh derived files so the README/palette name match the new slug.
        if let Ok(mut theme) = self.load_theme(new_name) {
            theme.name = new_name.to_string();
            let _ = self.export_theme(&theme);
        }
        Ok(())
    }

    pub fn delete_theme(&self, name: &str) -> Result<(), ConfigError> {
        let path = self.themes_dir.join(name);
        fs::remove_dir_all(&path)?;
        Ok(())
    }

    /// Whether an apply command is available: either a custom `OTS_APPLY_CMD`
    /// (e.g. an SSH-to-VM wrapper) or `omarchy-theme-set` on PATH.
    pub fn apply_available(&self) -> bool {
        std::env::var_os("OTS_APPLY_CMD").is_some() || which_omarchy_theme_set().is_some()
    }

    /// Apply the named theme. Only call this after explicit user confirmation —
    /// it switches the live desktop theme.
    ///
    /// If `OTS_APPLY_CMD` is set it is run as `<cmd> <name>` via the shell, so it
    /// can be a wrapper that syncs to a remote machine and applies there. The
    /// command must be non-interactive (key-based SSH / ssh-agent) — it runs
    /// synchronously inside the TUI. Otherwise `omarchy-theme-set <name>` is used.
    pub fn apply_theme(&self, name: &str) -> Result<(), ConfigError> {
        // Use `.output()` so the child's stdout/stderr are captured rather than
        // printed onto the TUI's alternate screen (which corrupts the display).
        let (label, output) = if let Some(cmd) = std::env::var_os("OTS_APPLY_CMD") {
            let cmd = cmd.to_string_lossy().into_owned();
            // `name` is passed as positional `$1`, so the shell cannot reinterpret it.
            let out = Command::new("sh")
                .arg("-c")
                .arg(format!("{cmd} \"$1\""))
                .arg("sh")
                .arg(name)
                .output()
                .map_err(|e| ConfigError::Apply(e.to_string()))?;
            (format!("OTS_APPLY_CMD ({cmd})"), out)
        } else {
            let bin = which_omarchy_theme_set()
                .ok_or_else(|| ConfigError::Apply("omarchy-theme-set not found on PATH".into()))?;
            let out = Command::new(bin)
                .arg(name)
                .output()
                .map_err(|e| ConfigError::Apply(e.to_string()))?;
            ("omarchy-theme-set".to_string(), out)
        };

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let tail = stderr.trim().lines().last().unwrap_or("").to_string();
            Err(ConfigError::Apply(format!(
                "{label} exited with {}{}",
                output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
                if tail.is_empty() { String::new() } else { format!(": {tail}") },
            )))
        }
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

fn which_omarchy_theme_set() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("omarchy-theme-set");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

// ── File generators ──────────────────────────────────────────────────────────

fn palette_json(p: &ThemePalette) -> Result<String, ConfigError> {
    serde_json::to_string_pretty(p).map_err(|e| ConfigError::Parse(e.to_string()))
}

/// Canonical Omarchy palette. `omarchy-theme-set-templates` reads this to fill
/// `{{ key }}` placeholders and derive btop/neovim/etc colors, so it is the
/// cornerstone of compatibility. We emit the enriched-schema keys it understands
/// and let it derive the rest.
pub fn colors_toml(p: &ThemePalette) -> String {
    // Derive a couple of surfaces from the background so downstream apps get a
    // believable depth ramp even though our model only tracks core colors.
    let lighter = p.background.luminance() < 0.5;
    let surface = if lighter { p.background.shade(14) } else { p.background.shade(-14) };
    let surface_alt = if lighter { p.background.shade(28) } else { p.background.shade(-28) };

    let mut s = String::new();
    s.push_str("# Generated by omarchy-theme-studio\n");
    s.push_str("[palette]\n");
    for (k, c) in [
        ("background", p.background),
        ("foreground", p.foreground),
        ("text", p.foreground),
        ("accent", p.accent),
        ("accent_alt", p.accent2),
        ("surface", surface),
        ("surface_alt", surface_alt),
    ] {
        s.push_str(&format!("{k} = \"{}\"\n", c.to_hex()));
    }
    s
}

fn hyprland_conf(p: &ThemePalette) -> String {
    format!(
        "# Generated by omarchy-theme-studio\n\
         $activeBorderColor = rgb({active})\n\
         $inactiveBorderColor = rgb({inactive})\n\
         \n\
         general {{\n\
         \x20   col.active_border = $activeBorderColor\n\
         \x20   col.inactive_border = $inactiveBorderColor\n\
         }}\n\
         \n\
         group {{\n\
         \x20   col.border_active = $activeBorderColor\n\
         \x20   col.border_inactive = $inactiveBorderColor\n\
         }}\n",
        active = p.active_border.to_hex_bare(),
        inactive = p.inactive_border.to_hex_bare(),
    )
}

/// Waybar's style layer references named colors; an Omarchy theme only supplies
/// the `@define-color` values (matching Omarchy's `waybar.css` template — just
/// foreground/background). The active-workspace accent is driven by Omarchy from
/// `colors.toml`'s `accent`, not from a per-theme waybar color.
fn waybar_css(p: &ThemePalette) -> String {
    format!(
        "/* Generated by omarchy-theme-studio */\n\
         @define-color foreground {fg};\n\
         @define-color background {bg};\n",
        fg = p.waybar_foreground.to_hex(),
        bg = p.waybar_background.to_hex(),
    )
}

/// walker reads named colors from its own layout CSS, so a theme provides only
/// these `@define-color` values (matching Omarchy's `walker.css` template /
/// the installed themes). `selected-text` is the selection highlight — defining
/// it with the wrong name leaves the selected entry unstyled.
fn walker_css(p: &ThemePalette) -> String {
    format!(
        "/* Generated by omarchy-theme-studio */\n\
         @define-color selected-text {sel};\n\
         @define-color text {fg};\n\
         @define-color base {bg};\n\
         @define-color border {border};\n\
         @define-color foreground {fg};\n\
         @define-color background {bg};\n",
        sel = p.launcher_selected_background.to_hex(),
        fg = p.launcher_foreground.to_hex(),
        bg = p.launcher_background.to_hex(),
        border = p.accent.to_hex(),
    )
}

/// Hyprlock (lock screen) colors. `~/.config/hypr/hyprlock.conf` does
/// `source = current/theme/hyprlock.conf`, so omitting this leaves the lock
/// screen unthemed (black & white). Hyprland's `rgba()` takes decimal channels.
fn hyprlock_conf(p: &ThemePalette) -> String {
    let d = |c: RgbColor| format!("{}, {}, {}", c.r, c.g, c.b);
    format!(
        "# Generated by omarchy-theme-studio\n\
         $color = rgba({bg}, 1.0)\n\
         $inner_color = rgba({bg}, 0.8)\n\
         $outer_color = rgba({accent}, 1.0)\n\
         $font_color = rgba({fg}, 1.0)\n\
         $placeholder_color = rgba({fg}, 0.7)\n\
         $check_color = rgba({accent2}, 1.0)\n",
        bg = d(p.background),
        accent = d(p.accent),
        fg = d(p.foreground),
        accent2 = d(p.accent2),
    )
}

fn ghostty_conf(p: &ThemePalette) -> String {
    format!(
        "# Generated by omarchy-theme-studio\n\
         background = {bg}\n\
         foreground = {fg}\n\
         cursor-color = {cursor}\n\
         selection-background = {sel_bg}\n\
         selection-foreground = {sel_fg}\n",
        bg = p.terminal_background.to_hex(),
        fg = p.terminal_foreground.to_hex(),
        cursor = p.accent.to_hex(),
        sel_bg = p.accent.to_hex(),
        sel_fg = p.terminal_background.to_hex(),
    )
}

fn readme_md(name: &str, p: &ThemePalette) -> String {
    let row = |label: &str, c: RgbColor| format!("| {label} | `{}` |\n", c.to_hex());
    let mut s = String::new();
    s.push_str(&format!("# {name}\n\n"));
    s.push_str("An Omarchy desktop theme generated by [omarchy-theme-studio](https://github.com/allisonhere/omarchy-theme-studio).\n\n");
    s.push_str("Install by copying this directory into `~/.config/omarchy/themes/` and running:\n\n");
    s.push_str(&format!("```sh\nomarchy-theme-set {name}\n```\n\n"));
    s.push_str("## Palette\n\n| Field | Color |\n| --- | --- |\n");
    s.push_str(&row("Background", p.background));
    s.push_str(&row("Foreground", p.foreground));
    s.push_str(&row("Accent", p.accent));
    s.push_str(&row("Accent 2", p.accent2));
    s.push_str(&row("Active border", p.active_border));
    s.push_str(&row("Inactive border", p.inactive_border));
    s.push_str(&row("Waybar background", p.waybar_background));
    s.push_str(&row("Waybar foreground", p.waybar_foreground));
    s.push_str(&row("Waybar active workspace", p.waybar_active_workspace));
    s.push_str(&row("Launcher background", p.launcher_background));
    s.push_str(&row("Launcher foreground", p.launcher_foreground));
    s.push_str(&row("Launcher selected bg", p.launcher_selected_background));
    s.push_str(&row("Launcher selected fg", p.launcher_selected_foreground));
    s.push_str(&row("Terminal background", p.terminal_background));
    s.push_str(&row("Terminal foreground", p.terminal_foreground));
    s.push_str(&row("Notification background", p.notification_background));
    s.push_str(&row("Notification border", p.notification_border));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemePalette;

    #[test]
    fn palette_json_round_trips() {
        let mut p = ThemePalette::default();
        p.accent = RgbColor::new(1, 2, 3);
        let json = palette_json(&p).unwrap();
        let back: ThemePalette = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn hyprland_conf_uses_bare_hex_for_active_border() {
        let mut p = ThemePalette::default();
        p.active_border = RgbColor::new(0xaa, 0xbb, 0xcc);
        let conf = hyprland_conf(&p);
        assert!(conf.contains("rgb(aabbcc)"), "got: {conf}");
        assert!(!conf.contains("#aabbcc"), "hyprland hex must not include '#'");
    }

    #[test]
    fn colors_toml_has_core_keys() {
        let conf = colors_toml(&ThemePalette::default());
        for key in ["background", "foreground", "accent", "accent_alt"] {
            assert!(conf.contains(&format!("{key} = ")), "missing {key} in {conf}");
        }
    }

    #[test]
    fn hyprlock_conf_defines_color_vars_in_decimal_rgba() {
        let mut p = ThemePalette::default();
        p.background = RgbColor::new(10, 20, 30);
        let conf = hyprlock_conf(&p);
        assert!(conf.contains("$color = rgba(10, 20, 30, 1.0)"), "got: {conf}");
        assert!(conf.contains("$check_color = rgba("));
        // Hyprland rgba() here is decimal — no hex color literals.
        assert!(!conf.contains("rgba(#"));
    }

    #[test]
    fn css_files_match_omarchy_schema() {
        let mut p = ThemePalette::default();
        p.waybar_background = RgbColor::new(0x12, 0x34, 0x56);
        p.launcher_selected_background = RgbColor::new(0x65, 0x43, 0x21);

        let waybar = waybar_css(&p);
        assert!(waybar.contains("@define-color background #123456"));
        assert!(!waybar.contains("window#waybar"), "theme waybar.css must not carry selectors");

        let walker = walker_css(&p);
        // The selection highlight must be defined under the name walker references.
        assert!(walker.contains("@define-color selected-text #654321"), "got: {walker}");
        assert!(!walker.contains("#entry"), "theme walker.css must not carry selectors");
    }

    #[test]
    fn export_writes_full_theme_dir_and_round_trips() {
        let tmp = std::env::temp_dir().join(format!(
            "ots-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cm = ConfigManager::with_themes_dir(tmp.clone());

        let mut palette = ThemePalette::default();
        palette.accent = RgbColor::new(0x11, 0x22, 0x33);
        let theme = Theme::new("verify-theme", palette);

        let dir = cm.export_theme(&theme).expect("export should succeed");

        for file in [
            "palette.json",
            "colors.toml",
            "hyprland.conf",
            "hyprlock.conf",
            "waybar.css",
            "walker.css",
            "ghostty.conf",
            "README.md",
        ] {
            let path = dir.join(file);
            assert!(path.is_file(), "missing {file}");
            assert!(
                fs::metadata(&path).unwrap().len() > 0,
                "{file} should be non-empty"
            );
        }

        // Round-trips through palette.json.
        let listed = cm.list_themes().expect("list");
        assert!(listed.contains(&"verify-theme".to_string()));
        let loaded = cm.load_theme("verify-theme").expect("load");
        assert_eq!(loaded.palette, theme.palette);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn apply_honors_ots_apply_cmd_override() {
        let cm = ConfigManager::with_themes_dir(std::env::temp_dir());
        // `true` ignores its argument and exits 0 — a hermetic stand-in for the
        // real apply command / SSH wrapper.
        // SAFETY: single-threaded within this test; we clear it immediately after.
        std::env::set_var("OTS_APPLY_CMD", "true");
        assert!(cm.apply_available());
        let result = cm.apply_theme("whatever");
        std::env::remove_var("OTS_APPLY_CMD");
        assert!(result.is_ok(), "override apply should succeed: {result:?}");
    }
}
