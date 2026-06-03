use crate::theme::{PaletteField, PaletteGroup, RgbColor};
use crate::ui::color_picker::{
    contrast_text, hsv_field_cell, picker_layout, ColorPickerFocus, ColorPickerMode, EditableField,
};
use crate::ui::state::{App, InputMode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

use super::state::normalize_theme_name;

// ── Chrome (theme-independent) colors used for status bar and overlays ───────
const CHROME_BG: Color = Color::Rgb(22, 22, 26);
const CHROME_FG: Color = Color::Rgb(212, 212, 230);
const CHROME_MUTED: Color = Color::Rgb(120, 120, 145);
const CHROME_ACCENT_BG: Color = Color::Rgb(97, 88, 150);
const CHROME_ACCENT_FG: Color = Color::Rgb(242, 240, 255);

pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let popup_width = width.min(area.width.saturating_sub(2)).max(1);
    let popup_height = height.min(area.height.saturating_sub(2)).max(1);
    let x = area.x + area.width.saturating_sub(popup_width) / 2;
    let y = area.y + area.height.saturating_sub(popup_height) / 2;
    Rect::new(x, y, popup_width, popup_height)
}

fn clip_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn tui_rgb(color: RgbColor) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Within-group ("leaf") label — the group tab supplies the context.
fn field_leaf_label(f: PaletteField) -> &'static str {
    use PaletteField::*;
    match f {
        Background | WaybarBackground | LauncherBackground | TerminalBackground
        | NotificationBackground => "Background",
        Foreground | WaybarForeground | LauncherForeground | TerminalForeground => "Foreground",
        Accent => "Accent",
        Accent2 => "Accent 2",
        ActiveBorder => "Active border",
        InactiveBorder => "Inactive border",
        WaybarActiveWorkspace => "Active workspace",
        LauncherSelectedBackground => "Selected bg",
        LauncherSelectedForeground => "Selected fg",
        NotificationBorder => "Border",
    }
}

/// Which preview region a palette field belongs to (drives selection highlight).
#[derive(PartialEq, Eq, Clone, Copy)]
enum Region {
    Desktop,
    ActiveWindow,
    InactiveWindow,
    Waybar,
    Launcher,
    Terminal,
    Notification,
}

fn region_of(field: PaletteField) -> Region {
    use PaletteField::*;
    match field {
        Background | Foreground | Accent | Accent2 => Region::Desktop,
        ActiveBorder => Region::ActiveWindow,
        InactiveBorder => Region::InactiveWindow,
        WaybarBackground | WaybarForeground | WaybarActiveWorkspace => Region::Waybar,
        LauncherBackground | LauncherForeground | LauncherSelectedBackground
        | LauncherSelectedForeground => Region::Launcher,
        TerminalBackground | TerminalForeground => Region::Terminal,
        NotificationBackground | NotificationBorder => Region::Notification,
    }
}

impl App {
    fn sel(&self, region: Region) -> bool {
        region_of(self.selected) == region
    }

    pub fn render(&self, frame: &mut Frame) {
        match self.input_mode {
            InputMode::Preview => self.render_preview(frame),
            InputMode::ColorPicker => {
                self.render_preview(frame);
                self.render_color_picker_overlay(frame);
            }
            InputMode::ThemeNameInput => {
                self.render_preview(frame);
                self.render_theme_name_input_overlay(frame);
            }
            InputMode::ApplyConfirm => {
                self.render_preview(frame);
                self.render_apply_confirm_overlay(frame);
            }
            InputMode::FieldSearch => {
                self.render_preview(frame);
                self.render_field_search_overlay(frame);
            }
            InputMode::ThemeLoad => {
                self.render_preview(frame);
                self.render_theme_load_overlay(frame);
            }
            InputMode::ThemeLoadRename => {
                self.render_preview(frame);
                self.render_theme_load_overlay(frame);
                self.render_theme_name_input_overlay(frame);
            }
            InputMode::ThemeLoadDeleteConfirm => {
                self.render_preview(frame);
                self.render_theme_load_overlay(frame);
            }
            InputMode::UpdateRestartConfirm => {
                self.render_preview(frame);
                self.render_update_restart_overlay(frame);
            }
            InputMode::Help => {
                self.render_preview(frame);
                self.render_help_overlay(frame);
            }
        }
    }

    // ── Mock Hyprland desktop preview ────────────────────────────────────────

    fn render_preview(&self, frame: &mut Frame) {
        let p = &self.theme.palette;
        let area = frame.area();
        let [tabs, selector, waybar, body, status] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // Desktop backdrop
        frame.render_widget(Block::default().style(Style::new().bg(tui_rgb(p.background))), body);

        self.render_group_tabs(frame, tabs);
        self.render_field_selector(frame, selector);
        self.render_waybar(frame, waybar);

        // Optional "Desktop" banner when a desktop-group color is selected.
        let body_inner = if self.sel(Region::Desktop) {
            let [banner, rest] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(body);
            let line = Line::from(vec![
                Span::styled(" Desktop ", Style::new().fg(tui_rgb(p.background)).bg(tui_rgb(p.accent)).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("  background / foreground / accent · editing {} ", self.selected.label()),
                    Style::new().fg(tui_rgb(p.foreground)).bg(tui_rgb(p.background)),
                ),
            ]);
            frame.render_widget(Paragraph::new(line).style(Style::new().bg(tui_rgb(p.background))), banner);
            rest
        } else {
            body
        };

        let [top, bottom] =
            Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(body_inner);
        let [active, inactive] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(top);
        let [launcher, terminal, notification] = Layout::horizontal([
            Constraint::Percentage(36),
            Constraint::Percentage(34),
            Constraint::Percentage(30),
        ])
        .areas(bottom);

        self.render_window(frame, active, true);
        self.render_window(frame, inactive, false);
        self.render_launcher(frame, launcher);
        self.render_terminal(frame, terminal);
        self.render_notification(frame, notification);

        self.render_status_bar(frame, status);
    }

