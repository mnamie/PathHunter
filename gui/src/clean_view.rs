//! The "Clean up" side panel and its confirmation dialog.

use std::collections::HashSet;

use eframe::egui::{self, CornerRadius, Frame, Margin, Rect, RichText, Stroke, Ui};

use pathhunter::cleanplan::{self, Selection, dead_key, norm};
use pathhunter::winenv;

use crate::app::{App, PathView, dot};
use crate::theme::{Palette, tint};

/// Segments of `old` missing from `new`. `new` is always an in-order
/// subsequence of `old` (cleaning only ever removes), so a single walk works.
fn removed(old: &[String], new: &[String]) -> Vec<String> {
    let mut rest = new.iter().peekable();
    old.iter()
        .filter(|s| {
            if rest.peek() == Some(s) {
                rest.next();
                false
            } else {
                true
            }
        })
        .cloned()
        .collect()
}

fn toggle(set: &mut HashSet<String>, key: &str, on: bool) {
    if on {
        set.insert(key.to_owned());
    } else {
        set.remove(key);
    }
}

/// One checkbox per item, plus a select-all toggle in the group header.
fn group(
    ui: &mut Ui,
    p: &Palette,
    title: &str,
    note: Option<&str>,
    enabled: bool,
    items: &[(String, String)], // (key, label)
    set: &mut HashSet<String>,
) {
    if items.is_empty() {
        return;
    }
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width().max(0.0));
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let all = items.iter().all(|(k, _)| set.contains(k));
                    let mut on = all;
                    if ui
                        .add_enabled(enabled, egui::Checkbox::without_text(&mut on))
                        .on_hover_text("Select all")
                        .changed()
                    {
                        for (k, _) in items {
                            toggle(set, k, on);
                        }
                    }
                });
            });
            if let Some(note) = note {
                ui.label(RichText::new(note).small().color(p.duplicate));
            }
            ui.add_space(4.0);
            ui.add_enabled_ui(enabled, |ui| {
                for (key, label) in items {
                    let mut on = enabled && set.contains(key);
                    let text = RichText::new(label).monospace();
                    if ui.checkbox(&mut on, text).changed() {
                        toggle(set, key, on);
                    }
                }
            });
        });
    ui.add_space(8.0);
}

fn info_group(ui: &mut Ui, p: &Palette, title: &str, note: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width().max(0.0));
            ui.label(RichText::new(title).strong());
            ui.label(RichText::new(note).small().color(p.muted));
            ui.add_space(4.0);
            for item in items {
                ui.horizontal(|ui| {
                    dot(ui, p.dead);
                    ui.label(RichText::new(item).monospace().color(p.muted));
                });
            }
        });
    ui.add_space(8.0);
}

impl App {
    pub fn default_selection(&self) -> Selection {
        let plan = &self.scan.plan;
        let admin = self.scan.admin;
        Selection {
            user_dead: plan.user_dead.iter().map(dead_key).collect(),
            system_dead: if admin {
                plan.system_dead.iter().map(dead_key).collect()
            } else {
                HashSet::new()
            },
            user_cross: plan.user_cross_dups.iter().map(|s| norm(s)).collect(),
            user_dups: plan.user_intra_dups.iter().map(|d| d.key.clone()).collect(),
            system_dups: if admin {
                plan.system_intra_dups
                    .iter()
                    .map(|d| d.key.clone())
                    .collect()
            } else {
                HashSet::new()
            },
        }
    }

    fn preview(&self) -> (Vec<String>, Vec<String>) {
        let s = &self.scan;
        cleanplan::apply(&s.user_segs, &s.system_segs, s.admin, &self.sel)
    }

    /// (removed from User, removed from System) for the current selection.
    fn removals(&self) -> (Vec<String>, Vec<String>) {
        let (user, system) = self.preview();
        (
            removed(&self.scan.user_segs, &user),
            removed(&self.scan.system_segs, &system),
        )
    }

