use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, Sense, Stroke, Theme, Ui,
};
use egui_extras::{Column, TableBuilder};

use pathhunter::audit::{self, EntryState, PathEntry};
use pathhunter::cleanplan::{Plan, Selection};
use pathhunter::expandvars;
use pathhunter::source::SourceMap;
use pathhunter::winenv;

use crate::theme::{Palette, tint};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Coarse entry category, as shown on the summary cards.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ok,
    Symlink,
    Duplicate,
    Dead,
    Empty,
}

impl Kind {
    pub const ALL: [Kind; 5] = [
        Kind::Ok,
        Kind::Dead,
        Kind::Symlink,
        Kind::Duplicate,
        Kind::Empty,
    ];

    pub fn of(state: &EntryState) -> Kind {
        match state {
            EntryState::Ok => Kind::Ok,
            EntryState::Symlink(_) => Kind::Symlink,
            EntryState::Duplicate(_) => Kind::Duplicate,
            EntryState::Empty => Kind::Empty,
            EntryState::Dead | EntryState::File | EntryState::Dangling => Kind::Dead,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Ok => "OK",
            Kind::Symlink => "Symlink",
            Kind::Duplicate => "Duplicate",
            Kind::Dead => "Dead",
            Kind::Empty => "Empty",
        }
    }

    fn plural(self) -> &'static str {
        match self {
            Kind::Ok => "Healthy",
            Kind::Symlink => "Symlinks",
            Kind::Duplicate => "Duplicates",
            Kind::Dead => "Dead",
            Kind::Empty => "Empty",
        }
    }
}

