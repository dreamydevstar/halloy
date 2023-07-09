#![allow(clippy::large_enum_variant, clippy::too_many_arguments)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod buffer;
mod event;
mod font;
mod icon;
mod logger;
mod screen;
mod stream;
mod theme;
mod widget;

use std::env;

use data::config::{self, Config};
use data::server;
use iced::widget::container;
use iced::{executor, window, Application, Command, Length, Subscription};
use screen::{dashboard, help, welcome};

use self::event::{events, Event};
pub use self::theme::Theme;
use self::widget::Element;

pub fn main() -> iced::Result {
    let mut args = env::args();
    args.next();

    let version = args
        .next()
        .map(|s| s == "--version" || s == "-V")
        .unwrap_or_default();

    if version {
        println!("halloy {}", data::environment::formatted_version());

        return Ok(());
    }

    #[cfg(debug_assertions)]
    let is_debug = true;
    #[cfg(not(debug_assertions))]
    let is_debug = false;

    logger::setup(is_debug).expect("setup logging");
    log::info!(
        "halloy {} has started",
        data::environment::formatted_version()
    );

    // Create themes directory
    config::create_themes_dir();

    let config_load = Config::load();

    // DANGER ZONE - font must be set using config
    // before we do any iced related stuff w/ it
    font::set(config_load.as_ref().ok());

    if let Err(error) = Halloy::run(settings(config_load)) {
        log::error!("{}", error.to_string());
        Err(error)
    } else {
        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios", windows)))]
fn window_settings() -> iced::window::Settings {
    Default::default()
}

#[cfg(target_os = "macos")]
fn window_settings() -> iced::window::Settings {
    iced::window::Settings {
        platform_specific: iced::window::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        ..Default::default()
    }
}

#[cfg(target_os = "windows")]
fn window_settings() -> iced::window::Settings {
    use image::EncodableLayout;

    let img = image::load_from_memory_with_format(
        include_bytes!("../assets/logo.png"),
        image::ImageFormat::Png,
    );
    match img {
        Ok(img) => match img.as_rgba8() {
            Some(icon) => iced::window::Settings {
                icon: window::icon::from_rgba(
                    icon.as_bytes().to_vec(),
                    icon.width(),
                    icon.height(),
                )
                .ok(),
                ..Default::default()
            },
            None => Default::default(),
        },
        Err(_) => iced::window::Settings {
            ..Default::default()
        },
    }
}

fn settings(
    config_load: Result<Config, config::Error>,
) -> iced::Settings<Result<Config, config::Error>> {
    let default_text_size = config_load
        .as_ref()
        .ok()
        .and_then(|config| config.font.size)
        .map(f32::from)
        .unwrap_or(theme::TEXT_SIZE);

    iced::Settings {
        default_font: font::MONO.clone().into(),
        default_text_size,
        window: iced::window::Settings {
            ..window_settings()
        },
        exit_on_close_request: false,
        flags: config_load,
        id: None,
        antialiasing: false,
    }
}

struct Halloy {
    screen: Screen,
    theme: Theme,
    config: Config,
    clients: data::client::Map,
    servers: server::Map,
}

impl Halloy {
    pub fn load_from_state(
        config_load: Result<Config, config::Error>,
    ) -> (Halloy, Command<Message>) {
        let load_dashboard = |config| match data::Dashboard::load() {
            Ok(dashboard) => screen::Dashboard::restore(dashboard),
            Err(error) => {
                // TODO: Show this in error screen too? Maybe w/ option to report bug on GH
                // and reset settings to continue loading?
                log::warn!("failed to load dashboard: {error}");

                screen::Dashboard::empty(config)
            }
        };

        let (screen, config, command) = match config_load {
            Ok(config) => {
                let (screen, command) = load_dashboard(&config);

                (
                    Screen::Dashboard(screen),
                    config,
                    command.map(Message::Dashboard),
                )
            }
            Err(error) => match &error {
                config::Error::Parse(_) => (
                    Screen::Help(screen::Help::new(error)),
                    Config::default(),
                    Command::none(),
                ),
                _ => (
                    Screen::Welcome(screen::Welcome::new()),
                    Config::default(),
                    Command::none(),
                ),
            },
        };

        (
            Halloy {
                screen,
                theme: Theme::new_from_palette(config.palette),
                clients: Default::default(),
                servers: config.servers.clone(),
                config,
            },
            command,
        )
    }
}

pub enum Screen {
    Dashboard(screen::Dashboard),
    Help(screen::Help),
    Welcome(screen::Welcome),
}

#[derive(Debug)]
pub enum Message {
    Dashboard(dashboard::Message),
    Stream(stream::Update),
    Help(help::Message),
    Welcome(welcome::Message),
    Event(Event),
    FontsLoaded(Result<(), iced::font::Error>),
}

impl Application for Halloy {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = theme::Theme;
    type Flags = Result<Config, config::Error>;

    fn new(config_load: Self::Flags) -> (Halloy, Command<Self::Message>) {
        let (halloy, command) = Halloy::load_from_state(config_load);

        (
            halloy,
            Command::batch(vec![font::load().map(Message::FontsLoaded), command]),
        )
    }

    fn title(&self) -> String {
        String::from("Halloy")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Dashboard(message) => {
                let Screen::Dashboard(dashboard) = &mut self.screen else {
                    return Command::none()
                };

                let command =
                    dashboard.update(message, &mut self.clients, &mut self.servers, &self.config);
                // Retrack after dashboard state changes
                let track = dashboard.track();

                Command::batch(vec![
                    command.map(Message::Dashboard),
                    track.map(Message::Dashboard),
                ])
            }
            Message::Help(message) => {
                let Screen::Help(help) = &mut self.screen else {
                    return Command::none();
                };

                if let Some(event) = help.update(message) {
                    match event {
                        help::Event::RefreshConfiguration => {
                            let (halloy, command) = Halloy::load_from_state(Config::load());
                            *self = halloy;

                            return command;
                        }
                    }
                }

                Command::none()
            }
            Message::Welcome(message) => {
                let Screen::Welcome(welcome) = &mut self.screen else {
                    return Command::none();
                };

                if let Some(event) = welcome.update(message) {
                    match event {
                        welcome::Event::RefreshConfiguration => {
                            let (halloy, command) = Halloy::load_from_state(Config::load());
                            *self = halloy;

                            return command;
                        }
                    }
                }

                Command::none()
            }
            Message::Stream(update) => match update {
                stream::Update::Disconnected {
                    server,
                    is_initial,
                    error,
                } => {
                    self.clients.disconnected(server.clone());

                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Command::none()
                    };

                    if is_initial {
                        // Intial is sent when first trying to connect
                        dashboard.broadcast_connecting(&server);
                    } else {
                        dashboard.broadcast_disconnected(&server, error);
                    }

                    Command::none()
                }
                stream::Update::Connected {
                    server,
                    connection,
                    is_initial,
                } => {
                    self.clients.ready(server.clone(), connection);

                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Command::none()
                    };

                    if is_initial {
                        dashboard.broadcast_connected(&server);
                    } else {
                        dashboard.broadcast_reconnected(&server);
                    }

                    Command::none()
                }
                stream::Update::ConnectionFailed { server, error } => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                            return Command::none()
                        };

                    dashboard.broadcast_connection_failed(&server, error);

                    Command::none()
                }
                stream::Update::MessagesReceived(server, messages) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Command::none()
                    };

                    messages.into_iter().for_each(|encoded| {
                        if let Some(event) = self.clients.receive(&server, encoded) {
                            match event {
                                data::client::Event::Single(message) => {
                                    dashboard.record_message(&server, message);
                                }
                                data::client::Event::Whois(message) => {
                                    dashboard.record_whois(&server, message);
                                }
                                data::client::Event::Brodcast(brodcast) => match brodcast {
                                    data::client::Brodcast::Quit { user, comment } => {
                                        let user_channels = self
                                            .clients
                                            .get_user_channels(&server, user.nickname());

                                        dashboard.broadcast_quit(
                                            &server,
                                            user,
                                            comment,
                                            user_channels,
                                        );
                                    }
                                    data::client::Brodcast::Nickname {
                                        old_user,
                                        new_nick,
                                        ourself,
                                    } => {
                                        let old_nick = old_user.nickname();
                                        let user_channels =
                                            self.clients.get_user_channels(&server, old_nick);

                                        dashboard.broadcast_nickname(
                                            &server,
                                            old_nick.to_owned(),
                                            new_nick,
                                            ourself,
                                            user_channels,
                                        );
                                    }
                                },
                            }
                        }
                    });

                    // Must be called after receiving message batches to ensure
                    // user & channel lists are in sync
                    self.clients.sync(&server);

                    Command::none()
                }
            },
            Message::FontsLoaded(Ok(())) => Command::none(),
            Message::FontsLoaded(Err(error)) => {
                log::error!("fonts failed to load: {error:?}");
                Command::none()
            }
            Message::Event(event) => {
                if let Screen::Dashboard(dashboard) = &mut self.screen {
                    dashboard.handle_event(event).map(Message::Dashboard)
                } else if let event::Event::CloseRequested = event {
                    window::close()
                } else {
                    Command::none()
                }
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let content = match &self.screen {
            Screen::Dashboard(dashboard) => dashboard
                .view(&self.clients, &self.config)
                .map(Message::Dashboard),
            Screen::Help(help) => help.view().map(Message::Help),
            Screen::Welcome(welcome) => welcome.view().map(Message::Welcome),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Primary)
            .into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        let screen_subscription = match &self.screen {
            Screen::Dashboard(dashboard) => dashboard.subscription().map(Message::Dashboard),
            Screen::Help(_) => Subscription::none(),
            Screen::Welcome(_) => Subscription::none(),
        };

        let streams =
            Subscription::batch(self.servers.entries().map(stream::run)).map(Message::Stream);

        Subscription::batch(vec![
            streams,
            events().map(Message::Event),
            screen_subscription,
        ])
    }
}
