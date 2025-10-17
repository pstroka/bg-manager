// SPDX-License-Identifier: GPL-3.0

use std::path::PathBuf;

use crate::config::{Bg, Config};
use crate::fl;
use crate::unique::UniqueIterator;
use cosmic::applet::menu_button;
use cosmic::applet::token::subscription::{
    activation_token_subscription, TokenRequest, TokenUpdate,
};
use cosmic::cctk::sctk::reexports::calloop;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::cosmic_theme::palette::{Darken, Lighten, Mix, Srgb};
use cosmic::cosmic_theme::{Theme, ThemeBuilder, ThemeMode, THEME_MODE_ID};
use cosmic::iced::{color, Color, Length};
use cosmic::iced::{window::Id, Subscription};
use cosmic::iced_widget::row;
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::iced_winit::graphics::image::image_rs::Pixel;
use cosmic::prelude::*;
use cosmic::widget::color_picker::color_button;
use cosmic::widget::settings::item;
use cosmic::widget::{self, text, toggler};
use cosmic_bg_config::{context, Source};
use cosmic_settings_wallpaper::load_image_with_thumbnail;

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config_handler: Option<cosmic_config::Config>,
    config: Config,
    token_tx: Option<calloop::channel::Sender<TokenRequest>>,
    colors: Vec<Color>,
}

