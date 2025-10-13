// SPDX-License-Identifier: GPL-3.0

use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry},
    Application,
};
use cosmic_bg_config::{context, Context, Entry};

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

    pub fn update_bg(&self, is_dark: bool, context: &Context) {
        let mut config = cosmic_bg_config::Config::load(context).unwrap();
        let entries = if is_dark { &self.dark } else { &self.light };
        entries
            .iter()
            .for_each(|e| config.set_entry(context, e.clone()).unwrap());
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Bg {
    pub entries: Vec<Entry>,
}

impl CosmicConfigEntry for Bg {
    const VERSION: u64 = 1;

    fn write_entry(&self, _config: &cosmic_config::Config) -> Result<(), cosmic_config::Error> {
        Ok(())
    }

    fn get_entry(
        _config: &cosmic_config::Config,
    ) -> Result<Self, (Vec<cosmic_config::Error>, Self)> {
        let context = context().unwrap();
        let mut config = cosmic_bg_config::Config::load(&context).unwrap();
        let mut entries = Vec::with_capacity(config.backgrounds.len() + 1);
        entries.push(config.default_background);
        entries.append(&mut config.backgrounds);
        Ok(Self { entries })
    }

    fn update_keys<T: AsRef<str>>(
        &mut self,
        config: &cosmic_config::Config,
        changed_keys: &[T],
    ) -> (Vec<cosmic_config::Error>, Vec<&'static str>) {
        if changed_keys
            .iter()
            .map(|k| k.as_ref())
            .any(|k| k == "all" || k.starts_with("output"))
        {
            *self = Bg::get_entry(config).unwrap();
            (vec![], vec![""])
        } else {
            (vec![], vec![])
        }
    }
}
