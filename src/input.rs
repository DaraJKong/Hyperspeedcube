use glium::glutin::event::*;
use std::collections::HashSet;
use std::ops::Index;

use crate::puzzle::{traits::*, PuzzleController, PuzzleEnum, Rubiks3D, Rubiks4D};

const SHIFT: ModifiersState = ModifiersState::SHIFT;
const CTRL: ModifiersState = ModifiersState::CTRL;
const ALT: ModifiersState = ModifiersState::ALT;
const LOGO: ModifiersState = ModifiersState::LOGO;

#[must_use = "call finish()"]
pub struct FrameInProgress<'a> {
    state: &'a mut State,
    puzzle: &'a mut PuzzleEnum,
}
impl FrameInProgress<'_> {
    pub fn handle_event(&mut self, ev: &Event<'_, ()>) {
        match ev {
            // Handle WindowEvents.
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        self.state.keys.update(*input);
                        if self.state.has_keyboard {
                            self.handle_key(*input);
                        }
                    }
                    WindowEvent::ModifiersChanged(new_modifiers) => {
                        self.state.modifiers = *new_modifiers;
                    }

                    // Ignore other `WindowEvent`s.
                    _ => (),
                }
            }

            // Ignore non-`WindowEvent`s.
            _ => (),
        }
    }

    fn handle_key(&mut self, input: KeyboardInput) {
        // We don't care about left vs. right modifiers, so just extract
        // the bits that don't specify left vs. right.
        let modifiers = self.state.modifiers & (SHIFT | CTRL | ALT | LOGO);

        if (modifiers & (CTRL | ALT | LOGO)).is_empty() {
            if let KeyboardInput {
                state: ElementState::Pressed,
                virtual_keycode: Some(keycode),
                ..
            } = input
            {
                match self.puzzle {
                    PuzzleEnum::Rubiks3D(cube) => handle_key_rubiks3d(cube, keycode, self.state),
                    PuzzleEnum::Rubiks4D(cube) => handle_key_rubiks4d(cube, keycode, self.state),
                }
            }
        } else if input.state == ElementState::Pressed {
            if modifiers == CTRL {
                match input.virtual_keycode {
                    // Undo.
                    Some(VirtualKeyCode::Z) => println!("TODO undo"),
                    // Redo.
                    Some(VirtualKeyCode::Y) => println!("TODO redo"),
                    // Reset.
                    Some(VirtualKeyCode::R) => println!("TODO reset puzzle state"),
                    // Copy puzzle state.
                    Some(VirtualKeyCode::C) => println!("TODO copy puzzle state"),
                    // Paste puzzle state.
                    Some(VirtualKeyCode::V) => println!("TODO paste puzzle state"),
                    // Full scramble.
                    Some(VirtualKeyCode::F) => println!("TODO full scramble"),
                    // Partial scramble.
                    Some(VirtualKeyCode::Key1) => println!("TODO scramble 1"),
                    Some(VirtualKeyCode::Key2) => println!("TODO scramble 2"),
                    Some(VirtualKeyCode::Key3) => println!("TODO scramble 3"),
                    Some(VirtualKeyCode::Key4) => println!("TODO scramble 4"),
                    Some(VirtualKeyCode::Key5) => println!("TODO scramble 5"),
                    Some(VirtualKeyCode::Key6) => println!("TODO scramble 6"),
                    Some(VirtualKeyCode::Key7) => println!("TODO scramble 7"),
                    Some(VirtualKeyCode::Key8) => println!("TODO scramble 8"),
                    _ => (),
                }
            }

            if modifiers == SHIFT | CTRL {
                match input.virtual_keycode {
                    // Redo.
                    Some(VirtualKeyCode::Z) => println!("TODO redo"),
                    _ => (),
                }
            }
        }
    }

    pub fn finish(self) {
        let mut config = crate::get_config();

        let speed = 1.0_f32.to_radians();

        if self.state.keys[VirtualKeyCode::Up] {
            config.gfx.theta += speed;
        }
        if self.state.keys[VirtualKeyCode::Down] {
            config.gfx.theta -= speed;
        }
        if self.state.keys[VirtualKeyCode::Right] {
            config.gfx.phi += speed;
        }
        if self.state.keys[VirtualKeyCode::Left] {
            config.gfx.phi -= speed;
        }

        match self.puzzle {
            PuzzleEnum::Rubiks3D(cube) => update_display_rubiks3d(cube, self.state),
            PuzzleEnum::Rubiks4D(cube) => update_display_rubiks4d(cube, self.state),
        }
    }
}