    pub fn removal_count(&self) -> usize {
        if !cfg!(windows) {
            return 0;
        }
        let (u, s) = self.removals();
        u.len() + s.len()
    }

    /// Overlay floating over the right edge of `body` (the area between the
    /// top and status bars), so opening it never re-lays out the main view.
    pub fn clean_panel(&mut self, ui: &mut Ui, p: &Palette, body: Rect) {
        if !self.clean_open {
            return;
        }
        const WIDTH: f32 = 400.0;
        const MARGIN: i8 = 16;
        let rect = Rect::from_min_max(
            egui::pos2((body.right() - WIDTH).max(body.left()), body.top()),
            body.right_bottom(),
        );
        // Content size = outer rect minus inner margin and 1px stroke on each side.
        let inner =
            (rect.size() - egui::Vec2::splat(2.0 * (MARGIN as f32 + 1.0))).max(egui::Vec2::ZERO);
        let frame = Frame::new()
            .fill(p.panel)
            .inner_margin(Margin::same(MARGIN))
            .stroke(Stroke::new(1.0, p.border));
        egui::Area::new(egui::Id::new("clean"))
            .order(egui::Order::Middle)
            .fixed_pos(rect.min)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                frame.show(ui, |ui| {
                ui.set_min_size(inner);
                ui.set_max_size(inner);
                ui.heading("Clean up");
                ui.label(
                    RichText::new("Remove dead and redundant entries from the registry PATH.")
                        .color(p.muted),
                );
                ui.add_space(10.0);

                if !cfg!(windows) {
                    ui.label(
                        RichText::new("Clean is only supported on Windows.").color(p.muted),
                    );
                    return;
                }
                let plan = self.scan.plan.clone();
                if !plan.any_issue() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Nothing to clean.").size(16.0).color(p.ok));
                        ui.label(RichText::new("Your PATH looks tidy.").color(p.muted));
                    });
                    return;
                }

                let admin = self.scan.admin;
                let admin_note = (!admin).then_some("Requires administrator — will be skipped");
                let dead_items = |v: &[pathhunter::audit::PathEntry]| -> Vec<(String, String)> {
                    v.iter().map(|e| (dead_key(e), e.path.clone())).collect()
                };
                let dup_items = |v: &[cleanplan::DupInfo]| -> Vec<(String, String)> {
                    v.iter()
                        .map(|d| (d.key.clone(), format!("{}  ×{}", d.display, d.extra + 1)))
                        .collect()
                };

