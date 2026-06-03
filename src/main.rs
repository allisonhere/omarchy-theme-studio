mod config;
mod theme;
mod ui;
mod update;

use ui::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("export") => run_export(&args[1..]),
        Some("-h") | Some("--help") => {
            print_usage();
            Ok(())
        }
        _ => {
            let app = App::new();
            ui::run(app)?;
            Ok(())
        }
    }
}

/// Headless export: `omarchy-theme-studio export <name> [preset]`
/// Writes a theme directory under ~/.config/omarchy/themes/ without the TUI.
fn run_export(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(name) = args.first() else {
        print_usage();
        std::process::exit(2);
    };
    let slug = ui::normalize_theme_name(name);
    if slug.is_empty() {
        eprintln!("error: theme name must contain letters or numbers");
        std::process::exit(2);
    }
    let palette = match args.get(1) {
        Some(preset) => theme::presets::get(preset).ok_or_else(|| {
            let names: Vec<_> = theme::presets::builtins().into_iter().map(|(n, _)| n).collect();
            format!("unknown preset '{preset}' (available: {})", names.join(", "))
        })?,
        None => theme::ThemePalette::default(),
    };
    let theme = theme::Theme::new(slug, palette);
    let dir = config::ConfigManager::new().export_theme(&theme)?;
    println!("{}", dir.display());
    Ok(())
}

fn print_usage() {
    eprintln!(
        "omarchy-theme-studio — Omarchy desktop theme editor\n\
         USAGE:\n\
         \x20   omarchy-theme-studio                 launch the TUI editor\n\
         \x20   omarchy-theme-studio export <name> [preset]\n\
         \x20                                        write a theme dir (no TUI)\n\
         \n\
         PRESETS: tokyo-night, nord, gruvbox, rose-pine (default: built-in dark)"
    );
}