impl AppModel {
    fn update_bg(&mut self, is_dark: bool) {
        let context = context().unwrap();
        let mut config = cosmic_bg_config::Config::load(&context).unwrap();
        if self.config.enabled {
            let entries = if is_dark {
                &self.config.dark
            } else {
                &self.config.light
            };
            entries
                .iter()
                .for_each(|e| config.set_entry(&context, e.clone()).unwrap());
        }

        let backgrounds = if config.same_on_all {
            vec![config.default_background]
        } else {
            config.backgrounds
        };

        self.colors = backgrounds
            .iter()
            .flat_map(|e| match e.source.clone() {
                Source::Path(path_buf) => dominant_colors(path_buf),
                Source::Color(color) => match color {
                    cosmic_bg_config::Color::Single(color) => {
                        let color = Srgb::from(color);
                        vec![
                            color.lighten(0.66).into(),
                            color.lighten(0.33).into(),
                            color.darken(0.33).into(),
                            color.darken(0.66).into(),
                        ]
                    }
                    cosmic_bg_config::Color::Gradient(gradient) => {
                        let mut colors = gradient
                            .colors
                            .iter()
                            .map(|&color| color.into())
                            .collect::<Vec<_>>();
                        if let Some(color) = gradient
                            .colors
                            .iter()
                            .map(|&color| Srgb::from(color))
                            .reduce(|l, r| l.mix(r, 0.5))
                        {
                            colors.push(color.into());
                        }
                        colors
                    }
                },
            })
            .collect_unique();
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    ConfigUpdate(Config),
    BgUpdate(Bg),
    ThemeModeUpdate(ThemeMode),
    Toggle(bool),
    OpenSettings(bool),
    ChangeAccentColor(Color),
    Token(TokenUpdate),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.github.pstroka.BackgroundManager";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let app = AppModel {
            core,
            config_handler: Config::config().ok(),
            config: Config::config()
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    }
                })
                .unwrap_or_default(),
            ..Default::default()
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("com.github.pstroka.BackgroundManager-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let content_list = widget::list_column()
            // .list_item_padding([8, 0, 8, 0])
            .padding([8, 0, 8, 0])
            .add(item(
                fl!("switcher-text"),
                toggler(self.config.enabled).on_toggle(Message::Toggle),
            ))
            .add(
                menu_button(text(fl!("settings-dark")))
                    .padding([8, 0, 8, 0])
                    .on_press(Message::OpenSettings(true)),
            )
            .add(
                menu_button(text(fl!("settings-light")))
                    .padding([8, 0, 8, 0])
                    .on_press(Message::OpenSettings(false)),
            )
            .add(item(
                fl!("accent-color"),
                row(self.colors.iter().map(|color| {
                    color_button(
                        Some(Message::ChangeAccentColor(*color)),
                        Some(*color),
                        Length::Fill,
                    )
                    .into()
                }))
                .spacing(8),
            ));

        self.core.applet.popup_container(content_list).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            activation_token_subscription(0).map(Message::Token),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::ConfigUpdate(update.config)
                }),
            self.core()
                .watch_config::<ThemeMode>(THEME_MODE_ID)
                .map(|update| Message::ThemeModeUpdate(update.config)),
            self.core()
                .watch_config::<Bg>(cosmic_bg_config::NAME)
                .map(|update| Message::BgUpdate(update.config)),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::ConfigUpdate(config) => {
                self.config = config;
                self.update_bg(self.core.system_theme_mode().is_dark);
            }
            Message::Toggle(toggled) => {
                self.config
                    .set_enabled(self.config_handler.as_ref().unwrap(), toggled)
                    .unwrap();
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::ThemeModeUpdate(theme_mode) => {
                self.update_bg(theme_mode.is_dark);
            }
            Message::BgUpdate(config) => {
                if config.entries.is_empty() {
                    return Task::none();
                }
                let is_dark = self.core.system_theme_mode().is_dark;
                if is_dark && config.entries != self.config.dark {
                    self.config
                        .set_dark(self.config_handler.as_ref().unwrap(), config.entries)
                        .unwrap();
                } else if !is_dark && config.entries != self.config.light {
                    self.config
                        .set_light(self.config_handler.as_ref().unwrap(), config.entries)
                        .unwrap();
                }
            }
            Message::OpenSettings(is_dark) => {
                self.core
                    .system_theme_mode()
                    .set_is_dark(&ThemeMode::config().unwrap(), is_dark)
                    .unwrap();
                if let Some(tx) = self.token_tx.as_ref() {
                    let _ = tx.send(TokenRequest {
                        app_id: Self::APP_ID.to_string(),
                        exec: "cosmic-settings wallpaper".to_string(),
                    });
                }
            }
            Message::Token(u) => match u {
                TokenUpdate::Init(tx) => {
                    self.token_tx = Some(tx);
                }
                TokenUpdate::Finished => {
                    self.token_tx = None;
                }
                TokenUpdate::ActivationToken { token, .. } => {
                    let mut cmd = std::process::Command::new("cosmic-settings");
                    cmd.arg("wallpaper");
                    if let Some(token) = token {
                        cmd.env("XDG_ACTIVATION_TOKEN", &token);
                        cmd.env("DESKTOP_STARTUP_ID", &token);
                    }
                    tokio::spawn(cosmic::process::spawn(cmd));
                }
            },
            Message::ChangeAccentColor(color) => {
                let (builder_config, theme_config) = if self.core.system_theme_mode().is_dark {
                    (
                        ThemeBuilder::dark_config().unwrap(),
                        Theme::dark_config().unwrap(),
                    )
                } else {
                    (
                        ThemeBuilder::light_config().unwrap(),
                        Theme::light_config().unwrap(),
                    )
                };
                let mut builder = ThemeBuilder::get_entry(&builder_config)
                    .unwrap()
                    .accent(color.into());
                builder.window_hint = Some(color.into());
                builder.write_entry(&builder_config).unwrap();
                let theme = builder.build();
                theme.write_entry(&theme_config).unwrap();
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}

fn dominant_colors(path: PathBuf) -> Vec<Color> {
    if let Some((_, thumbnail, _)) = load_image_with_thumbnail(path) {
        let pixels = thumbnail
            .pixels()
            .flat_map(|p| p.to_rgb().0)
            .collect::<Vec<_>>();
        let a = dominant_color::get_colors_with_config(
            &pixels,
            false,
            (thumbnail.width() * thumbnail.height()).into(),
            0.001,
        );
        a.chunks_exact(3)
            .map(|s| color!(s[0], s[1], s[2]))
            .collect()
    } else {
        vec![]
    }
}