#[derive(Debug, Default)]
pub struct State {
    /// Set of pressed keys.
    keys: KeysPressed,
    /// Set of pressed modifiers.
    modifiers: ModifiersState,
    /// Whether to handle keyboard input (false if it is captured by imgui).
    has_keyboard: bool,
}
impl State {
    pub fn frame<'a>(
        &'a mut self,
        puzzle: &'a mut PuzzleEnum,
        imgui_io: &imgui::Io,
    ) -> FrameInProgress<'a> {
        self.has_keyboard = !imgui_io.want_capture_keyboard;
        FrameInProgress {
            state: self,
            puzzle,
        }
    }
}

// TODO: document this
#[derive(Debug, Default)]
struct KeysPressed {
    /// The set of scancodes for keys that are held.
    scancodes: HashSet<u32>,
    /// The set of virtual keycodes for keys that are held.
    virtual_keycodes: HashSet<VirtualKeyCode>,
}
impl KeysPressed {
    /// Updates internal key state based on a KeyboardInput event.
    pub fn update(&mut self, input: KeyboardInput) {
        match input.state {
            ElementState::Pressed => {
                self.scancodes.insert(input.scancode);
                if let Some(virtual_keycode) = input.virtual_keycode {
                    self.virtual_keycodes.insert(virtual_keycode);
                }
            }
            ElementState::Released => {
                self.scancodes.remove(&input.scancode);
                if let Some(virtual_keycode) = input.virtual_keycode {
                    self.virtual_keycodes.remove(&virtual_keycode);
                }
            }
        }
    }
}
impl Index<u32> for KeysPressed {
    type Output = bool;
    fn index(&self, scancode: u32) -> &bool {
        if self.scancodes.contains(&scancode) {
            &true
        } else {
            &false
        }
    }
}
impl Index<VirtualKeyCode> for KeysPressed {
    type Output = bool;
    fn index(&self, virtual_keycode: VirtualKeyCode) -> &bool {
        if self.virtual_keycodes.contains(&virtual_keycode) {
            &true
        } else {
            &false
        }
    }
}

fn handle_key_rubiks3d(
    cube: &mut PuzzleController<Rubiks3D>,
    keycode: VirtualKeyCode,
    state: &mut State,
) {
    use crate::puzzle::rubiks3d::twists;
    use VirtualKeyCode as Vk;

    if state.modifiers.shift() {
        match keycode {
            _ => (),
        }
    } else {
        match keycode {
            Vk::U => cube.twist(twists::R),
            Vk::E => cube.twist(twists::R.rev()),
            Vk::L => cube.twist(twists::R.fat()),
            Vk::M => cube.twist(twists::R.fat().rev()),
            Vk::N => cube.twist(twists::U),
            Vk::T => cube.twist(twists::U.rev()),
            Vk::S => cube.twist(twists::L),
            Vk::F => cube.twist(twists::L.rev()),
            Vk::V => cube.twist(twists::L.fat()),
            Vk::P => cube.twist(twists::L.fat().rev()),
            Vk::R => cube.twist(twists::D),
            Vk::I => cube.twist(twists::D.rev()),
            Vk::H => cube.twist(twists::F),
            Vk::D => cube.twist(twists::F.rev()),
            Vk::W => cube.twist(twists::B),
            Vk::Y => cube.twist(twists::B.rev()),
            Vk::G | Vk::J => cube.twist(twists::X),
            Vk::B | Vk::K => cube.twist(twists::X.rev()),
            Vk::O => cube.twist(twists::Y),
            Vk::A => cube.twist(twists::Y.rev()),
            Vk::Semicolon => cube.twist(twists::Z),
            Vk::Q => cube.twist(twists::Z.rev()),
            _ => (),
        }
    }
}

fn handle_key_rubiks4d(
    cube: &mut PuzzleController<Rubiks4D>,
    keycode: VirtualKeyCode,
    state: &mut State,
) {
    use crate::puzzle::rubiks4d::twists;
    use VirtualKeyCode as Vk;

    if state.modifiers.shift() {
        match keycode {
            // TODO
            _ => (),
        }
    } else {
        match keycode {
            // TODO
            _ => (),
        }
    }
}

fn update_display_rubiks3d(_cube: &mut PuzzleController<Rubiks3D>, _state: &mut State) {}

fn update_display_rubiks4d(cube: &mut PuzzleController<Rubiks4D>, state: &mut State) {
    // TODO
}