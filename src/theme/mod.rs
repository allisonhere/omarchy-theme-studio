use serde::{Deserialize, Serialize};

pub mod presets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Hex digits with no leading `#` (Hyprland's `rgb(RRGGBB)` form, ghostty, etc.).
    pub fn to_hex_bare(self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Perceptual luminance in 0.0..=1.0 (Rec. 709 weights).
    pub fn luminance(&self) -> f32 {
        (0.2126 * f32::from(self.r) + 0.7152 * f32::from(self.g) + 0.0722 * f32::from(self.b)) / 255.0
    }

    /// Lighten (positive) or darken (negative) each channel by `amount` (0..=255).
    pub fn shade(&self, amount: i16) -> RgbColor {
        let adj = |c: u8| (i16::from(c) + amount).clamp(0, 255) as u8;
        RgbColor::new(adj(self.r), adj(self.g), adj(self.b))
    }

    pub fn saturating_add(self, delta: i8) -> u8 {
        let val = i32::from(self) + i32::from(delta);
        val.clamp(0, 255) as u8
    }

    pub fn saturating_add_unsigned(self, delta: u8) -> u8 {
        let val = i32::from(self) + i32::from(delta);
        val.clamp(0, 255) as u8
    }
}

impl From<RgbColor> for i32 {
    fn from(color: RgbColor) -> Self {
        i32::from(color.r) * 256 * 256 + i32::from(color.g) * 256 + i32::from(color.b)
    }
}

impl From<RgbColor> for u8 {
    fn from(color: RgbColor) -> Self {
        color.r
    }
}

/// Convenience for building palettes from hex literals. Panics on an invalid
/// literal, which only ever happens for hard-coded constants in this crate.
pub fn hex(s: &str) -> RgbColor {
    RgbColor::from_hex(s).unwrap_or_else(|| panic!("invalid hex literal: {s}"))
}

/// A flat, app-neutral desktop color palette. Each field is a single color so
/// the editor, undo stack, yank/paste, and color picker all operate uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePalette {
    pub background: RgbColor,
    pub foreground: RgbColor,
    pub accent: RgbColor,
    pub accent2: RgbColor,
    pub active_border: RgbColor,
    pub inactive_border: RgbColor,
    pub waybar_background: RgbColor,
    pub waybar_foreground: RgbColor,
    pub waybar_active_workspace: RgbColor,
    pub launcher_background: RgbColor,
    pub launcher_foreground: RgbColor,
    pub launcher_selected_background: RgbColor,
    pub launcher_selected_foreground: RgbColor,
    pub terminal_background: RgbColor,
    pub terminal_foreground: RgbColor,
    pub notification_background: RgbColor,
    pub notification_border: RgbColor,
}

impl Default for ThemePalette {
    fn default() -> Self {
        // Tokyo Night-ish dark default.
        Self {
            background: hex("#1a1b26"),
            foreground: hex("#c0caf5"),
            accent: hex("#7aa2f7"),
            accent2: hex("#bb9af7"),
            active_border: hex("#7aa2f7"),
            inactive_border: hex("#292e42"),
            waybar_background: hex("#16161e"),
            waybar_foreground: hex("#c0caf5"),
            waybar_active_workspace: hex("#7aa2f7"),
            launcher_background: hex("#1a1b26"),
            launcher_foreground: hex("#c0caf5"),
            launcher_selected_background: hex("#7aa2f7"),
            launcher_selected_foreground: hex("#1a1b26"),
            terminal_background: hex("#1a1b26"),
            terminal_foreground: hex("#c0caf5"),
            notification_background: hex("#1a1b26"),
            notification_border: hex("#7aa2f7"),
        }
    }
}

/// A named palette — the unit the loader, exporter, and editor pass around.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub palette: ThemePalette,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: String::from("untitled"),
            palette: ThemePalette::default(),
        }
    }
}

impl Theme {
    pub fn new(name: impl Into<String>, palette: ThemePalette) -> Self {
        Self {
            name: name.into(),
            palette,
        }
    }
}

/// Editable target: which single color of the palette is currently selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaletteField {
    Background,
    Foreground,
    Accent,
    Accent2,
    ActiveBorder,
    InactiveBorder,
    WaybarBackground,
    WaybarForeground,
    WaybarActiveWorkspace,
    LauncherBackground,
    LauncherForeground,
    LauncherSelectedBackground,
    LauncherSelectedForeground,
    TerminalBackground,
    TerminalForeground,
    NotificationBackground,
    NotificationBorder,
}