/// Wording mirrors the CLI's `display::fmt_detail`.
fn detail(state: &EntryState) -> String {
    match state {
        EntryState::Ok => String::new(),
        EntryState::Symlink(target) => format!("links to {target}"),
        EntryState::Duplicate(idx) => format!("duplicate #{idx}"),
        EntryState::Dead => "directory not found".to_owned(),
        EntryState::Dangling => "dangling symlink".to_owned(),
        EntryState::File => "is a file, not a dir".to_owned(),
        EntryState::Empty => "means current directory — security risk".to_owned(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PathView {
    /// This process's `PATH` — what `ph` audits.
    Process,
    /// System + User registry values, expanded — what new programs get.
    Fresh,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Issues,
    Only(Kind),
}

/// One audit pass plus the registry state needed to clean it.
pub struct Scan {
    pub entries: Vec<PathEntry>,
    pub user_segs: Vec<String>,
    pub system_segs: Vec<String>,
    pub admin: bool,
    pub plan: Plan,
}

impl Scan {
    fn load(view: PathView) -> Scan {
        let user_segs = winenv::read_raw_user_path_segments();
        let system_segs = winenv::read_raw_system_path_segments();
        let raw = match view {
            PathView::Process => std::env::var_os("PATH")
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            PathView::Fresh => system_segs
                .iter()
                .chain(&user_segs)
                .map(|s| expandvars::windows(s))
                .collect::<Vec<_>>()
                .join(";"),
        };
        let mut sm = SourceMap::default();
        sm.build();
        let entries = if raw.is_empty() {
            Vec::new()
        } else {
            audit::scan(&sm, &raw)
        };
        let admin = winenv::is_admin();
        let plan = Plan::build(&entries, &user_segs, &system_segs, admin);
        Scan {
            entries,
            user_segs,
            system_segs,
            admin,
            plan,
        }
    }

    fn count(&self, kind: Kind) -> usize {
        self.entries
            .iter()
            .filter(|e| Kind::of(&e.state) == kind)
            .count()
    }
}

pub struct Toast {
    pub text: String,
    pub ok: bool,
    pub until: f64,
}

pub struct App {
    pub scan: Scan,
    pub view: PathView,
    filter: Filter,
    search: String,
    pub sel: Selection,
    pub clean_open: bool,
    pub confirm_open: bool,
    pub toast: Option<Toast>,
}

impl App {
    pub fn new() -> App {
        let mut app = App {
            scan: Scan::load(PathView::Process),
            view: PathView::Process,
            filter: Filter::All,
            search: String::new(),
            sel: Selection::default(),
            clean_open: false,
            confirm_open: false,
            toast: None,
        };
        app.sel = app.default_selection();
        app.clean_open = cfg!(windows) && app.scan.plan.any_issue();
        app
    }

    pub fn rescan(&mut self) {
        self.scan = Scan::load(self.view);
        self.sel = self.default_selection();
    }

    pub fn notify(&mut self, ui: &Ui, text: impl Into<String>, ok: bool) {
        let now = ui.input(|i| i.time);
        self.toast = Some(Toast {
            text: text.into(),
            ok,
            until: now + 6.0,
        });
    }

    fn visible(&self) -> Vec<usize> {
        let needle = self.search.to_lowercase();
        self.scan
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let kind = Kind::of(&e.state);
                let kind_ok = match self.filter {
                    Filter::All => true,
                    Filter::Issues => kind != Kind::Ok && kind != Kind::Symlink,
                    Filter::Only(k) => kind == k,
                };
                kind_ok
                    && (needle.is_empty()
                        || e.path.to_lowercase().contains(&needle)
                        || e.source.to_lowercase().contains(&needle))
            })
            .map(|(i, _)| i)
            .collect()
    }

    // ── Top bar ─────────────────────────────────────────────────────────

    fn top_bar(&mut self, ui: &mut Ui, p: &Palette) {
        let frame = Frame::new()
            .fill(p.panel)
            .inner_margin(Margin::symmetric(20, 12))
            .stroke(Stroke::new(1.0, p.border));
        egui::Panel::top("top")
            .frame(frame)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Path Hunter")
                            .size(20.0)
                            .strong()
                            .color(p.accent),
                    );
                    ui.label(RichText::new(format!("v{VERSION}")).color(p.muted));
                    ui.add_space(8.0);
                    let os = if cfg!(windows) {
                        "Windows"
                    } else if cfg!(target_os = "macos") {
                        "macOS"
                    } else {
                        "Unix"
                    };
                    badge(ui, os, p.muted);
                    if cfg!(windows) {
                        if self.scan.admin {
                            badge(ui, "Administrator", p.duplicate);
                        } else {
                            badge(ui, "Standard user", p.muted);
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let dark = ui.ctx().theme() == Theme::Dark;
                        if ui
                            .button(if dark { "Light mode" } else { "Dark mode" })
                            .clicked()
                        {
                            ui.ctx()
                                .set_theme(if dark { Theme::Light } else { Theme::Dark });
                        }
                        if ui.button("Rescan").clicked() {
                            self.rescan();
                        }
                        if cfg!(windows) {
                            let n = self.removal_count();
                            let label = if n > 0 {
                                format!("Clean up  ({n})")
                            } else {
                                "Clean up".to_owned()
                            };
                            let text = if self.clean_open {
                                RichText::new(label).color(p.accent)
                            } else {
                                RichText::new(label)
                            };
                            if ui.selectable_label(self.clean_open, text).clicked() {
                                self.clean_open = !self.clean_open;
                            }
                        }
                    });
                });
            });
    }

    // ── Status bar ──────────────────────────────────────────────────────

    fn status_bar(&mut self, ui: &mut Ui, p: &Palette) {
        let now = ui.input(|i| i.time);
        if self.toast.as_ref().is_some_and(|t| t.until < now) {
            self.toast = None;
        }
        let frame = Frame::new()
            .fill(p.panel)
            .inner_margin(Margin::symmetric(20, 8));
        egui::Panel::bottom("status")
            .frame(frame)
            .show_separator_line(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let src = match self.view {
                        PathView::Process => "this process's PATH",
                        PathView::Fresh => "registry PATH (as new programs see it)",
                    };
                    ui.label(
                        RichText::new(format!("{} entries  ·  {src}", self.scan.entries.len()))
                            .color(p.muted)
                            .small(),
                    );
                    if let Some(t) = &self.toast {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let c = if t.ok { p.ok } else { p.dead };
                            ui.label(RichText::new(&t.text).color(c).strong());
                        });
                        ui.ctx().request_repaint_after_secs(1.0);
                    }
                });
            });
    }

    // ── Summary cards ───────────────────────────────────────────────────

    fn summary(&mut self, ui: &mut Ui, p: &Palette) {
        ui.columns(Kind::ALL.len(), |cols| {
            for (ui, kind) in cols.iter_mut().zip(Kind::ALL) {
                let n = self.scan.count(kind);
                let active = self.filter == Filter::Only(kind);
                let color = p.kind(kind);
                let stroke = if active {
                    Stroke::new(1.5, color)
                } else {
                    Stroke::new(1.0, p.border)
                };
                let resp = Frame::new()
                    .fill(p.card)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width().max(0.0));
                        let num_color = if n == 0 { p.muted } else { color };
                        ui.label(
                            RichText::new(n.to_string())
                                .size(28.0)
                                .strong()
                                .color(num_color),
                        );
                        ui.horizontal(|ui| {
                            dot(ui, color);
                            ui.label(RichText::new(kind.plural()).color(p.muted));
                        });
                    })
                    .response
                    .interact(Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    self.filter = if active {
                        Filter::All
                    } else {
                        Filter::Only(kind)
                    };
                }
            }
        });
    }

    // ── Toolbar ─────────────────────────────────────────────────────────

    fn toolbar(&mut self, ui: &mut Ui, p: &Palette) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Filter by path or source…")
                    .desired_width(280.0)
                    .margin(Margin::symmetric(10, 6)),
            );
            ui.add_space(8.0);
            for (f, label) in [(Filter::All, "All"), (Filter::Issues, "Issues only")] {
                if ui.selectable_label(self.filter == f, label).clicked() {
                    self.filter = f;
                }
            }

            if cfg!(windows) {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut changed = false;
                    for (v, label, tip) in [
                        (
                            PathView::Fresh,
                            "Registry",
                            "System + User PATH from the registry — what newly launched programs get",
                        ),
                        (
                            PathView::Process,
                            "This process",
                            "The PATH this app inherited when it started",
                        ),
                    ] {
                        if ui
                            .selectable_label(self.view == v, label)
                            .on_hover_text(tip)
                            .clicked()
                            && self.view != v
                        {
                            self.view = v;
                            changed = true;
                        }
                    }
                    ui.label(RichText::new("View").color(p.muted));
                    if changed {
                        self.rescan();
                    }
                });
            }
        });
    }

    // ── Entry table ─────────────────────────────────────────────────────

    fn table(&mut self, ui: &mut Ui, p: &Palette) {
        let rows = self.visible();
        if rows.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                let msg = if self.scan.entries.is_empty() {
                    "PATH is not set or empty."
                } else {
                    "No entries match the current filter."
                };
                ui.label(RichText::new(msg).color(p.muted).size(16.0));
            });
            return;
        }

        let frame = Frame::new()
            .fill(p.card)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(8, 6));
        frame.show(ui, |ui| {
            ui.set_min_size(ui.available_size().max(egui::Vec2::ZERO));
            // Path is the flexible column, so it tracks the window width; once
            // the minimums no longer fit, the horizontal scroll area takes over.
            egui::ScrollArea::horizontal().show(ui, |ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .sense(Sense::click())
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .column(Column::exact(32.0))
                    .column(Column::exact(118.0))
                    .column(Column::remainder().at_least(200.0).clip(true))
                    .column(Column::initial(110.0).at_least(70.0).clip(true))
                    .column(Column::initial(180.0).at_least(100.0).clip(true))
                    .header(28.0, |mut h| {
                        for title in ["#", "Status", "Path", "Source", "Details"] {
                            h.col(|ui| {
                                ui.label(RichText::new(title).small().strong().color(p.muted));
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(32.0, rows.len(), |mut row| {
                            let idx = rows[row.index()];
                            let e = &self.scan.entries[idx];
                            let kind = Kind::of(&e.state);
                            let color = p.kind(kind);
                            row.col(|ui| {
                                ui.label(
                                    RichText::new((idx + 1).to_string()).small().color(p.muted),
                                );
                            });
                            row.col(|ui| pill(ui, kind.label(), color));
                            row.col(|ui| {
                                let mut text = RichText::new(&e.path).monospace();
                                if kind == Kind::Dead {
                                    text = text.color(color);
                                } else if kind == Kind::Empty {
                                    text = text.color(color).italics();
                                }
                                ui.add(egui::Label::new(text).truncate())
                                    .on_hover_text(&e.path);
                            });
                            row.col(|ui| {
                                if !e.source.is_empty() {
                                    badge(ui, &e.source, source_color(p, &e.source));
                                }
                            });
                            row.col(|ui| {
                                let d = detail(&e.state);
                                let c = if kind == Kind::Ok { p.muted } else { color };
                                ui.add(egui::Label::new(RichText::new(&d).color(c)).truncate())
                                    .on_hover_text(&d);
                            });

                            let path = e.path.clone();
                            let is_dir = matches!(e.state, EntryState::Ok | EntryState::Symlink(_));
                            row.response().context_menu(|ui| {
                                if ui.button("Copy path").clicked() {
                                    ui.ctx().copy_text(path.clone());
                                    ui.close();
                                }
                                if ui
                                    .add_enabled(is_dir, egui::Button::new("Open folder"))
                                    .clicked()
                                {
                                    open_folder(&path);
                                    ui.close();
                                }
                            });
                        });
                    });
            });
        });
    }
}

