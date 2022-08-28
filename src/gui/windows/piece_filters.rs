use crate::app::App;
use crate::gui::{util, widgets};
use crate::preferences::{PieceFilter, DEFAULT_PREFS};
use crate::puzzle::{traits::*, Face, Piece, PieceType};

const MIN_WIDTH: f32 = 300.0;

pub fn cleanup(app: &mut App) {
    app.puzzle.set_preview_hidden(|_| None);
}

pub fn build(ui: &mut egui::Ui, app: &mut App) {
    app.puzzle.set_preview_hidden(|_| None);

    let puzzle_type = app.puzzle.ty();

    ui.set_min_width(MIN_WIDTH);

    let prefs = &mut app.prefs;

    let mut changed = false;
    let mut prefs_ui = util::PrefsUi {
        ui,
        current: &mut prefs.opacity,
        defaults: &DEFAULT_PREFS.opacity,
        changed: &mut changed,
    };

    prefs_ui.percent("Hidden", access!(.hidden));
    crate::gui::prefs::build_unhide_grip_checkbox(&mut prefs_ui);

    prefs.needs_save |= changed;
    if changed {
        app.request_redraw_puzzle();
    }

    ui.separator();

    PieceFilterWidget::new_uppercased("everything", |_| true)
        .no_all_except()
        .show(ui, app);

    ui.collapsing("Types", |ui| {
        for (i, piece_type) in puzzle_type.piece_types().iter().enumerate() {
            PieceFilterWidget::new_uppercased(&format!("{}s", piece_type.name), move |piece| {
                puzzle_type.info(piece).piece_type == PieceType(i as _)
            })
            .show(ui, app);
        }
    });

    ui.collapsing("Colors", |ui| {
        ui.set_enabled(!app.prefs.colors.blindfold);

        let face_colors = app.prefs.colors.face_colors_list(app.puzzle.ty());

        let colors_selection_id = unique_id!();
        let mut selected_colors: Vec<bool> =
            ui.data().get_temp(colors_selection_id).unwrap_or_default();
        selected_colors.resize(app.puzzle.faces().len(), false);

        for i in 0..puzzle_type.faces().len() {
            PieceFilterWidget::new_uppercased("pieces with this color", move |piece| {
                let mut stickers = puzzle_type.info(piece).stickers.iter();
                stickers.any(|&sticker| puzzle_type.info(sticker).color == Face(i as _))
            })
            .label_ui(|ui: &mut egui::Ui| {
                ui.horizontal(|ui| {
                    egui::color_picker::show_color(ui, face_colors[i], ui.spacing().interact_size);
                    ui.checkbox(&mut selected_colors[i], "");
                })
                .response
            })
            .show(ui, app);
        }

        ui.add_enabled_ui(selected_colors.contains(&true), |ui| {
            PieceFilterWidget::new_uppercased("pieces with all these colors", |piece| {
                let stickers = &puzzle_type.info(piece).stickers;
                selected_colors.iter().enumerate().all(|(i, selected)| {
                    !selected
                        || stickers
                            .iter()
                            .any(|&s| puzzle_type.info(s).color == Face(i as _))
                })
            })
            .show(ui, app);
            PieceFilterWidget::new_uppercased("pieces with any of these colors", |piece| {
                let stickers = &puzzle_type.info(piece).stickers;
                stickers
                    .iter()
                    .any(|&s| selected_colors[puzzle_type.info(s).color.0 as usize])
            })
            .show(ui, app);
            PieceFilterWidget::new_uppercased("pieces with only these colors", |piece| {
                let stickers = &puzzle_type.info(piece).stickers;
                stickers
                    .iter()
                    .all(|&s| selected_colors[puzzle_type.info(s).color.0 as usize])
            })
            .show(ui, app);
        });

        ui.data().insert_temp(colors_selection_id, selected_colors);
    });

    ui.collapsing("Presets", |ui| {
        ui.set_enabled(!app.prefs.colors.blindfold);

        let opacity_prefs = &mut app.prefs.opacity;
        let mut piece_filter_presets = std::mem::take(&mut app.prefs.piece_filters[puzzle_type]);

        let mut changed = false;

        let mut presets_ui = widgets::PresetsUi {
            id: unique_id!(),
            presets: &mut piece_filter_presets,
            changed: &mut changed,
        };

        presets_ui.show_header(ui, || PieceFilter {
            visible_pieces: app.puzzle.visible_pieces_string(),
            hidden_opacity: opacity_prefs
                .save_opacity_in_piece_filter_preset
                .then_some(opacity_prefs.hidden),
        });
        presets_ui.show_postheader(ui, |ui| {
            ui.checkbox(
                &mut opacity_prefs.save_opacity_in_piece_filter_preset,
                "Save opacity",
            );
        });
        ui.separator();
        presets_ui.show_list(ui, |ui, idx, preset| {
            PieceFilterWidget::new(
                &preset.preset_name,
                |piece| crate::util::b16_fetch_bit(&preset.value.visible_pieces, piece.0 as _),
                preset.value.hidden_opacity,
            )
            .show(ui, app)
        });

        app.prefs.piece_filters[puzzle_type] = piece_filter_presets;

        app.prefs.needs_save |= changed;
    });
}