    fn region_border(&self, region: Region, color: RgbColor) -> (Style, BorderType) {
        let selected = self.sel(region);
        let style = Style::new()
            .fg(tui_rgb(color))
            .add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() });
        let bt = if selected { BorderType::Thick } else { BorderType::Rounded };
        (style, bt)
    }

    fn title(&self, region: Region, text: &str) -> String {
        if self.sel(region) {
            format!(" ▶ {} ◀ ", text)
        } else {
            format!(" {} ", text)
        }
    }

    fn render_waybar(&self, frame: &mut Frame, area: Rect) {
        let p = &self.theme.palette;
        let bg = tui_rgb(p.waybar_background);
        let fg = tui_rgb(p.waybar_foreground);
        let active = tui_rgb(p.waybar_active_workspace);
        let highlight = self.sel(Region::Waybar);

        let mut spans: Vec<Span> = Vec::new();
        if highlight {
            spans.push(Span::styled(" ▶ ", Style::new().fg(active).bg(bg).add_modifier(Modifier::BOLD)));
        } else {
            spans.push(Span::styled(" ", Style::new().bg(bg)));
        }
        // Workspaces — first is active
        spans.push(Span::styled(
            " 1 ",
            Style::new().fg(tui_rgb(p.waybar_background)).bg(active).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(" 2 ", Style::new().fg(fg).bg(bg)));
        spans.push(Span::styled(" 3 ", Style::new().fg(fg).bg(bg)));

        let right = "  12:34   80%   ";
        let used: usize = spans.iter().map(|s| s.width()).sum::<usize>() + right.chars().count();
        let fill = (area.width as usize).saturating_sub(used);
        spans.push(Span::styled(" ".repeat(fill), Style::new().bg(bg)));
        spans.push(Span::styled(right, Style::new().fg(fg).bg(bg)));

        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::new().bg(bg)),
            area,
        );
    }

    fn render_window(&self, frame: &mut Frame, area: Rect, active: bool) {
        let p = &self.theme.palette;
        let (region, border_color, label) = if active {
            (Region::ActiveWindow, p.active_border, "Active window — Hyprland")
        } else {
            (Region::InactiveWindow, p.inactive_border, "Inactive window")
        };
        let (border_style, bt) = self.region_border(region, border_color);
        let body_fg = if active { p.foreground } else { p.foreground.shade(-40) };

        let content = vec![
            Line::from(Span::styled(
                if active { "  ~/projects/omarchy-theme-studio" } else { "  ~/notes" },
                Style::new().fg(tui_rgb(body_fg)).bg(tui_rgb(p.background)),
            )),
            Line::from(Span::styled(
                if active { "  cargo run --release" } else { "  it can wait…" },
                Style::new().fg(tui_rgb(body_fg)).bg(tui_rgb(p.background)),
            )),
        ];

        let block = Block::bordered()
            .border_type(bt)
            .title(self.title(region, label))
            .title_style(border_style)
            .border_style(border_style)
            .style(Style::new().bg(tui_rgb(p.background)));
        frame.render_widget(Paragraph::new(content).block(block), area);
    }

    fn render_launcher(&self, frame: &mut Frame, area: Rect) {
        let p = &self.theme.palette;
        let (border_style, bt) = self.region_border(Region::Launcher, p.accent);
        let lbg = tui_rgb(p.launcher_background);
        let lfg = tui_rgb(p.launcher_foreground);
        let sbg = tui_rgb(p.launcher_selected_background);
        let sfg = tui_rgb(p.launcher_selected_foreground);

        let block = Block::bordered()
            .border_type(bt)
            .title(self.title(Region::Launcher, "Launcher"))
            .title_style(border_style)
            .border_style(border_style)
            .style(Style::new().bg(lbg));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows = vec![
            Line::from(Span::styled(" Search…", Style::new().fg(lfg).bg(lbg).add_modifier(Modifier::DIM))),
            Line::from(Span::styled(" Firefox", Style::new().fg(sfg).bg(sbg).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(" Terminal", Style::new().fg(lfg).bg(lbg))),
            Line::from(Span::styled(" Files", Style::new().fg(lfg).bg(lbg))),
        ];
        frame.render_widget(Paragraph::new(rows).style(Style::new().bg(lbg)), inner);
    }

    fn render_terminal(&self, frame: &mut Frame, area: Rect) {
        let p = &self.theme.palette;
        let (border_style, bt) = self.region_border(Region::Terminal, p.accent2);
        let tbg = tui_rgb(p.terminal_background);
        let tfg = tui_rgb(p.terminal_foreground);

        let block = Block::bordered()
            .border_type(bt)
            .title(self.title(Region::Terminal, "Terminal"))
            .title_style(border_style)
            .border_style(border_style)
            .style(Style::new().bg(tbg));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::from(vec![
                Span::styled(" $ ", Style::new().fg(tui_rgb(p.accent)).bg(tbg)),
                Span::styled("omarchy-theme-set mytheme", Style::new().fg(tfg).bg(tbg)),
            ]),
            Line::from(Span::styled("  ✓ applied", Style::new().fg(tfg).bg(tbg))),
            Line::from(vec![
                Span::styled(" $ ", Style::new().fg(tui_rgb(p.accent)).bg(tbg)),
                Span::styled("█", Style::new().fg(tfg).bg(tbg)),
            ]),
        ];
        frame.render_widget(Paragraph::new(lines).style(Style::new().bg(tbg)), inner);
    }

    fn render_notification(&self, frame: &mut Frame, area: Rect) {
        let p = &self.theme.palette;
        let (border_style, bt) = self.region_border(Region::Notification, p.notification_border);
        let nbg = tui_rgb(p.notification_background);
        let nfg = tui_rgb(p.foreground);

        let block = Block::bordered()
            .border_type(bt)
            .title(self.title(Region::Notification, "Notification"))
            .title_style(border_style)
            .border_style(border_style)
            .style(Style::new().bg(nbg));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::from(Span::styled(" 🔔 Theme exported", Style::new().fg(nfg).bg(nbg).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(" ~/.config/omarchy", Style::new().fg(nfg).bg(nbg))),
        ];
        frame.render_widget(Paragraph::new(lines).style(Style::new().bg(nbg)), inner);
    }

    /// Top row: the six group pills with the active one highlighted.
    fn render_group_tabs(&self, frame: &mut Frame, area: Rect) {
        let active = self.selected.group();
        let mut spans: Vec<Span> = Vec::new();
        for (i, g) in PaletteGroup::all().iter().enumerate() {
            let is_active = *g == active;
            let (fg, bg, m) = if is_active {
                (CHROME_ACCENT_FG, CHROME_ACCENT_BG, Modifier::BOLD)
            } else {
                (CHROME_MUTED, CHROME_BG, Modifier::empty())
            };
            spans.push(Span::styled(
                format!(" {} {} ", i + 1, g.label()),
                Style::new().fg(fg).bg(bg).add_modifier(m),
            ));
            spans.push(Span::styled(" ", Style::new().bg(CHROME_BG)));
        }
        spans.push(Span::styled(
            "  1-6 / ←→ Tab group · ↑↓ field · / find",
            Style::new().fg(CHROME_MUTED).bg(CHROME_BG),
        ));
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::new().bg(CHROME_BG)),
            area,
        );
    }

    /// Prominent row under the group tabs: the active group's fields as swatch
    /// chips, the selected one boxed and highlighted. This is the main,
    /// front-and-center indicator of what you're editing.
    fn render_field_selector(&self, frame: &mut Frame, area: Rect) {
        const SEL_BG: Color = Color::Rgb(44, 41, 60);
        let group = self.selected.group();
        let palette = &self.theme.palette;

        let mut chips: Vec<Span> = Vec::new();
        for f in group.fields() {
            let selected = *f == self.selected;
            let swatch = tui_rgb(f.get(palette));
            let label = field_leaf_label(*f);
            if selected {
                chips.push(Span::styled("▏", Style::new().fg(CHROME_ACCENT_FG).bg(SEL_BG)));
                chips.push(Span::styled("  ", Style::new().bg(swatch)));
                chips.push(Span::styled(
                    format!(" {} ", label),
                    Style::new().fg(CHROME_ACCENT_FG).bg(SEL_BG).add_modifier(Modifier::BOLD),
                ));
                chips.push(Span::styled("▕", Style::new().fg(CHROME_ACCENT_FG).bg(SEL_BG)));
            } else {
                chips.push(Span::styled(" ", Style::new().bg(CHROME_BG)));
                chips.push(Span::styled("  ", Style::new().bg(swatch)));
                chips.push(Span::styled(
                    format!(" {} ", label),
                    Style::new().fg(CHROME_MUTED).bg(CHROME_BG),
                ));
                chips.push(Span::styled(" ", Style::new().bg(CHROME_BG)));
            }
            chips.push(Span::styled("  ", Style::new().bg(CHROME_BG)));
        }

        // Center the chips in the row.
        let used: usize = chips.iter().map(|s| s.width()).sum();
        let pad = (area.width as usize).saturating_sub(used) / 2;
        let mut line = vec![Span::styled(" ".repeat(pad), Style::new().bg(CHROME_BG))];
        line.extend(chips);
        frame.render_widget(
            Paragraph::new(Line::from(line)).style(Style::new().bg(CHROME_BG)),
            area,
        );
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let color = self.current_color();
        let hints = " c color · y/p yank · u undo · s save · l load · a apply · / find · ? help · q quit";
        let info = format!(
            " {}{} · {} · {} ",
            self.theme.name,
            if self.dirty { "*" } else { "" },
            self.selected.label(),
            color.to_hex(),
        );

        let mut spans: Vec<Span> =
            vec![Span::styled(hints, Style::new().fg(CHROME_MUTED).bg(CHROME_BG))];

        let right_w = info.chars().count() + 2;
        let used: usize = spans.iter().map(|s| s.width()).sum();
        let fill = (area.width as usize).saturating_sub(used + right_w);
        spans.push(Span::styled(" ".repeat(fill), Style::new().bg(CHROME_BG)));
        spans.push(Span::styled("  ", Style::new().bg(tui_rgb(color))));
        spans.push(Span::styled(
            info,
            Style::new().fg(CHROME_FG).bg(CHROME_BG).add_modifier(Modifier::BOLD),
        ));

        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::new().bg(CHROME_BG)),
            area,
        );

        // Transient message / update status overlays the hints area when present.
        if let Some(ref msg) = self.message {
            let msg_color = if msg.starts_with('✗') { Color::Red } else { Color::Green };
            let line = Line::from(Span::styled(
                format!(" {} ", msg),
                Style::new().fg(msg_color).bg(CHROME_BG).add_modifier(Modifier::BOLD),
            ));
            let row = Rect { x: area.x, y: area.y, width: area.width.min((msg.chars().count() + 2) as u16), height: 1 };
            frame.render_widget(Paragraph::new(line).style(Style::new().bg(CHROME_BG)), row);
        }
    }

    // ── Apply confirmation overlay ───────────────────────────────────────────

    fn render_apply_confirm_overlay(&self, frame: &mut Frame) {
        let area = centered_rect(frame.area(), 60, 9);
        frame.render_widget(Clear, area);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Apply theme ")
            .title_style(Style::new().fg(CHROME_ACCENT_FG).add_modifier(Modifier::BOLD))
            .border_style(Style::new().fg(Color::Rgb(90, 85, 115)))
            .style(Style::new().bg(CHROME_BG));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let name = normalize_theme_name(&self.theme.name);
        let lines = vec![
            Line::from(Span::styled(
                format!(" Run omarchy-theme-set \"{}\"?", name),
                Style::new().fg(CHROME_FG).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " This exports the theme and switches your live desktop.",
                Style::new().fg(CHROME_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" y ", Style::new().fg(CHROME_ACCENT_FG).bg(CHROME_ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" apply now    ", Style::new().fg(CHROME_MUTED)),
                Span::styled(" n ", Style::new().fg(CHROME_ACCENT_FG).bg(CHROME_ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" cancel", Style::new().fg(CHROME_MUTED)),
            ]),
        ];
        frame.render_widget(Paragraph::new(lines).style(Style::new().bg(CHROME_BG)), inner);
    }

    // ── Fuzzy field palette ( / ) ────────────────────────────────────────────

    fn render_field_search_overlay(&self, frame: &mut Frame) {
        let matches = self.filtered_fields();
        let visible = (matches.len().clamp(1, 12) + 3) as u16;
        let area = centered_rect(frame.area(), 54, visible);
        frame.render_widget(Clear, area);

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Find field ")
            .title_style(Style::new().fg(CHROME_ACCENT_FG).add_modifier(Modifier::BOLD))
            .border_style(Style::new().fg(Color::Rgb(90, 85, 115)))
            .style(Style::new().bg(CHROME_BG));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [query_row, list_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" › ", Style::new().fg(CHROME_ACCENT_FG).bg(CHROME_BG)),
                Span::styled(
                    format!("{}_", self.field_search_query),
                    Style::new().fg(CHROME_FG).bg(CHROME_BG).add_modifier(Modifier::BOLD),
                ),
            ]))
            .style(Style::new().bg(CHROME_BG)),
            query_row,
        );

        if matches.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled("   no match", Style::new().fg(CHROME_MUTED)))
                    .style(Style::new().bg(CHROME_BG)),
                list_area,
            );
            return;
        }

        let rows = list_area.height as usize;
        let sel = self.field_search_index.min(matches.len() - 1);
        let scroll = if sel >= rows { sel + 1 - rows } else { 0 };
        for (row, f) in matches.iter().enumerate().skip(scroll).take(rows) {
            let y = list_area.y + (row - scroll) as u16;
            let selected = row == sel;
            let bg = if selected { Color::Rgb(40, 38, 54) } else { CHROME_BG };
            let line = Line::from(vec![
                Span::styled(if selected { " ▸ " } else { "   " }, Style::new().fg(CHROME_ACCENT_FG).bg(bg)),
                Span::styled("● ", Style::new().fg(tui_rgb(f.get(&self.theme.palette))).bg(bg)),
                Span::styled(format!("{:<13}", f.group().label()), Style::new().fg(CHROME_MUTED).bg(bg)),
                Span::styled(
                    f.label().to_string(),
                    Style::new()
                        .fg(if selected { CHROME_FG } else { Color::Rgb(192, 192, 214) })
                        .bg(bg)
                        .add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() }),
                ),
            ]);
            frame.render_widget(
                Paragraph::new(line).style(Style::new().bg(bg)),
                Rect { x: list_area.x, y, width: list_area.width, height: 1 },
            );
        }
    }

    // ── Color picker overlay (reused, retargeted to PaletteField) ────────────

    fn render_color_picker_overlay(&self, frame: &mut Frame) {
        const OB_BG: Color = Color::Rgb(22, 22, 26);
        const OB_BORDER: Color = Color::Rgb(90, 85, 115);
        const OB_TEXT: Color = Color::Rgb(212, 212, 230);
        const OB_MUTED: Color = Color::Rgb(120, 120, 145);
        const OB_DIM: Color = Color::Rgb(84, 84, 104);
        const ACCENT_BG: Color = Color::Rgb(97, 88, 150);
        const ACCENT_FG: Color = Color::Rgb(242, 240, 255);
        const SUBTLE_BG: Color = Color::Rgb(54, 50, 74);
        const SUBTLE_FG: Color = Color::Rgb(214, 210, 235);
        const SURFACE_BG: Color = Color::Rgb(26, 26, 32);
        const SURFACE_FOCUS_BG: Color = Color::Rgb(34, 31, 46);

        let rects = picker_layout(frame.area(), self.color_editor.mode);
        frame.render_widget(Clear, rects.overlay);

        let outer = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(OB_BORDER))
            .style(Style::new().bg(OB_BG));
        let inner = outer.inner(rects.overlay);
        frame.render_widget(outer, rects.overlay);

        let [header, body, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(16),
            Constraint::Length(2),
        ])
        .areas(inner);
        let [main_col, side_col] =
            Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)]).areas(body);
        let [preview_area, _fields_area] =
            Layout::vertical([Constraint::Length(5), Constraint::Fill(1)]).areas(side_col);

        let current_rgb = self.color_editor.to_rgb();
        let current_hex = self.color_editor.hex();
        let hsl = self.color_editor.hsl;
        let hsv = self.color_editor.hsv();

        let original_rgb = self.original_color;

        let mk_pill = |key: &str, label: &str, active: bool| -> Vec<Span<'static>> {
            let (key_bg, key_fg, lbl_bg, lbl_fg) = if active {
                (ACCENT_BG, ACCENT_FG, SUBTLE_BG, SUBTLE_FG)
            } else {
                (SUBTLE_BG, SUBTLE_FG, OB_BG, OB_MUTED)
            };
            vec![
                Span::styled("", Style::new().fg(key_bg).bg(OB_BG)),
                Span::styled(
                    format!(" {} ", key),
                    Style::new().fg(key_fg).bg(key_bg).add_modifier(Modifier::BOLD),
                ),
                Span::styled("", Style::new().fg(lbl_bg).bg(key_bg)),
                Span::styled(format!(" {} ", label), Style::new().fg(lbl_fg).bg(lbl_bg)),
                Span::styled("", Style::new().fg(lbl_bg).bg(OB_BG)),
            ]
        };

        let mut header_spans = vec![Span::styled(
            " Color Picker ",
            Style::new().fg(OB_TEXT).add_modifier(Modifier::BOLD),
        )];
        let mode_focused = self.color_editor.focus == ColorPickerFocus::ModeToggle;
        let mode_pills = [
            if mode_focused {
                vec![Span::styled("› ", Style::new().fg(ACCENT_FG).add_modifier(Modifier::BOLD))]
            } else {
                vec![]
            },
            mk_pill("M", "rgb", self.color_editor.mode == ColorPickerMode::RgbSliders),
            vec![Span::raw(" ")],
            mk_pill("M", "hsl", self.color_editor.mode == ColorPickerMode::HslField),
            if mode_focused {
                vec![Span::styled(" ‹", Style::new().fg(ACCENT_FG).add_modifier(Modifier::BOLD))]
            } else {
                vec![]
            },
        ]
        .concat();
        let right_pill = {
            let name = self.selected.label();
            vec![
                Span::styled("", Style::new().fg(ACCENT_BG).bg(OB_BG)),
                Span::styled(
                    format!(" {} ", name),
                    Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD),
                ),
                Span::styled("", Style::new().fg(ACCENT_BG).bg(OB_BG)),
            ]
        };
        let left_w: usize = header_spans.iter().map(|s| s.width()).sum::<usize>()
            + mode_pills.iter().map(|s| s.width()).sum::<usize>()
            + 1;
        let right_w: usize = right_pill.iter().map(|s| s.width()).sum();
        let gap = (header.width as usize).saturating_sub(left_w + right_w);
        header_spans.push(Span::raw(" "));
        header_spans.extend(mode_pills);
        header_spans.push(Span::styled(" ".repeat(gap.max(1)), Style::new().bg(OB_BG)));
        header_spans.extend(right_pill);
        frame.render_widget(
            Paragraph::new(Line::from(header_spans)).style(Style::new().bg(OB_BG)),
            header,
        );

        frame.render_widget(Block::default().style(Style::new().bg(SURFACE_BG)), main_col);
        frame.render_widget(Block::default().style(Style::new().bg(OB_BG)), side_col);

        match self.color_editor.mode {
            ColorPickerMode::RgbSliders => {
                let channels_focus = matches!(self.color_editor.focus, ColorPickerFocus::RgbSlider(_));
                let channels_block = Block::bordered()
                    .title(" Channels ")
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(if channels_focus { ACCENT_BG } else { OB_BORDER }))
                    .style(Style::new().bg(SURFACE_BG));
                let channels_inner = channels_block.inner(rects.main_view);
                frame.render_widget(channels_block, rects.main_view);
                let slider_width = channels_inner.width.saturating_sub(8) as usize;
                let labels = ["R", "G", "B"];
                for (idx, label) in labels.into_iter().enumerate() {
                    let row_rect = Rect {
                        x: channels_inner.x,
                        y: channels_inner.y + (idx as u16 * 2),
                        width: channels_inner.width,
                        height: 1,
                    };
                    let value = self.color_editor.rgb[idx];
                    let filled = ((value as f32 / 255.0) * slider_width as f32).round() as usize;
                    let bar: String = (0..slider_width)
                        .map(|i| if i < filled { '█' } else { '░' })
                        .collect();
                    let is_focus = self.color_editor.focus == ColorPickerFocus::RgbSlider(idx);
                    let color = match idx {
                        0 => Color::Rgb(255, 96, 96),
                        1 => Color::Rgb(106, 220, 124),
                        _ => Color::Rgb(102, 186, 255),
                    };
                    let line = Line::from(vec![
                        Span::styled(
                            format!(" {} ", label),
                            Style::new()
                                .fg(if is_focus { ACCENT_FG } else { OB_TEXT })
                                .bg(if is_focus { ACCENT_BG } else { SURFACE_BG })
                                .add_modifier(if is_focus { Modifier::BOLD } else { Modifier::empty() }),
                        ),
                        Span::styled(bar, Style::new().fg(color).bg(SURFACE_BG)),
                        Span::styled(
                            format!(" {:>3}", value),
                            Style::new().fg(if is_focus { OB_TEXT } else { OB_MUTED }),
                        ),
                    ]);
                    frame.render_widget(Paragraph::new(line).style(Style::new().bg(SURFACE_BG)), row_rect);
                }
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            " RGB sliders for exact channel edits",
                            Style::new().fg(OB_TEXT).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::styled(
                            " Press M to switch to the HSL field picker.",
                            Style::new().fg(OB_MUTED),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!(" Current  {}", current_hex),
                            Style::new().fg(OB_TEXT),
                        )),
                        Line::from(Span::styled(
                            format!(" HSV {:.0} / {:.0}% / {:.0}%", hsv.hue, hsv.saturation, hsv.value),
                            Style::new().fg(OB_MUTED),
                        )),
                    ])
                    .style(Style::new().bg(SURFACE_BG)),
                    Rect {
                        x: channels_inner.x,
                        y: channels_inner.y + 6,
                        width: channels_inner.width,
                        height: channels_inner.height.saturating_sub(6),
                    },
                );
            }
            ColorPickerMode::HslField => {
                let field_focus = self.color_editor.focus == ColorPickerFocus::HslField;
                let field_block = Block::bordered()
                    .title(if field_focus { " Color Field ● " } else { " Color Field " })
                    .title_style(
                        Style::new()
                            .fg(if field_focus { ACCENT_FG } else { OB_MUTED })
                            .add_modifier(if field_focus { Modifier::BOLD } else { Modifier::empty() }),
                    )
                    .border_type(if field_focus { BorderType::Double } else { BorderType::Rounded })
                    .border_style(Style::new().fg(if field_focus { ACCENT_BG } else { OB_BORDER }))
                    .style(Style::new().bg(if field_focus { SURFACE_FOCUS_BG } else { SURFACE_BG }));
                let field_area = field_block.inner(rects.main_view);
                frame.render_widget(field_block, rects.main_view);
                for row in 0..field_area.height {
                    let mut spans = Vec::with_capacity(field_area.width as usize);
                    for col in 0..field_area.width {
                        let x_frac = col as f32 / field_area.width.saturating_sub(1).max(1) as f32;
                        let top_frac = (row as f32 * 2.0) / (field_area.height.max(1) as f32 * 2.0 - 1.0);
                        let bottom_frac =
                            ((row as f32 * 2.0) + 1.0) / (field_area.height.max(1) as f32 * 2.0 - 1.0);
                        let top = hsv_field_cell(x_frac * 360.0, (1.0 - top_frac) * 100.0, hsv.value);
                        let bottom =
                            hsv_field_cell(x_frac * 360.0, (1.0 - bottom_frac) * 100.0, hsv.value);
                        let selected_col = ((hsv.hue / 360.0)
                            * field_area.width.saturating_sub(1).max(1) as f32)
                            .round() as u16;
                        let selected_row = (((100.0 - hsv.saturation) / 100.0)
                            * field_area.height.saturating_sub(1).max(1) as f32)
                            .round() as u16;
                        if col == selected_col && row == selected_row {
                            let marker = contrast_text(current_rgb);
                            spans.push(Span::styled(
                                "◉",
                                Style::new().fg(tui_rgb(marker)).bg(tui_rgb(current_rgb)).add_modifier(Modifier::BOLD),
                            ));
                        } else {
                            spans.push(Span::styled(
                                "▀",
                                Style::new().fg(tui_rgb(top)).bg(tui_rgb(bottom)),
                            ));
                        }
                    }
                    frame.render_widget(
                        Paragraph::new(Line::from(spans))
                            .style(Style::new().bg(if field_focus { SURFACE_FOCUS_BG } else { SURFACE_BG })),
                        Rect {
                            x: field_area.x,
                            y: field_area.y + row,
                            width: field_area.width,
                            height: 1,
                        },
                    );
                }
                let value_focus = self.color_editor.focus == ColorPickerFocus::LightnessSlider;
                let value_block = Block::bordered()
                    .title(if value_focus { " V ● " } else { " V " })
                    .title_style(
                        Style::new()
                            .fg(if value_focus { ACCENT_FG } else { OB_MUTED })
                            .add_modifier(if value_focus { Modifier::BOLD } else { Modifier::empty() }),
                    )
                    .border_type(if value_focus { BorderType::Double } else { BorderType::Rounded })
                    .border_style(Style::new().fg(if value_focus { ACCENT_BG } else { OB_BORDER }))
                    .style(Style::new().bg(if value_focus { SURFACE_FOCUS_BG } else { SURFACE_BG }));
                let value_area = value_block.inner(rects.aux_slider);
                frame.render_widget(value_block, rects.aux_slider);
                let selected_row = (((100.0 - hsv.value) / 100.0)
                    * value_area.height.saturating_sub(1).max(1) as f32)
                    .round() as u16;
                for row in 0..value_area.height {
                    let top_frac = (row as f32 * 2.0) / (value_area.height.max(1) as f32 * 2.0 - 1.0);
                    let bottom_frac =
                        ((row as f32 * 2.0) + 1.0) / (value_area.height.max(1) as f32 * 2.0 - 1.0);
                    let top_value = (1.0 - top_frac.clamp(0.0, 1.0)) * 100.0;
                    let bottom_value = (1.0 - bottom_frac.clamp(0.0, 1.0)) * 100.0;
                    let top_color = hsv_field_cell(hsv.hue, hsv.saturation, top_value);
                    let bottom_color = hsv_field_cell(hsv.hue, hsv.saturation, bottom_value);
                    let selected = row == selected_row;
                    let indicator_color = bottom_color;
                    let indicator_fg = contrast_text(indicator_color);
                    let content = if selected {
                        "█".repeat(value_area.width as usize)
                    } else {
                        "▀".repeat(value_area.width as usize)
                    };
                    let style = if selected {
                        Style::new()
                            .fg(tui_rgb(indicator_fg))
                            .bg(tui_rgb(indicator_color))
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::new().fg(tui_rgb(top_color)).bg(tui_rgb(bottom_color))
                    };
                    frame.render_widget(
                        Paragraph::new(Line::from(vec![Span::styled(content, style)])),
                        Rect {
                            x: value_area.x,
                            y: value_area.y + row,
                            width: value_area.width,
                            height: 1,
                        },
                    );
                }
            }
        }

        let preview_lines = {
            let current_fg = tui_rgb(contrast_text(current_rgb));
            let before_line = if let Some(orig) = original_rgb {
                Line::from(vec![
                    Span::styled("      ", Style::new().bg(tui_rgb(orig))),
                    Span::styled("  →  ", Style::new().fg(OB_DIM).bg(OB_BG)),
                    Span::styled("      ", Style::new().bg(tui_rgb(current_rgb))),
                ])
            } else {
                Line::from(vec![Span::styled("      ", Style::new().bg(tui_rgb(current_rgb)))])
            };
            vec![
                Line::from(Span::styled(
                    format!(" {}", current_hex),
                    Style::new().fg(OB_TEXT).add_modifier(Modifier::BOLD),
                )),
                before_line,
                Line::from(Span::styled(
                    format!(" rgb {} {} {}", current_rgb.r, current_rgb.g, current_rgb.b),
                    Style::new().fg(OB_MUTED),
                )),
                Line::from(Span::styled(
                    format!(" hsl {:.0} {:.0}% {:.0}%", hsl.hue, hsl.saturation, hsl.lightness),
                    Style::new().fg(OB_MUTED),
                )),
                Line::from(Span::styled(
                    format!(" hsv {:.0} {:.0}% {:.0}%", hsv.hue, hsv.saturation, hsv.value),
                    Style::new().fg(current_fg),
                )),
                Line::from(Span::styled(
                    format!(" focus {}", self.color_editor.focus_label()),
                    Style::new().fg(ACCENT_FG),
                )),
            ]
        };
        frame.render_widget(
            Paragraph::new(preview_lines).style(Style::new().bg(OB_BG)),
            preview_area,
        );

        let mut field_block = |rect: Rect, title: &str, focused: bool| {
            let border = if focused { ACCENT_BG } else { OB_BORDER };
            frame.render_widget(
                Block::bordered()
                    .title(format!(" {} ", title))
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().fg(border))
                    .style(Style::new().bg(OB_BG)),
                rect,
            );
        };
        field_block(rects.hex_field, "HEX", self.color_editor.focus == ColorPickerFocus::HexField);
        for (idx, rect) in rects.rgb_fields.iter().enumerate() {
            field_block(*rect, ["R", "G", "B"][idx], self.color_editor.focus == ColorPickerFocus::RgbField(idx));
        }
        for (idx, rect) in rects.hsl_fields.iter().enumerate() {
            field_block(*rect, ["H", "S", "L"][idx], self.color_editor.focus == ColorPickerFocus::HslFieldValue(idx));
        }
        let render_field_value = |frame: &mut Frame, rect: Rect, value: String, suffix: &str, editing: bool| {
            let inner = Rect {
                x: rect.x + 1,
                y: rect.y + 1,
                width: rect.width.saturating_sub(2),
                height: rect.height.saturating_sub(2),
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        value,
                        Style::new().fg(if editing { ACCENT_FG } else { OB_TEXT }).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(suffix.to_string(), Style::new().fg(OB_MUTED)),
                ]))
                .style(Style::new().bg(OB_BG)),
                inner,
            );
        };
        render_field_value(
            frame,
            rects.hex_field,
            self.color_editor.field_value(EditableField::Hex),
            "",
            matches!(
                self.color_editor.text_edit.as_ref().map(|edit| edit.target),
                Some(EditableField::Hex)
            ),
        );
        for idx in 0..3 {
            render_field_value(
                frame,
                rects.rgb_fields[idx],
                self.color_editor.field_value(EditableField::Rgb(idx)),
                "",
                matches!(
                    self.color_editor.text_edit.as_ref().map(|edit| edit.target),
                    Some(EditableField::Rgb(i)) if i == idx
                ),
            );
        }
        for idx in 0..3 {
            render_field_value(
                frame,
                rects.hsl_fields[idx],
                self.color_editor.field_value(EditableField::Hsl(idx)),
                if idx == 0 { "°" } else { "%" },
                matches!(
                    self.color_editor.text_edit.as_ref().map(|edit| edit.target),
                    Some(EditableField::Hsl(i)) if i == idx
                ),
            );
        }

        let footer_lines = vec![
            Line::from(vec![
                Span::styled(" Tab ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" focus  ", Style::new().fg(OB_MUTED)),
                Span::styled(" M ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" switch  ", Style::new().fg(OB_MUTED)),
                Span::styled(" Enter ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" edit/keep", Style::new().fg(OB_MUTED)),
            ]),
            Line::from(vec![
                Span::styled(" Mouse ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" drag  ", Style::new().fg(OB_MUTED)),
                Span::styled(" # ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" hex  ", Style::new().fg(OB_MUTED)),
                Span::styled(" Esc ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" cancel", Style::new().fg(OB_MUTED)),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(footer_lines).style(Style::new().bg(OB_BG)),
            footer,
        );
    }

    fn render_theme_name_input_overlay(&self, frame: &mut Frame) {
        let area = centered_rect(frame.area(), 64, 8);
        frame.render_widget(Clear, area);

        let preview_name = normalize_theme_name(&self.theme_name_input);
        let normalized = if preview_name.is_empty() {
            String::from("<invalid>")
        } else {
            format!("~/.config/omarchy/themes/{}/", preview_name)
        };

        let lines = vec![
            Line::from(" Export theme as "),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Name: ", Style::new().fg(Color::Rgb(167, 139, 250))),
                Span::styled(
                    format!("{}_", self.theme_name_input),
                    Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Dir:  ", Style::new().fg(Color::DarkGray)),
                Span::styled(normalized, Style::new().fg(Color::Gray)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " Letters, numbers, spaces, - and _ are allowed",
                Style::new().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                " Enter: export  Esc: cancel  Backspace: delete",
                Style::new().fg(Color::DarkGray),
            )),
        ];

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Theme Name ")
            .title_style(Style::new().fg(Color::Rgb(167, 139, 250)).add_modifier(Modifier::BOLD))
            .border_style(Style::new().fg(Color::Rgb(90, 85, 115)))
            .style(Style::new().bg(Color::Rgb(22, 22, 26)));

        frame.render_widget(
            Paragraph::new(lines).block(block).style(Style::new().bg(Color::Rgb(22, 22, 26))),
            area,
        );
    }

    fn render_help_overlay(&self, frame: &mut Frame) {
        const PANEL_BG: Color = Color::Rgb(22, 22, 26);
        const PANEL_BORDER: Color = Color::Rgb(90, 85, 115);
        const PANEL_MUTED: Color = Color::Rgb(120, 120, 145);
        const PANEL_TEXT: Color = Color::Rgb(212, 212, 230);
        const ACCENT_BG: Color = Color::Rgb(97, 88, 150);
        const ACCENT_FG: Color = Color::Rgb(242, 240, 255);
        const SUBTLE_BG: Color = Color::Rgb(54, 50, 74);
        const SUBTLE_FG: Color = Color::Rgb(214, 210, 235);
        const DIVIDER: Color = Color::Rgb(50, 48, 64);

        enum HelpRow<'a> {
            Section(&'a str),
            Entry(&'a str, &'a str),
            Spacer,
        }

        let rows: &[HelpRow<'_>] = &[
            HelpRow::Section("Editor"),
            HelpRow::Entry("1–6", "Jump to a palette group"),
            HelpRow::Entry("← → / Tab", "Previous / next group"),
            HelpRow::Entry("↑/k ↓/j", "Field within the current group"),
            HelpRow::Entry("/", "Find a field (fuzzy search)"),
            HelpRow::Entry("c / Enter", "Open color picker for selected field"),
            HelpRow::Entry("y", "Yank (copy) current color"),
            HelpRow::Entry("p", "Paste yanked color"),
            HelpRow::Entry("u", "Undo last color change"),
            HelpRow::Entry("s", "Export theme to ~/.config/omarchy/themes"),
            HelpRow::Entry("l", "Open theme loader"),
            HelpRow::Entry("a", "Apply via omarchy-theme-set (confirmed)"),
            HelpRow::Entry("U", "Install latest release when available"),
            HelpRow::Entry("?", "Toggle this help screen"),
            HelpRow::Entry("q / Esc", "Quit"),
            HelpRow::Spacer,
            HelpRow::Section("Color Picker"),
            HelpRow::Entry("Tab / Shift+Tab", "Move focus between controls"),
            HelpRow::Entry("m", "Switch RGB sliders / HSL field"),
            HelpRow::Entry("Mouse drag", "Drag in HSL field or value slider"),
            HelpRow::Entry("← → ↑ ↓", "Nudge the focused control"),
            HelpRow::Entry("Shift / Alt", "Coarse / fine nudging"),
            HelpRow::Entry("Enter", "Edit the focused value field or keep"),
            HelpRow::Entry("#", "Jump to hex field editing"),
            HelpRow::Entry("Esc", "Cancel"),
            HelpRow::Spacer,
            HelpRow::Section("Theme Loader"),
            HelpRow::Entry("type", "Search and filter themes"),
            HelpRow::Entry("Enter / ↓", "Commit search and move into results"),
            HelpRow::Entry("↑ ↓", "Navigate themes"),
            HelpRow::Entry("Enter", "Load selected theme into editor"),
            HelpRow::Entry("d", "Filter built-in themes"),
            HelpRow::Entry("s", "Filter saved themes"),
            HelpRow::Entry("r", "Rename selected saved theme"),
            HelpRow::Entry("x", "Delete selected saved theme"),
            HelpRow::Entry("Esc", "Clear search or cancel"),
        ];

        let overlay_h = 24u16.min(frame.area().height.saturating_sub(2));
        let overlay_w = 84u16.min(frame.area().width.saturating_sub(4));
        let area = centered_rect(frame.area(), overlay_w, overlay_h);
        frame.render_widget(Clear, area);

        let outer_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(PANEL_BORDER))
            .style(Style::new().bg(PANEL_BG));
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let [header_rect, body_rect, footer_rect] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .areas(inner);

        let title = vec![Span::styled(
            " omarchy-theme-studio · Help ",
            Style::new().fg(PANEL_TEXT).add_modifier(Modifier::BOLD),
        )];
        let scroll_label = format!("{:>2} rows", rows.len());
        let mut header_spans = title;
        let title_w: usize = header_spans.iter().map(|span| span.width()).sum();
        let right_spans = vec![Span::styled(scroll_label, Style::new().fg(PANEL_MUTED))];
        let right_w: usize = right_spans.iter().map(|span| span.width()).sum();
        let gap = (header_rect.width as usize).saturating_sub(title_w + right_w);
        header_spans.push(Span::styled(" ".repeat(gap), Style::new().bg(PANEL_BG)));
        header_spans.extend(right_spans);
        frame.render_widget(
            Paragraph::new(Line::from(header_spans)).style(Style::new().bg(PANEL_BG)),
            Rect { x: header_rect.x, y: header_rect.y, width: header_rect.width, height: 1 },
        );

        let header_hint = vec![
            Span::styled(" ↑↓ ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
            Span::styled(" scroll  ", Style::new().fg(PANEL_MUTED)),
            Span::styled(" PgUp/PgDn ", Style::new().fg(SUBTLE_FG).bg(SUBTLE_BG).add_modifier(Modifier::BOLD)),
            Span::styled(" jump", Style::new().fg(PANEL_MUTED)),
        ];
        frame.render_widget(
            Paragraph::new(Line::from(header_hint)).style(Style::new().bg(PANEL_BG)),
            Rect { x: header_rect.x, y: header_rect.y + 1, width: header_rect.width, height: 1 },
        );

        let [list_rect, detail_rect] =
            Layout::horizontal([Constraint::Percentage(66), Constraint::Percentage(34)]).areas(body_rect);

        frame.render_widget(
            Paragraph::new("").block(
                Block::default()
                    .borders(ratatui::widgets::Borders::RIGHT)
                    .border_style(Style::new().fg(DIVIDER)),
            ),
            list_rect,
        );

        let list_inner = Rect {
            x: list_rect.x,
            y: list_rect.y,
            width: list_rect.width.saturating_sub(1),
            height: list_rect.height,
        };
        let detail_inner = Rect {
            x: detail_rect.x + 1,
            y: detail_rect.y,
            width: detail_rect.width.saturating_sub(2),
            height: detail_rect.height,
        };

        let visible_rows = list_inner.height as usize;
        let max_scroll = rows.len().saturating_sub(visible_rows) as u16;
        let scroll = self.help_scroll.min(max_scroll);

        let list_lines: Vec<Line> = rows
            .iter()
            .map(|row| match row {
                HelpRow::Section(title) => Line::from(vec![
                    Span::styled(" ", Style::new().bg(PANEL_BG)),
                    Span::styled(
                        format!(" {} ", title),
                        Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD),
                    ),
                ]),
                HelpRow::Entry(key, desc) => Line::from(vec![
                    Span::styled(" ", Style::new().bg(PANEL_BG)),
                    Span::styled(format!("{:<18}", key), Style::new().fg(PANEL_TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled(desc.to_string(), Style::new().fg(PANEL_MUTED)),
                ]),
                HelpRow::Spacer => Line::from(""),
            })
            .collect();

        frame.render_widget(
            Paragraph::new(list_lines).scroll((scroll, 0)).style(Style::new().bg(PANEL_BG)),
            list_inner,
        );

        let detail_lines = vec![
            Line::from(Span::styled(" Exports", Style::new().fg(PANEL_MUTED).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(
                " Saving writes a real Omarchy theme dir: colors.toml, hyprland.conf, waybar.css, walker.css, ghostty.conf, README.md.",
                Style::new().fg(PANEL_TEXT),
            )),
            Line::from(""),
            Line::from(Span::styled(" Apply", Style::new().fg(PANEL_MUTED).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(
                " 'a' runs omarchy-theme-set only after you confirm. Nothing is applied automatically.",
                Style::new().fg(PANEL_TEXT),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!(" Showing from row {} of {}", scroll.saturating_add(1), rows.len()),
                Style::new().fg(if max_scroll > 0 { ACCENT_FG } else { PANEL_TEXT }),
            )),
        ];
        frame.render_widget(
            Paragraph::new(detail_lines).style(Style::new().bg(PANEL_BG)),
            detail_inner,
        );

        let footer_spans = vec![
            Span::styled(" ↑↓ ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
            Span::styled(" scroll  ", Style::new().fg(PANEL_MUTED)),
            Span::styled(" Home/End ", Style::new().fg(SUBTLE_FG).bg(SUBTLE_BG).add_modifier(Modifier::BOLD)),
            Span::styled(" jump  ", Style::new().fg(PANEL_MUTED)),
            Span::styled(" Esc ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
            Span::styled(" close", Style::new().fg(PANEL_MUTED)),
        ];
        frame.render_widget(
            Paragraph::new(Line::from(footer_spans)).style(Style::new().bg(PANEL_BG)),
            footer_rect,
        );
    }

    fn render_update_restart_overlay(&self, frame: &mut Frame) {
        const PANEL_BG: Color = Color::Rgb(22, 22, 26);
        const PANEL_BORDER: Color = Color::Rgb(90, 85, 115);
        const PANEL_MUTED: Color = Color::Rgb(120, 120, 145);
        const PANEL_TEXT: Color = Color::Rgb(212, 212, 230);
        const ACCENT_BG: Color = Color::Rgb(97, 88, 150);
        const ACCENT_FG: Color = Color::Rgb(242, 240, 255);
        const SUBTLE_BG: Color = Color::Rgb(54, 50, 74);
        const SUBTLE_FG: Color = Color::Rgb(214, 210, 235);

        let area = centered_rect(frame.area(), 58, 10);
        frame.render_widget(Clear, area);

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Update Installed ")
            .title_style(Style::new().fg(ACCENT_FG).add_modifier(Modifier::BOLD))
            .border_style(Style::new().fg(PANEL_BORDER))
            .style(Style::new().bg(PANEL_BG));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::from(Span::styled(
                " The new build is ready.",
                Style::new().fg(PANEL_TEXT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Restart now to launch the updated binary, or choose later and keep working.",
                Style::new().fg(PANEL_MUTED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Enter ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" restart now  ", Style::new().fg(PANEL_MUTED)),
                Span::styled(" L ", Style::new().fg(SUBTLE_FG).bg(SUBTLE_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" later", Style::new().fg(PANEL_MUTED)),
            ]),
        ];

        frame.render_widget(Paragraph::new(lines).style(Style::new().bg(PANEL_BG)), inner);
    }

    fn render_theme_load_overlay(&self, frame: &mut Frame) {
        let overlay_h = 20u16.min(frame.area().height.saturating_sub(2));
        let longest_name = self
            .loadable_themes
            .iter()
            .map(|entry| entry.name().chars().count())
            .max()
            .unwrap_or(24) as u16;
        let desired_w = longest_name.saturating_add(18);
        let overlay_w = desired_w.clamp(64, 78).min(frame.area().width.saturating_sub(4));
        let area = centered_rect(frame.area(), overlay_w, overlay_h);
        frame.render_widget(Clear, area);

        const PANEL_BG: Color = Color::Rgb(22, 22, 26);
        const PANEL_BORDER: Color = Color::Rgb(90, 85, 115);
        const PANEL_MUTED: Color = Color::Rgb(120, 120, 145);
        const PANEL_TEXT: Color = Color::Rgb(212, 212, 230);
        const PANEL_DIM: Color = Color::Rgb(84, 84, 104);
        const ACCENT_BG: Color = Color::Rgb(97, 88, 150);
        const ACCENT_FG: Color = Color::Rgb(242, 240, 255);
        const SUBTLE_BG: Color = Color::Rgb(54, 50, 74);
        const SUBTLE_FG: Color = Color::Rgb(214, 210, 235);

        let outer_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(PANEL_BORDER))
            .style(Style::new().bg(PANEL_BG));
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        use crate::ui::state::ThemeFilter;
        let display_query = self.theme_search_query.trim_start_matches('/');

        let title_spans = vec![Span::styled(
            " Themes ",
            Style::new().fg(PANEL_TEXT).add_modifier(Modifier::BOLD),
        )];
        let count_label = format!("{:>2} matches", self.loadable_themes.len());
        let count_spans = vec![Span::styled(count_label, Style::new().fg(PANEL_MUTED))];
        let title_w: usize = title_spans.iter().map(|s| s.width()).sum();
        let count_w: usize = count_spans.iter().map(|s| s.width()).sum();
        let search_label = format!(
            " / {}",
            if display_query.is_empty() {
                "(type to filter)".to_string()
            } else if self.search_focused {
                format!("{}_", display_query)
            } else {
                display_query.to_string()
            }
        );
        let search_fg = if display_query.is_empty() { PANEL_DIM } else { PANEL_TEXT };
        let search_spans = vec![Span::styled(search_label, Style::new().fg(search_fg))];
        let search_w: usize = search_spans.iter().map(|s| s.width()).sum();
        let gap = (inner.width as usize).saturating_sub(title_w + search_w + count_w);
        let mut header_spans = title_spans;
        header_spans.push(Span::styled(" ".repeat(gap), Style::new().bg(PANEL_BG)));
        header_spans.extend(search_spans);
        header_spans.push(Span::styled("  ", Style::new().bg(PANEL_BG)));
        header_spans.extend(count_spans);

        let header_rect = Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 };
        frame.render_widget(
            Paragraph::new(Line::from(header_spans)).style(Style::new().bg(PANEL_BG)),
            header_rect,
        );

        let active_name = self.original_theme.as_ref().map(|t| t.name.clone());
        let filter_pill = |key: &str, label: &str, active: bool| -> Vec<Span<'static>> {
            let (key_bg, key_fg, lbl_bg, lbl_fg) = if active {
                (ACCENT_BG, ACCENT_FG, SUBTLE_BG, SUBTLE_FG)
            } else {
                (SUBTLE_BG, SUBTLE_FG, PANEL_BG, PANEL_MUTED)
            };
            vec![
                Span::styled("", Style::new().fg(key_bg).bg(PANEL_BG)),
                Span::styled(
                    format!(" {} ", key),
                    Style::new().fg(key_fg).bg(key_bg).add_modifier(Modifier::BOLD),
                ),
                Span::styled("", Style::new().fg(lbl_bg).bg(key_bg)),
                Span::styled(format!(" {} ", label), Style::new().fg(lbl_fg).bg(lbl_bg)),
                Span::styled("", Style::new().fg(lbl_bg).bg(PANEL_BG)),
                Span::raw(" "),
            ]
        };

        let mut filter_spans = filter_pill("A", "all", matches!(self.theme_filter, ThemeFilter::All));
        filter_spans.extend(filter_pill("D", "built-in", matches!(self.theme_filter, ThemeFilter::Builtin)));
        filter_spans.extend(filter_pill("S", "saved", matches!(self.theme_filter, ThemeFilter::Saved)));
        let filter_rect = Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: 1 };
        frame.render_widget(
            Paragraph::new(Line::from(filter_spans)).style(Style::new().bg(PANEL_BG)),
            filter_rect,
        );

        let body_rect = Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: inner.height.saturating_sub(4),
        };
        let list_inner = body_rect;
        let cards_per_view = list_inner.height.saturating_sub(1) as usize;

        let scroll = if self.loadable_themes.len() <= cards_per_view {
            0
        } else if self.selected_theme_index < cards_per_view / 2 {
            0
        } else {
            (self.selected_theme_index.saturating_sub(cards_per_view / 2))
                .min(self.loadable_themes.len().saturating_sub(cards_per_view))
        };

        if self.loadable_themes.is_empty() {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(" No themes match ", Style::new().fg(PANEL_TEXT).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from(Span::styled(format!(" \"/{}\"", display_query), Style::new().fg(PANEL_MUTED))),
                ])
                .style(Style::new().bg(PANEL_BG)),
                list_inner,
            );
        } else {
            for i in scroll..self.loadable_themes.len() {
                let row_y = list_inner.y + (i - scroll) as u16;
                if row_y >= list_inner.y + list_inner.height {
                    break;
                }
                let row_rect = Rect { x: list_inner.x, y: row_y, width: list_inner.width, height: 1 };
                self.render_theme_card(frame, row_rect, i, i == self.selected_theme_index, &active_name);
            }
        }

        let footer_rect = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ↑↓ ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" move  ", Style::new().fg(PANEL_MUTED)),
                Span::styled(" Enter ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" load  ", Style::new().fg(PANEL_MUTED)),
                Span::styled(" type ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" filter  ", Style::new().fg(PANEL_MUTED)),
                Span::styled(" Esc ", Style::new().fg(ACCENT_FG).bg(ACCENT_BG).add_modifier(Modifier::BOLD)),
                Span::styled(" close", Style::new().fg(PANEL_MUTED)),
            ]))
            .style(Style::new().bg(PANEL_BG)),
            footer_rect,
        );
    }

    fn render_theme_card(
        &self,
        frame: &mut Frame,
        area: Rect,
        index: usize,
        selected: bool,
        active_name: &Option<String>,
    ) {
        let entry = &self.loadable_themes[index];
        let swatches = self
            .theme_swatches
            .get(entry.name())
            .copied()
            .unwrap_or([RgbColor::new(50, 50, 50); 4]);
        let is_active = active_name.as_deref() == Some(entry.name());
        let row_bg = if selected { Color::Rgb(30, 30, 38) } else { Color::Rgb(22, 22, 26) };
        let row_fg = if selected { Color::White } else { Color::Rgb(192, 192, 214) };
        let type_fg = if selected { Color::Rgb(176, 176, 202) } else { Color::Rgb(108, 108, 132) };

        let mut spans = vec![Span::styled(
            if selected { "> " } else { "  " },
            Style::new()
                .fg(if selected { Color::Rgb(200, 190, 240) } else { Color::Rgb(72, 72, 92) })
                .bg(row_bg)
                .add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() }),
        )];

        let type_label = if entry.is_builtin() { "B" } else { "S" };
        let detail_w = if selected { 12 } else { 4 };
        let reserved = 2 + detail_w;
        let name_width = (area.width as usize).saturating_sub(reserved + 1).max(12);
        let clipped_name = clip_text(entry.name(), name_width);
        spans.push(Span::styled(
            format!("{:<width$}", clipped_name, width = name_width),
            Style::new().fg(row_fg).bg(row_bg).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(" ", Style::new().bg(row_bg)));
        if selected {
            for sw in swatches.iter().take(3) {
                spans.push(Span::styled("● ", Style::new().fg(Color::Rgb(sw.r, sw.g, sw.b)).bg(row_bg)));
            }
            spans.push(Span::styled(type_label, Style::new().fg(type_fg).bg(row_bg).add_modifier(Modifier::BOLD)));
        } else if is_active {
            spans.push(Span::styled("● ", Style::new().fg(Color::Rgb(110, 220, 110)).bg(row_bg)));
            spans.push(Span::styled(type_label, Style::new().fg(type_fg).bg(row_bg)));
        } else {
            spans.push(Span::styled("  ", Style::new().bg(row_bg)));
            spans.push(Span::styled(type_label, Style::new().fg(type_fg).bg(row_bg)));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::new().bg(row_bg)),
            area,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    fn find_text(buffer: &Buffer, needle: &str) -> Option<(u16, u16)> {
        let width = buffer.area.width;
        let height = buffer.area.height;
        let chars: Vec<char> = needle.chars().collect();
        let needle_width = chars.len() as u16;
        for y in 0..height {
            for x in 0..width.saturating_sub(needle_width).saturating_add(1) {
                let matched = chars
                    .iter()
                    .enumerate()
                    .all(|(offset, ch)| buffer[(x + offset as u16, y)].symbol().starts_with(*ch));
                if matched {
                    return Some((x, y));
                }
            }
        }
        None
    }

    #[test]
    fn preview_renders_desktop_regions() {
        let app = App::default();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test terminal");
        let frame = terminal.draw(|f| app.render(f)).expect("draw should succeed");
        let buffer = &frame.buffer;
        assert!(find_text(buffer, "Active window").is_some());
        assert!(find_text(buffer, "Launcher").is_some());
        assert!(find_text(buffer, "Terminal").is_some());
        assert!(find_text(buffer, "Notification").is_some());
    }

    #[test]
    fn terminal_panel_uses_palette_background() {
        let mut app = App::default();
        app.theme.palette.terminal_background = RgbColor::new(7, 9, 11);
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test terminal");
        let frame = terminal.draw(|f| app.render(f)).expect("draw should succeed");
        let buffer = &frame.buffer;
        let pos = find_text(buffer, "applied").expect("terminal sample should render");
        assert_eq!(buffer[pos].bg, Color::Rgb(7, 9, 11));
    }

    #[test]
    fn group_tabs_show_all_six_groups() {
        let app = App::default();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test terminal");
        let frame = terminal.draw(|f| app.render(f)).expect("draw should succeed");
        let buffer = &frame.buffer;
        for label in ["Desktop", "Windows", "Waybar", "Launcher", "Terminal", "Notification"] {
            assert!(find_text(buffer, label).is_some(), "group tab '{label}' should render");
        }
    }

    #[test]
    fn field_selector_shows_active_group_fields() {
        // Desktop group is selected by default; its leaf labels appear up top.
        let app = App::default();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test terminal");
        let frame = terminal.draw(|f| app.render(f)).expect("draw should succeed");
        assert!(find_text(&frame.buffer, "Accent 2").is_some(), "selector should list group fields");
    }

    #[test]
    fn field_search_overlay_lists_matches() {
        let mut app = App::default();
        app.open_field_search();
        for c in "laun".chars() {
            app.push_field_search_char(c);
        }
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test terminal");
        let frame = terminal.draw(|f| app.render(f)).expect("draw should succeed");
        assert!(find_text(&frame.buffer, "Find field").is_some());
        assert!(find_text(&frame.buffer, "Launcher background").is_some());
    }

    #[test]
    fn every_input_mode_renders_without_panic() {
        let modes = [
            InputMode::Preview,
            InputMode::ColorPicker,
            InputMode::ThemeNameInput,
            InputMode::ApplyConfirm,
            InputMode::FieldSearch,
            InputMode::ThemeLoad,
            InputMode::Help,
        ];
        for mode in modes {
            let mut app = App::default();
            app.refresh_theme_list();
            app.input_mode = mode;
            let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("test terminal");
            terminal.draw(|f| app.render(f)).expect("draw should succeed");
        }
    }
}
