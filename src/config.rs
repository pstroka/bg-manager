// SPDX-License-Identifier: GPL-3.0

use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry},
    Application,
};
use cosmic_bg_config::{Context, Entry};

use crate::app::AppModel;

#[derive(Default, Debug, Clone, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct Config {
    pub enabled: bool,
    pub dark: Vec<Entry>,
    pub light: Vec<Entry>,
}

impl Config {
    pub fn config() -> Result<cosmic_config::Config, cosmic_config::Error> {
        cosmic_config::Config::new(AppModel::APP_ID, Config::VERSION)
    }

    pub fn load(&mut self, is_dark: bool, context: &Context, handler: &cosmic_config::Config) {
        let mut config = cosmic_bg_config::Config::load(context).unwrap();
        let mut entries = Vec::with_capacity(config.backgrounds.len() + 1);
        entries.push(config.default_background);
        entries.append(&mut config.backgrounds);
        if is_dark {
            self.set_dark(handler, entries).unwrap();
        } else {
            self.set_light(handler, entries).unwrap();
        }
    }

    pub fn _set_entry(&mut self, is_dark: bool, entry: Entry, handler: &cosmic_config::Config) {
        if is_dark {
            match self.dark.iter().position(|e| e.output == entry.output) {
                Some(index) => self.dark[index] = entry,
                _ => self.dark.push(entry),
            }
            self.set_dark(handler, self.dark.clone()).unwrap();
        } else {
            match self.light.iter().position(|e| e.output == entry.output) {
                Some(index) => self.light[index] = entry,
                _ => self.light.push(entry),
            }
            self.set_light(handler, self.light.clone()).unwrap();
        }
    }

    pub fn update_bg(&self, is_dark: bool, context: &Context) {
        let mut config = cosmic_bg_config::Config::load(context).unwrap();
        let entries = if is_dark { &self.dark } else { &self.light };
        entries
            .iter()
            .for_each(|e| config.set_entry(context, e.clone()).unwrap());
    }
}

#[derive(Debug, Clone, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct BgConfig {
    pub all: Entry,
}

impl Default for BgConfig {
    fn default() -> Self {
        Self {
            all: Entry::fallback(),
        }
    }
}