#[must_use]
struct PieceFilterWidget<'a, W, P> {
    name: &'a str,
    label_ui: W,
    highlight_if_active: bool,
    all_except: bool,
    predicate: P,
    hidden_opacity: Option<f32>,
}
impl<'a, P> PieceFilterWidget<'a, egui::Button, P>
where
    P: Copy + FnMut(Piece) -> bool,
{
    fn new_uppercased(name: &'a str, predicate: P) -> Self {
        let mut s = name.to_string();
        s[0..1].make_ascii_uppercase();
        Self::new_with_label(name, &s, predicate, None)
    }
    fn new(name: &'a str, predicate: P, hidden_opacity: Option<f32>) -> Self {
        Self::new_with_label(name, name, predicate, hidden_opacity)
    }
    fn new_with_label(
        name: &'a str,
        label: &str,
        predicate: P,
        hidden_opacity: Option<f32>,
    ) -> Self {
        Self {
            name,
            label_ui: egui::Button::new(label).frame(false),
            highlight_if_active: true,
            all_except: true,
            predicate,
            hidden_opacity,
        }
    }
}
impl<'a, W, P> PieceFilterWidget<'a, W, P>
where
    W: egui::Widget,
    P: Copy + FnMut(Piece) -> bool,
{
    fn label_ui<W2>(self, label_ui: W2) -> PieceFilterWidget<'a, W2, P> {
        PieceFilterWidget {
            name: self.name,
            label_ui,
            highlight_if_active: false,
            all_except: self.all_except,
            predicate: self.predicate,
            hidden_opacity: self.hidden_opacity,
        }
    }

    fn no_all_except(mut self) -> Self {
        self.all_except = false;
        self
    }

    fn show(mut self, ui: &mut egui::Ui, app: &mut App) -> egui::Response {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                let puzzle = &mut app.puzzle;

                ui.spacing_mut().item_spacing.x /= 2.0;

                let r = ui.add_enabled(
                    !puzzle.are_all_shown(self.predicate),
                    |ui: &mut egui::Ui| {
                        widgets::small_icon_button(ui, "👁", &format!("Show {}", self.name))
                    },
                );
                if r.hovered() {
                    puzzle.set_preview_hidden(|piece| (self.predicate)(piece).then_some(false));
                }
                if r.clicked() {
                    puzzle.show(self.predicate);
                }

                let r = ui.add_enabled(
                    !puzzle.are_all_hidden(self.predicate),
                    |ui: &mut egui::Ui| {
                        widgets::small_icon_button(ui, "ｘ", &format!("Hide {}", self.name))
                    },
                );
                if r.hovered() {
                    puzzle.set_preview_hidden(|piece| (self.predicate)(piece).then_some(true));
                }
                if r.clicked() {
                    puzzle.hide(self.predicate);
                }

                if self.all_except {
                    let r = ui.add_enabled(
                        !puzzle.are_all_hidden(|p| !(self.predicate)(p)),
                        |ui: &mut egui::Ui| {
                            widgets::small_icon_button(
                                ui,
                                "❎",
                                &format!("Hide all except {}", self.name),
                            )
                        },
                    );
                    if r.hovered() {
                        puzzle
                            .set_preview_hidden(|piece| (!(self.predicate)(piece)).then_some(true));
                    }
                    if r.clicked() {
                        puzzle.hide(|p| !(self.predicate)(p));
                    }
                }

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.min_size().y),
                    egui::Layout::centered_and_justified(egui::Direction::TopDown)
                        .with_cross_align(egui::Align::LEFT),
                    |ui| {
                        let puzzle = &mut app.puzzle;

                        // Highlight name of active filter.
                        if puzzle.are_all_shown(self.predicate)
                            && puzzle.are_all_hidden(|p| !(self.predicate)(p))
                        {
                            let visuals = ui.visuals_mut();
                            visuals.widgets.hovered = visuals.widgets.active;
                            visuals.widgets.inactive = visuals.widgets.active;
                        }

                        let r = ui.add(self.label_ui);
                        if r.hovered() {
                            puzzle.set_preview_hidden(|piece| Some(!(self.predicate)(piece)));
                        }
                        if r.clicked() {
                            puzzle.hide(|_| true);
                            puzzle.show(self.predicate);
                            if let Some(hidden_opacity) = self.hidden_opacity {
                                if app.prefs.opacity.hidden != hidden_opacity {
                                    app.prefs.opacity.hidden = hidden_opacity;
                                    app.prefs.needs_save = true;
                                    app.request_redraw_puzzle();
                                }
                            }
                        }
                    },
                );
            });
        })
        .response
    }
}
