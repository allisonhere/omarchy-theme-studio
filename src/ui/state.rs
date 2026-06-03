use crate::theme::{PaletteField, PaletteGroup, RgbColor, Theme};
use crate::ui::color_picker::ColorEditor;
use crate::update::UpdateMsg;

#[derive(Debug, Clone)]
pub enum ThemeEntry {
    User(String),
    Builtin(&'static str),
}

impl ThemeEntry {
    pub fn name(&self) -> &str {
        match self {
            Self::User(n) => n.as_str(),
            Self::Builtin(n) => n,
        }
    }
    pub fn is_builtin(&self) -> bool {
        matches!(self, Self::Builtin(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeFilter {
    All,
    Builtin,
    Saved,
}

pub struct App {
    pub theme: Theme,
    pub selected: PaletteField,
    pub config_manager: crate::config::ConfigManager,
    pub apply_available: bool,
    pub message: Option<String>,
    pub input_mode: InputMode,
    pub color_editor: ColorEditor,
    /// Color of the selected field when the picker was opened (for cancel/undo).
    pub original_color: Option<RgbColor>,
    pub theme_name_input: String,
    pub all_themes: Vec<ThemeEntry>,
    pub loadable_themes: Vec<ThemeEntry>,
    pub theme_filter: ThemeFilter,
    pub selected_theme_index: usize,
    pub dirty: bool,
    pub original_theme: Option<Theme>,
    pub theme_swatches: std::collections::HashMap<String, [RgbColor; 4]>,
    pub theme_search_query: String,
    pub search_focused: bool,
    pub help_scroll: u16,
    pub clipboard_color: Option<RgbColor>,
    pub undo_stack: Vec<(PaletteField, RgbColor)>,
    pub loader_action_index: usize,
    // Fuzzy field palette ( / )
    pub field_search_query: String,
    pub field_search_index: usize,
    // Self-update
    pub update_status: UpdateStatus,
    pub update_rx: Option<std::sync::mpsc::Receiver<UpdateMsg>>,
    pub restart_after_exit: bool,
    /// Set after shelling out to an external command so the event loop forces a
    /// full terminal repaint (the child may have drawn over the alternate screen).
    pub force_redraw: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Preview,
    ColorPicker,
    ThemeNameInput,
    ThemeLoad,
    ThemeLoadRename,
    ThemeLoadDeleteConfirm,
    ApplyConfirm,
    UpdateRestartConfirm,
    FieldSearch,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available(String),
    Downloading,
    Done,
    Failed(String),
}

fn theme_swatches(palette: &crate::theme::ThemePalette) -> [RgbColor; 4] {
    [
        palette.background,
        palette.accent,
        palette.accent2,
        palette.active_border,
    ]
}

fn load_entry(
    entry: &ThemeEntry,
    config_manager: &crate::config::ConfigManager,
) -> Result<Theme, crate::config::ConfigError> {
    match entry {
        ThemeEntry::User(n) => config_manager.load_theme(n),
        ThemeEntry::Builtin(n) => crate::theme::presets::get(n)
            .map(|palette| Theme::new(*n, palette))
            .ok_or_else(|| crate::config::ConfigError::ThemeNotFound(n.to_string())),
    }
}

impl Default for App {
    fn default() -> Self {
        let config_manager = crate::config::ConfigManager::new();
        let apply_available = config_manager.apply_available();
        Self {
            theme: Theme::default(),
            selected: PaletteField::Background,
            config_manager,
            apply_available,
            message: None,
            input_mode: InputMode::Preview,
            color_editor: ColorEditor::from_rgb(200, 200, 200),
            original_color: None,
            theme_name_input: String::from("untitled"),
            all_themes: Vec::new(),
            loadable_themes: Vec::new(),
            theme_filter: ThemeFilter::All,
            selected_theme_index: 0,
            dirty: false,
            original_theme: None,
            theme_swatches: std::collections::HashMap::new(),
            theme_search_query: String::new(),
            search_focused: false,
            help_scroll: 0,
            clipboard_color: None,
            undo_stack: Vec::new(),
            loader_action_index: 0,
            field_search_query: String::new(),
            field_search_index: 0,
            update_status: UpdateStatus::Idle,
            update_rx: None,
            restart_after_exit: false,
            force_redraw: false,
        }
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = Self::default();
        app.sync_theme_name_input();
        app
    }

    // ── Field selection / navigation ─────────────────────────────────────────

    fn group_field_index(&self) -> usize {
        self.selected
            .group()
            .fields()
            .iter()
            .position(|f| *f == self.selected)
            .unwrap_or(0)
    }

    /// ↑/k — previous field within the current group (wraps inside the group).
    pub fn prev_field_in_group(&mut self) {
        let fields = self.selected.group().fields();
        let idx = self.group_field_index();
        let next = if idx == 0 { fields.len() - 1 } else { idx - 1 };
        self.selected = fields[next];
    }

    /// ↓/j — next field within the current group (wraps inside the group).
    pub fn next_field_in_group(&mut self) {
        let fields = self.selected.group().fields();
        let idx = self.group_field_index();
        self.selected = fields[(idx + 1) % fields.len()];
    }

    pub fn select_group(&mut self, group: PaletteGroup) {
        self.selected = group.fields()[0];
    }

    /// Number keys 1–6 → jump straight to that group's first field.
    pub fn select_group_index(&mut self, n: usize) {
        if let Some(g) = PaletteGroup::all().get(n) {
            self.select_group(*g);
        }
    }

    pub fn next_group(&mut self) {
        let groups = PaletteGroup::all();
        let idx = self.selected.group().index();
        self.select_group(groups[(idx + 1) % groups.len()]);
    }

    pub fn prev_group(&mut self) {
        let groups = PaletteGroup::all();
        let idx = self.selected.group().index();
        let next = if idx == 0 { groups.len() - 1 } else { idx - 1 };
        self.select_group(groups[next]);
    }

    // ── Fuzzy field palette ( / ) ────────────────────────────────────────────

    pub fn open_field_search(&mut self) {
        self.field_search_query.clear();
        self.field_search_index = 0;
        self.input_mode = InputMode::FieldSearch;
        self.message = None;
    }

    /// Fields whose "<group> <label>" fuzzily matches the query (all when empty).
    pub fn filtered_fields(&self) -> Vec<PaletteField> {
        let q = self.field_search_query.to_ascii_lowercase();
        PaletteField::all()
            .iter()
            .copied()
            .filter(|f| {
                if q.is_empty() {
                    return true;
                }
                // Substring match over "<group> <label>" — predictable for field
                // names (e.g. "laun" → Launcher only, not scattered subsequences).
                let hay = format!("{} {}", f.group().label(), f.label()).to_ascii_lowercase();
                hay.contains(&q)
            })
            .collect()
    }

    pub fn push_field_search_char(&mut self, c: char) {
        self.field_search_query.push(c);
        self.field_search_index = 0;
    }

    pub fn pop_field_search_char(&mut self) {
        self.field_search_query.pop();
        self.field_search_index = 0;
    }

    pub fn move_field_search(&mut self, delta: i32) {
        let len = self.filtered_fields().len();
        if len == 0 {
            self.field_search_index = 0;
            return;
        }
        let cur = self.field_search_index.min(len - 1) as i32;
        self.field_search_index = (cur + delta).rem_euclid(len as i32) as usize;
    }

    pub fn commit_field_search(&mut self) {
        let matches = self.filtered_fields();
        if let Some(f) = matches.get(self.field_search_index).copied() {
            self.selected = f;
            self.message = Some(format!("→ {}", f.label()));
        }
        self.input_mode = InputMode::Preview;
    }

    pub fn cancel_field_search(&mut self) {
        self.input_mode = InputMode::Preview;
        self.message = None;
    }

    pub fn current_color(&self) -> RgbColor {
        self.selected.get(&self.theme.palette)
    }

    fn set_current_color(&mut self, color: RgbColor) {
        *self.selected.get_mut(&mut self.theme.palette) = color;
        self.dirty = true;
    }

    // ── Save / export ────────────────────────────────────────────────────────

    pub fn open_theme_name_input(&mut self) {
        self.sync_theme_name_input();
        self.input_mode = InputMode::ThemeNameInput;
        self.message = Some(String::from("Enter a theme name, then press Enter to export"));
    }

    pub fn save_theme_as_input_name(&mut self) {
        let normalized = normalize_theme_name(&self.theme_name_input);
        if normalized.is_empty() {
            self.message = Some(String::from("✗ Theme name must contain letters or numbers"));
            return;
        }
        self.theme.name = normalized;
        self.sync_theme_name_input();
        self.export_theme();
        self.refresh_theme_list();
        self.input_mode = InputMode::Preview;
    }

    fn export_theme(&mut self) {
        match self.config_manager.export_theme(&self.theme) {
            Ok(dir) => {
                self.message = Some(format!("✓ Exported to {}", dir.display()));
                self.dirty = false;
            }
            Err(e) => self.message = Some(format!("✗ Error: {}", e)),
        }
    }

    // ── Apply (gated, confirmed) ─────────────────────────────────────────────

    pub fn begin_apply(&mut self) {
        if !self.apply_available {
            self.message = Some(String::from("omarchy-theme-set not found — export only"));
            return;
        }
        let name = normalize_theme_name(&self.theme.name);
        if name.is_empty() {
            self.open_theme_name_input();
            return;
        }
        self.input_mode = InputMode::ApplyConfirm;
        self.message = Some(format!(
            "Apply \"{}\" to the live desktop with omarchy-theme-set? y = yes, n = no",
            name
        ));
    }

    pub fn confirm_apply(&mut self) {
        // Make sure the theme directory is on disk before applying.
        self.export_theme();
        let name = self.theme.name.clone();
        match self.config_manager.apply_theme(&name) {
            Ok(()) => self.message = Some(format!("✓ Applied \"{}\"", name)),
            Err(e) => self.message = Some(format!("✗ {}", e)),
        }
        self.input_mode = InputMode::Preview;
        self.force_redraw = true;
    }

    pub fn cancel_apply(&mut self) {
        self.input_mode = InputMode::Preview;
        self.message = Some(String::from("Apply cancelled"));
    }

    // ── Theme loader ─────────────────────────────────────────────────────────

    pub fn open_theme_load_dialog(&mut self) {
        self.theme_search_query = String::new();
        self.search_focused = false;
        self.theme_filter = ThemeFilter::All;
        self.original_theme = Some(self.theme.clone());
        self.refresh_theme_list();
        self.selected_theme_index = self
            .loadable_themes
            .iter()
            .position(|e| e.name() == self.theme.name)
            .unwrap_or(0);
        self.theme_swatches = self
            .all_themes
            .iter()
            .map(|entry| {
                let sw = match load_entry(entry, &self.config_manager) {
                    Ok(t) => theme_swatches(&t.palette),
                    Err(_) => [RgbColor::new(50, 50, 50); 4],
                };
                (entry.name().to_string(), sw)
            })
            .collect();
        if let Some(entry) = self.loadable_themes.get(self.selected_theme_index).cloned() {
            if let Ok(t) = load_entry(&entry, &self.config_manager) {
                self.theme = t;
            }
        }
        self.input_mode = InputMode::ThemeLoad;
        self.message = Some(String::from("Select a theme to load"));
    }

    pub fn load_selected_theme(&mut self) {
        if let Some(entry) = self.loadable_themes.get(self.selected_theme_index).cloned() {
            let name = entry.name().to_string();
            match load_entry(&entry, &self.config_manager) {
                Ok(t) => self.theme = t,
                Err(e) => {
                    self.message = Some(format!("✗ Error loading \"{}\": {}", name, e));
                    return;
                }
            }
            self.sync_theme_name_input();
            self.original_theme = None;
            self.dirty = false;
            self.message = Some(format!("✓ Loaded: {}", name));
            self.input_mode = InputMode::Preview;
        }
    }

    pub fn cancel_theme_load(&mut self) {
        if let Some(original) = self.original_theme.take() {
            self.theme = original;
        }
        self.sync_theme_name_input();
        self.theme_search_query = String::new();
        self.search_focused = false;
        self.input_mode = InputMode::Preview;
        self.message = None;
    }

    pub fn refresh_theme_list(&mut self) {
        let user_themes: Vec<ThemeEntry> = match self.config_manager.list_themes() {
            Ok(mut names) => {
                names.sort();
                names.into_iter().map(ThemeEntry::User).collect()
            }
            Err(e) => {
                self.message = Some(format!("✗ Error listing themes: {}", e));
                Vec::new()
            }
        };

        let builtin_themes: Vec<ThemeEntry> = crate::theme::presets::builtins()
            .into_iter()
            .map(|(name, _)| ThemeEntry::Builtin(name))
            .collect();

        self.all_themes = user_themes.into_iter().chain(builtin_themes).collect();
        self.apply_filter_to_list();
    }

    pub fn apply_filter_to_list(&mut self) {
        let q = self.theme_search_query.trim_start_matches('/').to_ascii_lowercase();
        self.loadable_themes = self
            .all_themes
            .iter()
            .filter(|e| {
                let matches_filter = match self.theme_filter {
                    ThemeFilter::All => true,
                    ThemeFilter::Builtin => e.is_builtin(),
                    ThemeFilter::Saved => !e.is_builtin(),
                };
                let matches_search = q.is_empty() || fuzzy_match(&e.name().to_ascii_lowercase(), &q);
                matches_filter && matches_search
            })
            .cloned()
            .collect();

        if self.selected_theme_index >= self.loadable_themes.len() {
            self.selected_theme_index = self.loadable_themes.len().saturating_sub(1);
        }
    }

    pub fn set_theme_filter(&mut self, filter: ThemeFilter) {
        self.theme_filter = if self.theme_filter == filter { ThemeFilter::All } else { filter };
        self.apply_filter_to_list();
        self.selected_theme_index = 0;
        if let Some(entry) = self.loadable_themes.first().cloned() {
            if let Ok(t) = load_entry(&entry, &self.config_manager) {
                self.theme = t;
            }
        }
    }

    pub fn move_theme_selection_up(&mut self) {
        if self.loadable_themes.is_empty() {
            self.selected_theme_index = 0;
        } else if self.selected_theme_index == 0 {
            self.selected_theme_index = self.loadable_themes.len() - 1;
        } else {
            self.selected_theme_index -= 1;
        }
        self.preview_selected_theme();
    }

    pub fn move_theme_selection_down(&mut self) {
        if self.loadable_themes.is_empty() {
            self.selected_theme_index = 0;
        } else {
            self.selected_theme_index = (self.selected_theme_index + 1) % self.loadable_themes.len();
        }
        self.preview_selected_theme();
    }

    pub fn move_theme_selection_to(&mut self, index: usize) {
        self.selected_theme_index = index;
        self.preview_selected_theme();
    }

    fn preview_selected_theme(&mut self) {
        if let Some(entry) = self.loadable_themes.get(self.selected_theme_index).cloned() {
            if let Ok(t) = load_entry(&entry, &self.config_manager) {
                self.theme = t;
            }
        }
    }

    // ── Name input ───────────────────────────────────────────────────────────

    pub fn push_theme_name_char(&mut self, c: char) {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
            self.theme_name_input.push(c);
        }
    }

    pub fn pop_theme_name_char(&mut self) {
        self.theme_name_input.pop();
    }

    pub fn sync_theme_name_input(&mut self) {
        self.theme_name_input = self.theme.name.clone();
    }

    // ── Color picker ─────────────────────────────────────────────────────────

    pub fn apply_current_color(&mut self) {
        let color = self.color_editor.to_rgb();
        self.set_current_color(color);
    }

    pub fn open_color_picker(&mut self) {
        let color = self.current_color();
        self.original_color = Some(color);
        let previous_mode = self.color_editor.mode;
        self.color_editor = ColorEditor::from_rgb(color.r, color.g, color.b);
        if self.color_editor.mode != previous_mode {
            self.color_editor.toggle_mode();
        }
        self.input_mode = InputMode::ColorPicker;
    }

    pub fn close_color_picker(&mut self, save: bool) {
        if save {
            self.record_undo();
        } else if let Some(original) = self.original_color.take() {
            self.set_current_color(original);
        }
        self.original_color = None;
        self.input_mode = InputMode::Preview;
    }

    // ── Yank / paste / undo ──────────────────────────────────────────────────

    pub fn yank_color(&mut self) {
        let c = self.current_color();
        self.clipboard_color = Some(c);
        self.message = Some(format!("Yanked {}", c.to_hex()));
    }

    pub fn paste_color(&mut self) {
        if let Some(c) = self.clipboard_color {
            let before = self.current_color();
            self.push_undo(self.selected, before);
            self.set_current_color(c);
            self.message = Some(format!("Pasted {}", c.to_hex()));
        } else {
            self.message = Some(String::from("Nothing to paste"));
        }
    }

    fn push_undo(&mut self, field: PaletteField, color: RgbColor) {
        self.undo_stack.push((field, color));
        const MAX: usize = 64;
        if self.undo_stack.len() > MAX {
            self.undo_stack.remove(0);
        }
    }

    fn record_undo(&mut self) {
        if let Some(original) = self.original_color {
            if self.current_color() != original {
                let field = self.selected;
                self.push_undo(field, original);
            }
        }
    }

    pub fn undo_color(&mut self) {
        if let Some((field, color)) = self.undo_stack.pop() {
            self.selected = field;
            self.set_current_color(color);
            let remaining = self.undo_stack.len();
            self.message = Some(if remaining == 0 {
                String::from("Undone")
            } else {
                format!("Undone ({} more)", remaining)
            });
        } else {
            self.message = Some(String::from("Nothing to undo"));
        }
    }

    // ── Rename / delete saved themes ─────────────────────────────────────────

    pub fn begin_rename_selected_theme(&mut self) {
        if self
            .loadable_themes
            .get(self.selected_theme_index)
            .map(|e| e.is_builtin())
            .unwrap_or(true)
        {
            self.message = Some(String::from("Cannot rename built-in themes"));
            return;
        }
        self.loader_action_index = self.selected_theme_index;
        self.theme_name_input = self.loadable_themes[self.selected_theme_index].name().to_string();
        self.input_mode = InputMode::ThemeLoadRename;
    }

    pub fn commit_rename_theme(&mut self) {
        let new_name = normalize_theme_name(&self.theme_name_input);
        if new_name.is_empty() {
            self.message = Some(String::from("✗ Invalid name"));
            return;
        }
        let old_name = self
            .loadable_themes
            .get(self.loader_action_index)
            .map(|e| e.name().to_string())
            .unwrap_or_default();
        match self.config_manager.rename_theme(&old_name, &new_name) {
            Ok(()) => self.message = Some(format!("✓ Renamed to {}", new_name)),
            Err(e) => self.message = Some(format!("✗ {}", e)),
        }
        self.refresh_theme_list();
        self.input_mode = InputMode::ThemeLoad;
    }

    pub fn begin_delete_selected_theme(&mut self) {
        if self
            .loadable_themes
            .get(self.selected_theme_index)
            .map(|e| e.is_builtin())
            .unwrap_or(true)
        {
            self.message = Some(String::from("Cannot delete built-in themes"));
            return;
        }
        self.loader_action_index = self.selected_theme_index;
        let name = self.loadable_themes[self.selected_theme_index].name().to_string();
        self.message = Some(format!("Delete \"{}\"? y = confirm, n = cancel", name));
        self.input_mode = InputMode::ThemeLoadDeleteConfirm;
    }

    pub fn confirm_delete_theme(&mut self) {
        let name = self
            .loadable_themes
            .get(self.loader_action_index)
            .map(|e| e.name().to_string())
            .unwrap_or_default();
        match self.config_manager.delete_theme(&name) {
            Ok(()) => self.message = Some(format!("✓ Deleted \"{}\"", name)),
            Err(e) => self.message = Some(format!("✗ {}", e)),
        }
        self.refresh_theme_list();
        self.selected_theme_index = self
            .selected_theme_index
            .min(self.loadable_themes.len().saturating_sub(1));
        self.input_mode = InputMode::ThemeLoad;
    }

    // ── Self-update ──────────────────────────────────────────────────────────

    pub fn start_update_check(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.update_status = UpdateStatus::Checking;
        self.update_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(UpdateMsg::VersionChecked(crate::update::check_version()));
        });
    }

    pub fn start_self_update(&mut self) {
        match &self.update_status {
            UpdateStatus::Available(tag) => {
                let tag = tag.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                self.update_status = UpdateStatus::Downloading;
                self.update_rx = Some(rx);
                self.message = None;
                std::thread::spawn(move || {
                    let _ = tx.send(UpdateMsg::UpdateComplete(crate::update::download_and_replace(&tag)));
                });
            }
            UpdateStatus::Checking => {
                self.message = Some(String::from("Still checking for updates"));
            }
            UpdateStatus::UpToDate | UpdateStatus::Idle => {
                self.message = Some(String::from("No update available"));
            }
            UpdateStatus::Downloading => {
                self.message = Some(String::from("Update already in progress"));
            }
            UpdateStatus::Done => {
                self.message = Some(String::from("Update already installed; restart to apply"));
            }
            UpdateStatus::Failed(err) => {
                self.message = Some(format!("✗ Update unavailable: {}", err));
            }
        }
    }

    pub fn poll_update_channel(&mut self) {
        let Some(rx) = self.update_rx.as_ref() else {
            return;
        };

        match rx.try_recv() {
            Ok(msg) => {
                self.update_rx = None;
                self.message = None;
                match msg {
                    UpdateMsg::VersionChecked(Ok(Some(tag))) => {
                        self.update_status = UpdateStatus::Available(tag);
                    }
                    UpdateMsg::VersionChecked(Ok(None)) => {
                        self.update_status = UpdateStatus::UpToDate;
                    }
                    UpdateMsg::VersionChecked(Err(e)) => {
                        self.update_status = UpdateStatus::Failed(e);
                    }
                    UpdateMsg::UpdateComplete(Ok(())) => {
                        self.update_status = UpdateStatus::Done;
                        self.input_mode = InputMode::UpdateRestartConfirm;
                    }
                    UpdateMsg::UpdateComplete(Err(e)) => {
                        self.update_status = UpdateStatus::Failed(e);
                    }
                };
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.update_rx = None;
                self.message = None;
                self.update_status = UpdateStatus::Failed(String::from("update worker disconnected"));
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }

    pub fn defer_restart(&mut self) {
        self.input_mode = InputMode::Preview;
        self.update_status = UpdateStatus::Idle;
        self.message = None;
    }

    pub fn confirm_restart(&mut self) {
        self.restart_after_exit = true;
        self.input_mode = InputMode::Preview;
    }
}

fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let mut hay_chars = haystack.chars();
    for n in needle.chars() {
        if !hay_chars.any(|h| h == n) {
            return false;
        }
    }
    true
}

pub fn normalize_theme_name(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                Some(c.to_ascii_lowercase())
            } else if c.is_ascii_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_jump_selects_group_first_field() {
        let mut app = App::default();
        app.select_group_index(2); // Waybar
        assert_eq!(app.selected, PaletteGroup::Waybar.fields()[0]);
        assert_eq!(app.selected.group(), PaletteGroup::Waybar);
    }

    #[test]
    fn next_group_wraps_to_first() {
        let mut app = App::default();
        app.select_group(PaletteGroup::Notification);
        app.next_group();
        assert_eq!(app.selected.group(), PaletteGroup::Desktop);
    }

    #[test]
    fn field_nav_stays_within_group_and_wraps() {
        let mut app = App::default();
        app.select_group(PaletteGroup::Waybar); // 3 fields
        let start = app.selected;
        for _ in 0..3 {
            app.next_field_in_group();
            assert_eq!(app.selected.group(), PaletteGroup::Waybar, "must not spill out of group");
        }
        assert_eq!(app.selected, start, "should wrap after a full cycle");
    }

    #[test]
    fn filtered_fields_narrows_to_launcher() {
        let mut app = App::default();
        app.field_search_query = "laun".to_string();
        let m = app.filtered_fields();
        assert_eq!(m.len(), 4);
        assert!(m.iter().all(|f| f.group() == PaletteGroup::Launcher));
    }

    #[test]
    fn commit_field_search_selects_and_closes() {
        let mut app = App::default();
        app.open_field_search();
        for c in "active".chars() {
            app.push_field_search_char(c);
        }
        let expected = app.filtered_fields()[app.field_search_index];
        app.commit_field_search();
        assert_eq!(app.selected, expected);
        assert_eq!(app.input_mode, InputMode::Preview);
    }
}
