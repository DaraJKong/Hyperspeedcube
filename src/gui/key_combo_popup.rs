use key_names::KeyMappingCode;
use std::sync::Arc;
use winit::event::{ElementState, ModifiersState, VirtualKeyCode, WindowEvent};

use super::components::KeybindSetAccessor;
use super::ext::*;
use crate::app::App;
use crate::preferences::{Key, KeyCombo};

const KEYBIND_POPUP_SIZE: egui::Vec2 = egui::vec2(300.0, 200.0);

const SCANCODE_EXPLANATION: &str = "Scancodes are based on physical key position, while virtual keycodes depend on the keyboard layout";

#[derive(Default, Clone)]
pub(super) struct State {
    /// Callback to set the new key combo. This is `None` to indicate that the
    /// popup is closed.
    callback: Option<Arc<dyn Send + Sync + Fn(&mut App, KeyCombo)>>,

    key_combo: Option<KeyCombo>,

    mods: ModifiersState,
    ordered_pressed_sc: Vec<Key>,
    ordered_pressed_vk: Vec<Key>,

    use_vk: bool,
    use_vk_id: Option<egui::Id>,
}
impl State {
    fn update_keybind(&mut self) {
        let keys: Vec<Key>;

        if self.use_vk {
            keys = self
                .ordered_pressed_vk
                .clone()
                .into_iter()
                .filter(|key| match key {
                    Key::Vk(_) => true,
                    _ => false,
                })
                .collect();
        } else {
            keys = self
                .ordered_pressed_sc
                .clone()
                .into_iter()
                .filter(|key| match key {
                    Key::Sc(_) => true,
                    _ => false,
                })
                .collect();
        }

        self.key_combo = Some(KeyCombo::new(keys, self.mods));
    }
    fn set_key(&mut self, sc: Option<KeyMappingCode>, vk: Option<VirtualKeyCode>) {
        self.ordered_pressed_sc.retain(|&key| !key.is_modifier());
        self.ordered_pressed_vk.retain(|&key| !key.is_modifier());
        if let Some(sc) = sc {
            if !self.ordered_pressed_sc.contains(&Key::Sc(sc)) {
                self.ordered_pressed_sc.push(Key::Sc(sc));
            }
        }
        if let Some(vk) = vk {
            if !self.ordered_pressed_vk.contains(&Key::Vk(vk)) {
                self.ordered_pressed_vk.push(Key::Vk(vk));
            }
        }
        self.update_keybind();
    }
    fn remove_key(&mut self, sc: Option<KeyMappingCode>, vk: Option<VirtualKeyCode>) {
        self.ordered_pressed_sc.retain(|&k| {
            if let Some(sc) = sc {
                k != Key::Sc(sc)
            } else {
                true
            }
        });

        self.ordered_pressed_vk.retain(|&k| {
            if let Some(vk) = vk {
                k != Key::Vk(vk)
            } else {
                true
            }
        });
    }
    fn confirm(&mut self, app: &mut App) {
        if let Some(callback) = self.callback.take() {
            callback(app, self.key_combo.clone().unwrap_or_default());
        }
    }
    fn cancel(&mut self) {
        self.callback = None;
    }
}

pub(super) fn popup_state_mut(data: &mut egui::util::IdTypeMap) -> &mut State {
    data.get_temp_mut_or_default(popup_state_id())
}
pub(super) fn popup_state(ctx: &egui::Context) -> State {
    ctx.data().get_temp(popup_state_id()).unwrap_or_default()
}
fn popup_state_id() -> egui::Id {
    unique_id!()
}

pub(super) fn open<S: KeybindSetAccessor>(
    ctx: &egui::Context,
    key_combo: Option<KeyCombo>,
    keybind_set: S,
    idx: usize,
) {
    let mut data = ctx.data();

    // Global keybinds should use virtual keycodes by default, while puzzle
    // keybinds should use scancodes by default. If the user manually overrides
    // one, remember that decision for as long as the application is running.
    let use_vk_id = unique_id!().with(S::USE_VK_BY_DEFAULT);
    let use_vk = data.get_temp(use_vk_id).unwrap_or(S::USE_VK_BY_DEFAULT);

    *popup_state_mut(&mut data) = State {
        callback: Some(Arc::new(move |app, new_key_combo| {
            keybind_set.get_mut(&mut app.prefs)[idx].key = new_key_combo;
            app.prefs.needs_save = true;
        })),

        key_combo,

        mods: ModifiersState::empty(),
        ordered_pressed_sc: Vec::new(),
        ordered_pressed_vk: Vec::new(),

        use_vk,
        use_vk_id: Some(use_vk_id),
    };
}

