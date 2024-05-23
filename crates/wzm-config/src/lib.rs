use std::collections::HashSet;
use std::fs;

use serde::{Deserialize, Serialize};
use xkbcommon::xkb::Keysym;

use crate::keybinding::{Action, KeyBinding, Modifier};

pub mod action;
pub mod keybinding;

#[derive(Debug, Deserialize, Serialize)]
pub struct WzmConfig {
    pub gaps: u32,
    pub keybindings: Vec<KeyBinding>,
}

impl WzmConfig {
    pub fn get() -> anyhow::Result<WzmConfig> {
        let file = dirs::home_dir()
            .expect("$HOME should be set")
            .join(".config/wazemmes/config.ron");

        let file = fs::read_to_string(file)?;
        let config = ron::from_str(&file)?;
        Ok(config)
    }
}

impl Default for WzmConfig {
    fn default() -> Self {
        Self {
            gaps: 14,
            keybindings: vec![
                KeyBinding {
                    modifiers: HashSet::from([Modifier::Alt]),
                    key: Keysym::t,
                    action: Action::Run {
                        env: vec![],
                        command: "alacritty".to_string(),
                    },
                },
                KeyBinding {
                    modifiers: HashSet::from([Modifier::Alt]),
                    key: Keysym::g,
                    action: Action::Run {
                        env: vec![("WGPU_BACKEND".into(), "vulkan".into())],
                        command: "onagre".to_string(),
                    },
                },
            ],
        }
    }
}