                let mut apply_clicked = false;
                egui::Panel::bottom("clean-actions")
                    .frame(Frame::new().inner_margin(Margin::symmetric(0, 10)))
                    .show_separator_line(false)
                    .show(ui, |ui| {
                        let n = self.removal_count();
                        let label = format!(
                            "Remove {n} {}",
                            if n == 1 { "entry" } else { "entries" }
                        );
                        let btn = egui::Button::new(RichText::new(label).strong().color(
                            if n > 0 { p.bg } else { p.muted },
                        ))
                        .fill(if n > 0 { p.accent } else { p.card })
                        .corner_radius(CornerRadius::same(8))
                        .min_size(egui::vec2(ui.available_width().max(0.0), 36.0));
                        apply_clicked = ui.add_enabled(n > 0, btn).clicked();
                    });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    group(
                        ui,
                        p,
                        "Dead · User",
                        None,
                        true,
                        &dead_items(&plan.user_dead),
                        &mut self.sel.user_dead,
                    );
                    group(
                        ui,
                        p,
                        "Dead · System",
                        admin_note,
                        admin,
                        &dead_items(&plan.system_dead),
                        &mut self.sel.system_dead,
                    );
                    let cross: Vec<(String, String)> = plan
                        .user_cross_dups
                        .iter()
                        .map(|s| (norm(s), s.clone()))
                        .collect();
                    group(
                        ui,
                        p,
                        "Redundant · User entries already in System",
                        None,
                        true,
                        &cross,
                        &mut self.sel.user_cross,
                    );
                    group(
                        ui,
                        p,
                        "Duplicates within User",
                        None,
                        true,
                        &dup_items(&plan.user_intra_dups),
                        &mut self.sel.user_dups,
                    );
                    group(
                        ui,
                        p,
                        "Duplicates within System",
                        admin_note,
                        admin,
                        &dup_items(&plan.system_intra_dups),
                        &mut self.sel.system_dups,
                    );
                    let shell: Vec<String> =
                        plan.shell_dead.iter().map(|e| e.path.clone()).collect();
                    info_group(
                        ui,
                        p,
                        "Dead · Shell-injected",
                        "Not in the registry (added by a shell or launcher) — can't be cleaned here.",
                        &shell,
                    );
                });

                if apply_clicked {
                    self.confirm_open = true;
                }
                });
            });
    }

    pub fn confirm_modal(&mut self, ui: &mut Ui, p: &Palette) {
        if !self.confirm_open {
            return;
        }
        let (user_rm, system_rm) = self.removals();
        let mut confirmed = false;
        let mut cancelled = false;
        let resp = egui::Modal::new(egui::Id::new("confirm-clean"))
            .frame(
                Frame::new()
                    .fill(p.card)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::same(22)),
            )
            .show(ui.ctx(), |ui| {
                ui.set_width(520.0);
                ui.heading("Apply changes?");
                ui.label(
                    RichText::new("These segments will be removed from the registry PATH:")
                        .color(p.muted),
                );
                ui.add_space(10.0);
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (hive, list) in [("User", &user_rm), ("System", &system_rm)] {
                            if list.is_empty() {
                                continue;
                            }
                            ui.label(RichText::new(format!("{hive} PATH")).strong());
                            for s in list {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("−").color(p.dead).strong());
                                    ui.label(RichText::new(s).monospace());
                                });
                            }
                            ui.add_space(6.0);
                        }
                    });
                ui.add_space(8.0);
                Frame::new()
                    .fill(tint(p.duplicate))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Running shells keep their old PATH until restarted.")
                                .small()
                                .color(p.duplicate),
                        );
                    });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let apply = egui::Button::new(RichText::new("Apply").strong().color(p.bg))
                        .fill(p.dead)
                        .corner_radius(CornerRadius::same(8));
                    if ui.add(apply).clicked() {
                        confirmed = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancelled = true;
                    }
                });
            });
        if cancelled || resp.should_close() {
            self.confirm_open = false;
        }
        if confirmed {
            self.confirm_open = false;
            self.apply_clean(ui);
        }
    }

    fn apply_clean(&mut self, ui: &Ui) {
        let (user, system) = self.preview();
        let mut ok = true;
        let mut changed = false;
        if user != self.scan.user_segs {
            ok &= winenv::write_user_path_to_registry(&user);
            changed = true;
        }
        if self.scan.admin && system != self.scan.system_segs {
            ok &= winenv::write_system_path_to_registry(&system);
            changed = true;
        }
        if changed {
            winenv::broadcast_env_change();
        }
        // This process's own PATH is now stale; show what new programs get.
        self.view = PathView::Fresh;
        self.rescan();
        if ok {
            self.notify(
                ui,
                "PATH updated. Restart your shell for changes to take effect.",
                true,
            );
        } else {
            self.notify(ui, "Failed to write the registry PATH.", false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn removed_handles_duplicates() {
        let old = segs(&["a", "b", "a", "c", "a"]);
        let new = segs(&["a", "b", "c"]);
        assert_eq!(removed(&old, &new), segs(&["a", "a"]));
    }

    #[test]
    fn removed_nothing() {
        let old = segs(&["a", "b"]);
        assert!(removed(&old, &old).is_empty());
    }
}