pub(super) fn build(ctx: &egui::Context, app: &mut App) -> Option<egui::Response> {
    #[allow(clippy::question_mark)]
    if popup_state(ctx).callback.is_none() {
        return None;
    }

    popup_state_mut(&mut ctx.data()).mods = app.pressed_modifiers();

    let r = egui::Area::new("keybind_popup")
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(ui.visuals().window_fill())
                .rounding(ui.visuals().window_rounding)
                .shadow(ui.visuals().window_shadow)
                .stroke(ui.visuals().window_stroke())
                .inner_margin(egui::style::Margin::same(20.0))
                .show(ui, |ui| {
                    ui.allocate_ui_with_layout(
                        KEYBIND_POPUP_SIZE,
                        egui::Layout::top_down_justified(egui::Align::LEFT),
                        |ui| {
                            ui.vertical_centered(|ui| {
                                ui.spacing_mut().item_spacing.y = 20.0;

                                ui.heading("Press a key combination");

                                let key_combo = popup_state(ctx).key_combo.unwrap_or_default();
                                if key_combo.keys().len() > 0 {
                                    ui.strong(key_combo.to_string());
                                } else {
                                    ui.strong("(press a key)");
                                }

                                ui.columns(2, |columns| {
                                    let r = columns[0].with_layout(
                                        egui::Layout::top_down(egui::Align::RIGHT),
                                        |ui| ui.add_sized([60.0, 30.0], egui::Button::new("OK")),
                                    );
                                    if r.inner.clicked() {
                                        popup_state_mut(&mut ctx.data()).confirm(app);
                                    }

                                    let r = columns[1].with_layout(
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        |ui| {
                                            ui.add_sized([60.0, 30.0], egui::Button::new("Cancel"))
                                        },
                                    );
                                    if r.inner.clicked() {
                                        popup_state_mut(&mut ctx.data()).cancel();
                                    }
                                });

                                ui.separator();

                                let mut use_vk = popup_state(ctx).use_vk;
                                let mut changed = false;
                                ui.horizontal(|ui| {
                                    ui.label("Key type:");
                                    let r = ui.selectable_value(&mut use_vk, false, "Scancode");
                                    changed |= r.changed();
                                    let r = ui.selectable_value(&mut use_vk, true, "Keycode");
                                    changed |= r.changed();
                                })
                                .response
                                .on_hover_explanation("", SCANCODE_EXPLANATION);
                                if changed {
                                    let mut data = ctx.data();
                                    let popup = popup_state_mut(&mut data);
                                    let use_vk_id = popup.use_vk_id;
                                    popup.use_vk = use_vk;
                                    popup.update_keybind();
                                    if let Some(id) = use_vk_id {
                                        data.insert_temp(id, use_vk);
                                    }
                                }

                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing.y = ui.spacing().item_spacing.x;

                                    if ui.button("Bind Escape key").clicked() {
                                        popup_state_mut(&mut ctx.data()).set_key(
                                            Some(KeyMappingCode::Escape),
                                            Some(VirtualKeyCode::Escape),
                                        );
                                    }
                                    if ui.button("Bind Enter key").clicked() {
                                        popup_state_mut(&mut ctx.data()).set_key(
                                            Some(KeyMappingCode::Enter),
                                            Some(VirtualKeyCode::Return),
                                        );
                                    }
                                    if ui.button("Bind Numpad Enter key").clicked() {
                                        popup_state_mut(&mut ctx.data()).set_key(
                                            Some(KeyMappingCode::NumpadEnter),
                                            Some(VirtualKeyCode::NumpadEnter),
                                        );
                                    }
                                });
                            });
                        },
                    );
                });
        });

    Some(r.response)
}

/// Returns `true` if the key combo popup should handle the event exclusively.
/// Always call `key_combo_popup_handle_event()`, even if this function returns
/// `false`.
pub(crate) fn key_combo_popup_captures_event(ctx: &egui::Context, event: &WindowEvent) -> bool {
    let mut data = ctx.data();
    let popup = popup_state_mut(&mut data);

    popup.callback.is_some() && matches!(event, WindowEvent::KeyboardInput { .. })
}

/// Handles keyboard events for the keybind popup, if it is open. Returns `true`
/// if the event is consumed.
pub(crate) fn key_combo_popup_handle_event(
    ctx: &egui::Context,
    app: &mut App,
    event: &WindowEvent,
) {
    let mut data = ctx.data();
    let popup = popup_state_mut(&mut data);

    if popup.callback.is_some() {
        match event {
            winit::event::WindowEvent::KeyboardInput { input, .. }
                if input.state == ElementState::Pressed =>
            {
                match input.virtual_keycode {
                    Some(VirtualKeyCode::Return) if popup.ordered_pressed_vk.len() == 1 => {
                        popup.confirm(app)
                    }
                    Some(VirtualKeyCode::Escape) if popup.ordered_pressed_vk.len() == 1 => {
                        popup.cancel()
                    }
                    _ => {
                        let sc = key_names::sc_to_key(input.scancode as u16);
                        let vk = input.virtual_keycode;

                        popup.set_key(sc, vk);
                    }
                }
            }
            winit::event::WindowEvent::KeyboardInput { input, .. }
                if input.state == ElementState::Released =>
            {
                let sc = key_names::sc_to_key(input.scancode as u16);
                let vk = input.virtual_keycode;

                popup.remove_key(sc, vk)
            }

            // Will have to remove this in the future if not used
            winit::event::WindowEvent::ModifiersChanged(mods) => popup.mods = *mods,

            _ => (),
        }
    }
}
