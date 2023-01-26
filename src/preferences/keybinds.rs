use key_names::KeyMappingCode;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use winit::event::{ModifiersState, VirtualKeyCode};

use super::is_false;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct KeybindSet<C: Default> {
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub includes: BTreeSet<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keybinds: Vec<Keybind<C>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Keybind<C> {
    #[serde(flatten, deserialize_with = "deser_valid_key_combo")]
    pub key: KeyCombo,
    pub command: C,
}
fn deser_valid_key_combo<'de, D: Deserializer<'de>>(deserializer: D) -> Result<KeyCombo, D::Error> {
    KeyCombo::deserialize(deserializer).map(KeyCombo::validate)
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct KeyCombo {
    pub keys: Vec<Key>,

    #[serde(skip_serializing_if = "is_false")]
    ctrl: bool,
    #[serde(skip_serializing_if = "is_false")]
    shift: bool,
    #[serde(skip_serializing_if = "is_false")]
    alt: bool,
    #[serde(skip_serializing_if = "is_false")]
    logo: bool,
}
impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mods = key_names::mods_prefix_string(self.shift, self.ctrl, self.alt, self.logo);
        write!(f, "{}", mods)?;

        let mut display_text = String::new();

        for key in self.keys() {
            if display_text.len() > 0 {
                display_text.push_str(" + ");
            }

            match key {
                Key::Sc(sc) => display_text.push_str(key_names::key_name(*sc).as_str()),
                Key::Vk(vk) => match vk {
                    VirtualKeyCode::Key1 => display_text.push_str("1"),
                    VirtualKeyCode::Key2 => display_text.push_str("2"),
                    VirtualKeyCode::Key3 => display_text.push_str("3"),
                    VirtualKeyCode::Key4 => display_text.push_str("4"),
                    VirtualKeyCode::Key5 => display_text.push_str("5"),
                    VirtualKeyCode::Key6 => display_text.push_str("6"),
                    VirtualKeyCode::Key7 => display_text.push_str("7"),
                    VirtualKeyCode::Key8 => display_text.push_str("8"),
                    VirtualKeyCode::Key9 => display_text.push_str("9"),
                    VirtualKeyCode::Key0 => display_text.push_str("0"),
                    VirtualKeyCode::Scroll => display_text.push_str("ScrollLock"),
                    VirtualKeyCode::Back => display_text.push_str("Backspace"),
                    VirtualKeyCode::Return => display_text.push_str("Enter"),
                    VirtualKeyCode::Capital => display_text.push_str("CapsLock"),
                    other => display_text.push_str(format!("{:?}", other).as_str()),
                },
            }
        }

        write!(f, "{}", display_text)
    }
}
impl KeyCombo {
    pub fn new(keys: Vec<Key>, mods: ModifiersState) -> Self {
        Self {
            keys,
            ctrl: mods.ctrl(),
            shift: mods.shift(),
            alt: mods.alt(),
            logo: mods.logo(),
        }
        .validate()
    }
    #[must_use]
    pub fn validate(self) -> Self {
        let (mut ctrl, mut shift, mut alt, mut logo) = (false, false, false, false);

        for key in self.keys() {
            ctrl |= key.is_ctrl();
            shift |= key.is_shift();
            alt |= key.is_alt();
            logo |= key.is_logo();
        }

        Self {
            keys: self.keys.clone(),

            // If a `key` in keys is equivalent to a modifier key, exclude it from the
            // modifier booleans.
            ctrl: *self.ctrl() && !ctrl,
            shift: *self.shift() && !shift,
            alt: *self.alt() && !alt,
            logo: *self.logo() && !logo,
        }
    }
    pub fn keys(&self) -> &Vec<Key> {
        &self.keys
    }
    pub fn ctrl(&self) -> &bool {
        &self.ctrl
    }
    pub fn shift(&self) -> &bool {
        &self.shift
    }
    pub fn alt(&self) -> &bool {
        &self.alt
    }
    pub fn logo(&self) -> &bool {
        &self.logo
    }

    pub fn mods(self) -> ModifiersState {
        let mut ret = ModifiersState::empty();
        if *self.shift() {
            ret |= ModifiersState::SHIFT;
        }
        if *self.ctrl() {
            ret |= ModifiersState::CTRL;
        }
        if *self.alt() {
            ret |= ModifiersState::ALT;
        }
        if *self.logo() {
            ret |= ModifiersState::LOGO;
        }
        ret
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    /// OS-independent "key mapping code" which corresponds to OS-dependent
    /// scan code (i.e., physical location of key on keyboard).
    #[serde(with = "crate::serde_impl::KeyMappingCodeSerde")]
    Sc(KeyMappingCode),
    /// OS-independent "virtual key code" (i.e., semantic meaning of key on
    /// keyboard, taking into account the current layout).
    Vk(VirtualKeyCode),
}
impl Key {
    pub fn is_shift(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::ShiftLeft | Sc::ShiftRight) => true,
            Self::Vk(Vk::LShift | Vk::RShift) => true,
            _ => false,
        }
    }
    pub fn is_ctrl(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::ControlLeft | Sc::ControlRight) => true,
            Self::Vk(Vk::LControl | Vk::RControl) => true,
            _ => false,
        }
    }
    pub fn is_alt(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::AltLeft | Sc::AltRight) => true,
            Self::Vk(Vk::LAlt | Vk::RAlt) => true,
            _ => false,
        }
    }
    pub fn is_logo(self) -> bool {
        use KeyMappingCode as Sc;
        use VirtualKeyCode as Vk;
        match self {
            Self::Sc(Sc::MetaLeft | Sc::MetaRight) => true,
            Self::Vk(Vk::LWin | Vk::RWin) => true,
            _ => false,
        }
    }
    pub fn is_modifier(self) -> bool {
        self.is_shift() || self.is_ctrl() || self.is_alt() || self.is_logo()
    }

    pub fn modifier_bit(self) -> ModifiersState {
        match self {
            _ if self.is_shift() => ModifiersState::SHIFT,
            _ if self.is_ctrl() => ModifiersState::CTRL,
            _ if self.is_alt() => ModifiersState::ALT,
            _ if self.is_logo() => ModifiersState::LOGO,
            _ => ModifiersState::empty(),
        }
    }
}
