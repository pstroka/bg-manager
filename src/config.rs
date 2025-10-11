// SPDX-License-Identifier: GPL-3.0

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use cosmic_bg_config::context;
use cosmic_bg_config::state::State;
use cosmic_bg_config::Config as BgConfig;
use cosmic_bg_config::Source;

#[derive(Debug, Clone, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct Config {
    pub enabled: bool,
    pub dark: (Source, State),
    pub light: (Source, State),
}

impl Default for Config {
    fn default() -> Self {
        let bg_config = BgConfig::load(&context().unwrap()).unwrap();
        let default = bg_config.default_background.source;
        let state = State::state()
            .map(|context| match State::get_entry(&context) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default();
        Self {
            enabled: false,
            dark: (default.clone(), state.clone()),
            light: (default.clone(), state.clone()),
        }
    }
}