fn source_color(p: &Palette, source: &str) -> Color32 {
    match source {
        "User" => p.accent,
        "System" => p.symlink,
        _ => p.muted,
    }
}

fn open_folder(path: &str) {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(program).arg(path).spawn();
}

pub fn dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

/// Rounded status pill: colored dot + label on a tinted background.
pub fn pill(ui: &mut Ui, label: &str, color: Color32) {
    Frame::new()
        .fill(tint(color))
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.horizontal(|ui| {
                dot(ui, color);
                ui.label(RichText::new(label).color(color).strong().small());
            });
        });
}

pub fn badge(ui: &mut Ui, label: &str, color: Color32) {
    Frame::new()
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.5)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(label).color(color).small());
        });
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let p = Palette::of(ui.ctx());
        self.top_bar(ui, &p);
        self.status_bar(ui, &p);
        let body = ui.available_rect_before_wrap();

        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.bg).inner_margin(Margin::same(20)))
            .show(ui, |ui| {
                self.summary(ui, &p);
                ui.add_space(14.0);
                self.toolbar(ui, &p);
                ui.add_space(10.0);
                self.table(ui, &p);
            });

        // Drawn after the central panel so it floats on top of it.
        self.clean_panel(ui, &p, body);
        self.confirm_modal(ui, &p);
    }
}
