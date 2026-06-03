use super::{hex, ThemePalette};

/// Built-in starter palettes, surfaced in the loader as `[B]` themes.
pub fn builtins() -> Vec<(&'static str, ThemePalette)> {
    vec![
        ("tokyo-night", tokyo_night()),
        ("nord", nord()),
        ("gruvbox", gruvbox()),
        ("rose-pine", rose_pine()),
    ]
}

/// Look up a built-in palette by name.
pub fn get(name: &str) -> Option<ThemePalette> {
    builtins()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, p)| p)
}

fn tokyo_night() -> ThemePalette {
    // Matches ThemePalette::default(), kept explicit for clarity.
    ThemePalette::default()
}

fn nord() -> ThemePalette {
    ThemePalette {
        background: hex("#2e3440"),
        foreground: hex("#d8dee9"),
        accent: hex("#88c0d0"),
        accent2: hex("#81a1c1"),
        active_border: hex("#88c0d0"),
        inactive_border: hex("#3b4252"),
        waybar_background: hex("#2e3440"),
        waybar_foreground: hex("#d8dee9"),
        waybar_active_workspace: hex("#88c0d0"),
        launcher_background: hex("#2e3440"),
        launcher_foreground: hex("#d8dee9"),
        launcher_selected_background: hex("#88c0d0"),
        launcher_selected_foreground: hex("#2e3440"),
        terminal_background: hex("#2e3440"),
        terminal_foreground: hex("#d8dee9"),
        notification_background: hex("#3b4252"),
        notification_border: hex("#88c0d0"),
    }
}

fn gruvbox() -> ThemePalette {
    ThemePalette {
        background: hex("#282828"),
        foreground: hex("#ebdbb2"),
        accent: hex("#fabd2f"),
        accent2: hex("#b8bb26"),
        active_border: hex("#fabd2f"),
        inactive_border: hex("#3c3836"),
        waybar_background: hex("#1d2021"),
        waybar_foreground: hex("#ebdbb2"),
        waybar_active_workspace: hex("#fabd2f"),
        launcher_background: hex("#282828"),
        launcher_foreground: hex("#ebdbb2"),
        launcher_selected_background: hex("#fabd2f"),
        launcher_selected_foreground: hex("#282828"),
        terminal_background: hex("#282828"),
        terminal_foreground: hex("#ebdbb2"),
        notification_background: hex("#3c3836"),
        notification_border: hex("#fabd2f"),
    }
}

fn rose_pine() -> ThemePalette {
    ThemePalette {
        background: hex("#191724"),
        foreground: hex("#e0def4"),
        accent: hex("#ebbcba"),
        accent2: hex("#c4a7e7"),
        active_border: hex("#ebbcba"),
        inactive_border: hex("#26233a"),
        waybar_background: hex("#1f1d2e"),
        waybar_foreground: hex("#e0def4"),
        waybar_active_workspace: hex("#ebbcba"),
        launcher_background: hex("#191724"),
        launcher_foreground: hex("#e0def4"),
        launcher_selected_background: hex("#ebbcba"),
        launcher_selected_foreground: hex("#191724"),
        terminal_background: hex("#191724"),
        terminal_foreground: hex("#e0def4"),
        notification_background: hex("#26233a"),
        notification_border: hex("#ebbcba"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_unique_and_resolvable() {
        let names: Vec<_> = builtins().into_iter().map(|(n, _)| n).collect();
        for name in &names {
            assert!(get(name).is_some(), "{name} should resolve");
        }
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "preset names must be unique");
    }
}
