// SPDX-License-Identifier: GPL-3.0

use crate::config::{BgConfig, Config};
use crate::fl;
use cosmic::applet::token::subscription::{
    activation_token_subscription, TokenRequest, TokenUpdate,
};
use cosmic::cctk::sctk::reexports::calloop;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::cosmic_theme::{ThemeMode, THEME_MODE_ID};
use cosmic::iced::{window::Id, Subscription};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic_bg_config::context;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
#[derive(Default)]
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The popup id.
    popup: Option<Id>,
    /// Configuration data that persists between application runs.
    config_handler: Option<cosmic_config::Config>,
    config: Config,
    token_tx: Option<calloop::channel::Sender<TokenRequest>>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    UpdateConfig(Config),
    UpdateBgConfig(BgConfig),
    UpdateThemeMode(ThemeMode),
    Toggle(bool),
    OpenSettings(bool),
    Token(TokenUpdate),
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.github.pstroka.BackgroundManager";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Construct the app model with the runtime's core.
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

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("display-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let content_list = widget::list_column()
            .padding([8, 0, 8, 0])
            .add(widget::settings::item(
                fl!("switcher-text"),
                widget::toggler(self.config.enabled).on_toggle(Message::Toggle),
            ))
            .add(
                cosmic::applet::menu_button(widget::text(fl!("settings-dark")))
                    .on_press(Message::OpenSettings(true)),
            )
            .add(
                cosmic::applet::menu_button(widget::text(fl!("settings-light")))
                    .on_press(Message::OpenSettings(false)),
            );

        self.core.applet.popup_container(content_list).into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
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

                    Message::UpdateConfig(update.config)
                }),
            self.core()
                .watch_config::<ThemeMode>(THEME_MODE_ID)
                .map(|update| Message::UpdateThemeMode(update.config)),
            // TODO: watch all outputs
            self.core()
                .watch_config::<BgConfig>(cosmic_bg_config::NAME)
                .map(|update| Message::UpdateBgConfig(update.config)),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdateConfig(config) => {
                self.config = config;
                return self.update(Message::UpdateThemeMode(self.core.system_theme_mode()));
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
            Message::UpdateThemeMode(theme_mode) => {
                if self.config.enabled {
                    self.config
                        .update_bg(theme_mode.is_dark, &context().unwrap());
                }
            }
            Message::UpdateBgConfig(_config) => {
                // println!("{config:?}");
                // self.config.set_entry(
                //     self.core.system_theme_mode().is_dark,
                //     config.all,
                //     self.config_handler.as_ref().unwrap(),
                // );
                // TODO: don't trigger on mode update and don't load the while config
                self.config.load(
                    self.core.system_theme_mode().is_dark,
                    &context().unwrap(),
                    self.config_handler.as_ref().unwrap(),
                );
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
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}
