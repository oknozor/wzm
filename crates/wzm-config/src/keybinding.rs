use crate::action::{Direction, KeyAction};
use log::warn;
use serde::{Deserialize, Serialize, Serializer};
use smithay::input::keyboard::ModifiersState;
use std::collections::HashSet;
use std::hash::Hash;
use xkbcommon::xkb;
use xkbcommon::xkb::Keysym;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct KeyBinding {
    pub modifiers: HashSet<Modifier>,
    #[serde(serialize_with = "serialize_key")]
    #[serde(deserialize_with = "deserialize_key")]
    pub key: Keysym,
    pub action: Action,
    #[serde(default)]
    pub mode: Mode,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum Mode {
    #[default]
    Normal,
    Resize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ResizeType {
    Shrink,
    Grow,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ResizeDirection {
    Height,
    Width,
}

impl KeyBinding {
    pub fn match_action(
        &self,
        modifiers: ModifiersState,
        key: Keysym,
        mode: Mode,
    ) -> Option<Action> {
        let state: ModifiersState = self.into();

        if match_modifier(state, modifiers) && key == self.key && mode == self.mode {
            Some(self.action.clone())
        } else {
            None
        }
    }
}

fn match_modifier(modifier: ModifiersState, other: ModifiersState) -> bool {
    (
        modifier.ctrl,
        modifier.alt,
        modifier.shift,
        modifier.logo,
        modifier.caps_lock,
        modifier.num_lock,
    ) == (
        other.ctrl,
        other.alt,
        other.shift,
        other.logo,
        other.caps_lock,
        other.num_lock,
    )
}

impl From<&KeyBinding> for ModifiersState {
    fn from(val: &KeyBinding) -> Self {
        ModifiersState {
            ctrl: val.modifiers.contains(&Modifier::Ctrl),
            alt: val.modifiers.contains(&Modifier::Alt),
            shift: val.modifiers.contains(&Modifier::Shift),
            caps_lock: val.modifiers.contains(&Modifier::CapsLock),
            logo: val.modifiers.contains(&Modifier::Logo),
            num_lock: val.modifiers.contains(&Modifier::NumLock),
            iso_level3_shift: false,
            serialized: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Action {
    MoveWindowLeft,
    MoveWindowRight,
    MoveWindowDown,
    MoveWindowUp,
    MoveContainerLeft,
    MoveContainerRight,
    MoveContainerDown,
    MoveContainerUp,
    MoveFocusLeft,
    ToggleFullScreenWindow,
    ToggleFullScreenContainer,
    MoveFocusRight,
    MoveFocusDown,
    MoveFocusUp,
    MoveToWorkspace(u8),
    LayoutVertical,
    LayoutHorizontal,
    ToggleSwitchLayout,
    ToggleFloating,
    ToggleResize,
    Resize(ResizeDirection, ResizeType, u32),
    Run {
        env: Vec<(String, String)>,
        command: String,
    },
    CloseWindow,
    Quit,
}

impl From<Action> for KeyAction {
    fn from(val: Action) -> Self {
        match val {
            Action::MoveWindowLeft => KeyAction::MoveWindow(Direction::Left),
            Action::MoveWindowRight => KeyAction::MoveWindow(Direction::Right),
            Action::MoveWindowDown => KeyAction::MoveWindow(Direction::Down),
            Action::MoveWindowUp => KeyAction::MoveWindow(Direction::Up),
            Action::MoveContainerLeft => KeyAction::MoveContainer(Direction::Left),
            Action::MoveContainerRight => KeyAction::MoveContainer(Direction::Right),
            Action::MoveContainerDown => KeyAction::MoveContainer(Direction::Down),
            Action::MoveContainerUp => KeyAction::MoveContainer(Direction::Up),
            Action::MoveFocusLeft => KeyAction::MoveFocus(Direction::Left),
            Action::MoveFocusRight => KeyAction::MoveFocus(Direction::Right),
            Action::MoveFocusDown => KeyAction::MoveFocus(Direction::Down),
            Action::MoveFocusUp => KeyAction::MoveFocus(Direction::Up),
            Action::MoveToWorkspace(num) => KeyAction::MoveToWorkspace(num),
            Action::LayoutVertical => KeyAction::LayoutVertical,
            Action::LayoutHorizontal => KeyAction::LayoutHorizontal,
            Action::ToggleFloating => KeyAction::ToggleFloating,
            Action::Run { command, env } => KeyAction::Run(command, env),
            Action::CloseWindow => KeyAction::CloseWindow,
            Action::Quit => KeyAction::Quit,
            Action::ToggleFullScreenWindow => KeyAction::ToggleFullScreenWindow,
            Action::ToggleFullScreenContainer => KeyAction::ToggleFullScreenContainer,
            Action::ToggleResize => KeyAction::ToggleResize,
            Action::Resize(kind, direction, ammount) => KeyAction::Resize(direction, kind, ammount),
            Action::ToggleSwitchLayout => KeyAction::ToggleSwitchLayout,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct EnvVar {
    key: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Logo,
    CapsLock,
    NumLock,
}

fn serialize_key<S>(key: &Keysym, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let name = xkb::keysym_get_name(*key);
    serializer.serialize_str(&name)
}

#[allow(non_snake_case)]
fn deserialize_key<'de, D>(deserializer: D) -> Result<Keysym, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Error, Unexpected};

    let name = String::deserialize(deserializer)?;
    match xkb::keysym_from_name(&name, xkb::keysyms::KEY_NoSymbol) {
        Keysym::VoidSymbol => match xkb::keysym_from_name(&name, xkb::KEYSYM_CASE_INSENSITIVE) {
            Keysym::VoidSymbol => Err(<D::Error as Error>::invalid_value(
                Unexpected::Str(&name),
                &"xkb keysym",
            )),
            key => {
                warn!(
                    "Key-Binding '{}' only matched case insensitive for {:?}",
                    name,
                    xkb::keysym_get_name(key)
                );
                Ok(key)
            }
        },
        key => Ok(key),
    }
}

#[cfg(test)]
mod test {
    use crate::keybinding::{Action, KeyBinding, Modifier};
    use crate::WzmConfig;
    use indoc::indoc;
    use smithay::input::keyboard::Keysym;
    use speculoos::prelude::*;
    use std::collections::HashSet;

    #[test]
    fn should_deserialize_keybindings() {
        let keys = indoc! {
            r#"KeyBinding(
                modifiers: [Alt, Ctrl],
                key: "a",
                action: Run(
                    env: [],
                    command: "alacritty",
                )
            )"#
        };

        let binding = ron::from_str::<KeyBinding>(keys);

        assert_that!(binding).is_ok();
        let binding = binding.unwrap();

        assert_that!(&binding.modifiers).contains_all_of(&[&Modifier::Ctrl, &Modifier::Alt]);

        assert_that!(binding.key).is_equal_to(Keysym::a);

        assert_that!(binding.action).is_equal_to(Action::Run {
            env: vec![],
            command: "alacritty".to_string(),
        });
    }

    #[test]
    pub fn test() {
        let binding = vec![
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::t,
                action: Action::Run {
                    env: vec![],
                    command: "alacritty".to_string(),
                },
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::g,
                action: Action::Run {
                    env: vec![("WGPU_BACKEND".into(), "vulkan".into())],
                    command: "onagre".to_string(),
                },
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::a,
                action: Action::CloseWindow,
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::v,
                action: Action::LayoutVertical,
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::d,
                action: Action::LayoutHorizontal,
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Ctrl, Modifier::Shift]),
                key: Keysym::space,
                action: Action::ToggleFloating,
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::k,
                action: Action::MoveFocusUp,
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::h,
                action: Action::MoveFocusLeft,
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::l,
                action: Action::MoveFocusRight,
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::j,
                action: Action::MoveFocusDown,
                mode: Default::default(),
            },
            KeyBinding {
                modifiers: HashSet::from([Modifier::Alt]),
                key: Keysym::k,
                action: Action::MoveFocusUp,
                mode: Default::default(),
            },
        ];

        let config = WzmConfig {
            gaps: 14,
            keybindings: binding,
        };

        let string = ron::to_string(&config).unwrap();
        println!("{}", string);
    }
}