impl PaletteField {
    /// Region-grouped order — drives linear navigation and the field list.
    pub fn all() -> &'static [PaletteField] {
        use PaletteField::*;
        &[
            Background,
            Foreground,
            Accent,
            Accent2,
            ActiveBorder,
            InactiveBorder,
            WaybarBackground,
            WaybarForeground,
            WaybarActiveWorkspace,
            LauncherBackground,
            LauncherForeground,
            LauncherSelectedBackground,
            LauncherSelectedForeground,
            TerminalBackground,
            TerminalForeground,
            NotificationBackground,
            NotificationBorder,
        ]
    }

    /// The region group this field belongs to.
    pub fn group(&self) -> PaletteGroup {
        use PaletteField::*;
        match self {
            Background | Foreground | Accent | Accent2 => PaletteGroup::Desktop,
            ActiveBorder | InactiveBorder => PaletteGroup::Windows,
            WaybarBackground | WaybarForeground | WaybarActiveWorkspace => PaletteGroup::Waybar,
            LauncherBackground | LauncherForeground | LauncherSelectedBackground
            | LauncherSelectedForeground => PaletteGroup::Launcher,
            TerminalBackground | TerminalForeground => PaletteGroup::Terminal,
            NotificationBackground | NotificationBorder => PaletteGroup::Notification,
        }
    }

    pub fn label(&self) -> &'static str {
        use PaletteField::*;
        match self {
            Background => "Background",
            Foreground => "Foreground",
            Accent => "Accent",
            Accent2 => "Accent 2",
            ActiveBorder => "Active border",
            InactiveBorder => "Inactive border",
            WaybarBackground => "Waybar background",
            WaybarForeground => "Waybar foreground",
            WaybarActiveWorkspace => "Waybar active workspace",
            LauncherBackground => "Launcher background",
            LauncherForeground => "Launcher foreground",
            LauncherSelectedBackground => "Launcher selected bg",
            LauncherSelectedForeground => "Launcher selected fg",
            TerminalBackground => "Terminal background",
            TerminalForeground => "Terminal foreground",
            NotificationBackground => "Notification background",
            NotificationBorder => "Notification border",
        }
    }

    pub fn get(&self, p: &ThemePalette) -> RgbColor {
        *self.get_ref(p)
    }

    fn get_ref<'a>(&self, p: &'a ThemePalette) -> &'a RgbColor {
        use PaletteField::*;
        match self {
            Background => &p.background,
            Foreground => &p.foreground,
            Accent => &p.accent,
            Accent2 => &p.accent2,
            ActiveBorder => &p.active_border,
            InactiveBorder => &p.inactive_border,
            WaybarBackground => &p.waybar_background,
            WaybarForeground => &p.waybar_foreground,
            WaybarActiveWorkspace => &p.waybar_active_workspace,
            LauncherBackground => &p.launcher_background,
            LauncherForeground => &p.launcher_foreground,
            LauncherSelectedBackground => &p.launcher_selected_background,
            LauncherSelectedForeground => &p.launcher_selected_foreground,
            TerminalBackground => &p.terminal_background,
            TerminalForeground => &p.terminal_foreground,
            NotificationBackground => &p.notification_background,
            NotificationBorder => &p.notification_border,
        }
    }

    pub fn get_mut<'a>(&self, p: &'a mut ThemePalette) -> &'a mut RgbColor {
        use PaletteField::*;
        match self {
            Background => &mut p.background,
            Foreground => &mut p.foreground,
            Accent => &mut p.accent,
            Accent2 => &mut p.accent2,
            ActiveBorder => &mut p.active_border,
            InactiveBorder => &mut p.inactive_border,
            WaybarBackground => &mut p.waybar_background,
            WaybarForeground => &mut p.waybar_foreground,
            WaybarActiveWorkspace => &mut p.waybar_active_workspace,
            LauncherBackground => &mut p.launcher_background,
            LauncherForeground => &mut p.launcher_foreground,
            LauncherSelectedBackground => &mut p.launcher_selected_background,
            LauncherSelectedForeground => &mut p.launcher_selected_foreground,
            TerminalBackground => &mut p.terminal_background,
            TerminalForeground => &mut p.terminal_foreground,
            NotificationBackground => &mut p.notification_background,
            NotificationBorder => &mut p.notification_border,
        }
    }
}

/// A region grouping of palette fields — drives the two-tier field selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaletteGroup {
    Desktop,
    Windows,
    Waybar,
    Launcher,
    Terminal,
    Notification,
}

impl PaletteGroup {
    pub fn all() -> &'static [PaletteGroup; 6] {
        use PaletteGroup::*;
        &[Desktop, Windows, Waybar, Launcher, Terminal, Notification]
    }

    pub fn label(&self) -> &'static str {
        use PaletteGroup::*;
        match self {
            Desktop => "Desktop",
            Windows => "Windows",
            Waybar => "Waybar",
            Launcher => "Launcher",
            Terminal => "Terminal",
            Notification => "Notification",
        }
    }

    pub fn fields(&self) -> &'static [PaletteField] {
        use PaletteField::*;
        match self {
            PaletteGroup::Desktop => &[Background, Foreground, Accent, Accent2],
            PaletteGroup::Windows => &[ActiveBorder, InactiveBorder],
            PaletteGroup::Waybar => &[WaybarBackground, WaybarForeground, WaybarActiveWorkspace],
            PaletteGroup::Launcher => &[
                LauncherBackground,
                LauncherForeground,
                LauncherSelectedBackground,
                LauncherSelectedForeground,
            ],
            PaletteGroup::Terminal => &[TerminalBackground, TerminalForeground],
            PaletteGroup::Notification => &[NotificationBackground, NotificationBorder],
        }
    }

    pub fn index(&self) -> usize {
        Self::all().iter().position(|g| g == self).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_field_get_set_round_trips() {
        let mut p = ThemePalette::default();
        for field in PaletteField::all() {
            let c = RgbColor::new(10, 20, 30);
            *field.get_mut(&mut p) = c;
            assert_eq!(field.get(&p), c, "field {:?} should round-trip", field);
        }
    }

    #[test]
    fn hex_round_trips() {
        let c = hex("#7aa2f7");
        assert_eq!(c.to_hex(), "#7aa2f7");
        assert_eq!(c.to_hex_bare(), "7aa2f7");
    }

    #[test]
    fn groups_partition_all_fields_in_order() {
        let concatenated: Vec<PaletteField> = PaletteGroup::all()
            .iter()
            .flat_map(|g| g.fields().iter().copied())
            .collect();
        assert_eq!(concatenated.as_slice(), PaletteField::all());
        // Every field maps back to the group that lists it.
        for g in PaletteGroup::all() {
            for f in g.fields() {
                assert_eq!(f.group(), *g, "field {f:?} should belong to {g:?}");
            }
        }
    }
}
