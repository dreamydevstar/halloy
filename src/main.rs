#![allow(clippy::large_enum_variant, clippy::too_many_arguments)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod buffer;
mod event;
mod font;
mod icon;
mod logger;
mod notification;
mod screen;
mod stream;
mod theme;
mod widget;
mod window;

use std::env;
use std::time::{Duration, Instant};

use data::config::{self, Config};
use data::version::Version;
use data::{environment, file_transfer, server, version, User};
use iced::widget::container;
use iced::{executor, Application, Command, Length, Subscription};
use screen::{dashboard, help, migration, welcome};

use self::event::{events, Event};
use self::theme::Theme;
use self::widget::Element;

pub fn main() -> iced::Result {
    let mut args = env::args();
    args.next();

    let version = args
        .next()
        .map(|s| s == "--version" || s == "-V")
        .unwrap_or_default();

    if version {
        println!("halloy {}", environment::formatted_version());

        return Ok(());
    }

    #[cfg(debug_assertions)]
    let is_debug = true;
    #[cfg(not(debug_assertions))]
    let is_debug = false;

    // Prepare notifications.
    notification::prepare();

    logger::setup(is_debug).expect("setup logging");
    log::info!("halloy {} has started", environment::formatted_version());
    log::info!("config dir: {:?}", environment::config_dir());
    log::info!("data dir: {:?}", environment::data_dir());

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
        default_text_size: default_text_size.into(),
        window: window::Settings {
            exit_on_close_request: false,
            ..window::settings()
        },
        flags: config_load,
        id: None,
        antialiasing: false,
        fonts: font::load(),
    }
}

struct Halloy {
    version: Version,
    screen: Screen,
    theme: Theme,
    config: Config,
    clients: data::client::Map,
    servers: server::Map,
    file_transfers: file_transfer::Manager,
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
                _ => {
                    // If we have a YAML file, but end up in this arm
                    // it means the user tried to load Halloy with a YAML configuration, but it expected TOML.
                    if config::has_yaml_config() {
                        (
                            Screen::Migration(screen::Migration::new()),
                            Config::default(),
                            Command::none(),
                        )
                    } else {
                        // Otherwise, show regular welcome screen for new users.
                        (
                            Screen::Welcome(screen::Welcome::new()),
                            Config::default(),
                            Command::none(),
                        )
                    }
                }
            },
        };

        (
            Halloy {
                version: Version::new(),
                screen,
                theme: config.themes.default.clone().into(),
                clients: Default::default(),
                servers: config.servers.clone(),
                config,
                file_transfers: file_transfer::Manager::default(),
            },
            command,
        )
    }
}

pub enum Screen {
    Dashboard(screen::Dashboard),
    Help(screen::Help),
    Welcome(screen::Welcome),
    Migration(screen::Migration),
}

#[derive(Debug)]
pub enum Message {
    Dashboard(dashboard::Message),
    Stream(stream::Update),
    Help(help::Message),
    Welcome(welcome::Message),
    Migration(migration::Message),
    Event(Event),
    Tick(Instant),
    Version(Option<String>),
    FileTransfer(file_transfer::task::Update),
}

impl Application for Halloy {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = Result<Config, config::Error>;

    fn new(config_load: Self::Flags) -> (Halloy, Command<Self::Message>) {
        let (halloy, command) = Halloy::load_from_state(config_load);
        let latest_remote_version =
            Command::perform(version::latest_remote_version(), Message::Version);

        let command = Command::batch(vec![command, latest_remote_version]);

        (halloy, command)
    }

    fn title(&self) -> String {
        String::from("Halloy")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Dashboard(message) => {
                let Screen::Dashboard(dashboard) = &mut self.screen else {
                    return Command::none();
                };

                let command = dashboard.update(
                    message,
                    &mut self.clients,
                    &mut self.servers,
                    &mut self.theme,
                    &self.version,
                    &self.config,
                );
                // Retrack after dashboard state changes
                let track = dashboard.track();

                Command::batch(vec![
                    command.map(Message::Dashboard),
                    track.map(Message::Dashboard),
                ])
            }
            Message::Version(remote) => {
                // Set latest known remote version
                self.version.remote = remote;

                Command::none()
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
            Message::Migration(message) => {
                let Screen::Migration(migration) = &mut self.screen else {
                    return Command::none();
                };

                if let Some(event) = migration.update(message) {
                    match event {
                        migration::Event::RefreshConfiguration => {
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
                    sent_time,
                } => {
                    self.clients.disconnected(server.clone());

                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Command::none();
                    };

                    if is_initial {
                        // Intial is sent when first trying to connect
                        dashboard.broadcast_connecting(&server, &self.config, sent_time);
                    } else {
                        let notification = &self.config.notifications.disconnected;

                        if notification.enabled {
                            notification::show("Disconnected", &server, notification.sound());
                        };

                        dashboard.broadcast_disconnected(&server, error, &self.config, sent_time);
                    }

                    Command::none()
                }
                stream::Update::Connected {
                    server,
                    client: connection,
                    is_initial,
                    sent_time,
                } => {
                    self.clients.ready(server.clone(), connection);

                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Command::none();
                    };

                    if is_initial {
                        let notification = &self.config.notifications.connected;

                        if notification.enabled {
                            notification::show("Connected", &server, notification.sound());
                        }

                        dashboard.broadcast_connected(&server, &self.config, sent_time);
                    } else {
                        let notification = &self.config.notifications.reconnected;

                        if notification.enabled {
                            notification::show("Reconnected", &server, notification.sound());
                        }

                        dashboard.broadcast_reconnected(&server, &self.config, sent_time);
                    }

                    Command::none()
                }
                stream::Update::ConnectionFailed {
                    server,
                    error,
                    sent_time,
                } => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Command::none();
                    };

                    dashboard.broadcast_connection_failed(&server, error, &self.config, sent_time);

                    Command::none()
                }
                stream::Update::MessagesReceived(server, messages) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Command::none();
                    };

                    let commands = messages
                        .into_iter()
                        .flat_map(|message| {
                            let mut commands = vec![];

                            for event in self.clients.receive(&server, message) {
                                // Resolve a user using client state which stores attributes
                                let resolve_user_attributes = |user: &User, channel: &str| {
                                    self.clients
                                        .resolve_user_attributes(&server, channel, user)
                                        .cloned()
                                };

                                match event {
                                    data::client::Event::Single(encoded, our_nick) => {
                                        if let Some(message) = data::Message::received(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                        ) {
                                            dashboard.record_message(&server, message);
                                        }
                                    }
                                    data::client::Event::WithTarget(encoded, our_nick, target) => {
                                        if let Some(message) = data::Message::received(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                        ) {
                                            dashboard.record_message(
                                                &server,
                                                message.with_target(target),
                                            );
                                        }
                                    }
                                    data::client::Event::Broadcast(broadcast) => match broadcast {
                                        data::client::Broadcast::Quit {
                                            user,
                                            comment,
                                            channels,
                                            sent_time,
                                        } => {
                                            dashboard.broadcast_quit(
                                                &server,
                                                user,
                                                comment,
                                                channels,
                                                &self.config,
                                                sent_time,
                                            );
                                        }
                                        data::client::Broadcast::Nickname {
                                            old_user,
                                            new_nick,
                                            ourself,
                                            channels,
                                            sent_time,
                                        } => {
                                            let old_nick = old_user.nickname();

                                            dashboard.broadcast_nickname(
                                                &server,
                                                old_nick.to_owned(),
                                                new_nick,
                                                ourself,
                                                channels,
                                                &self.config,
                                                sent_time,
                                            );
                                        }
                                        data::client::Broadcast::Invite {
                                            inviter,
                                            channel,
                                            user_channels,
                                            sent_time,
                                        } => {
                                            let inviter = inviter.nickname();

                                            dashboard.broadcast_invite(
                                                &server,
                                                inviter.to_owned(),
                                                channel,
                                                user_channels,
                                                &self.config,
                                                sent_time,
                                            );
                                        }
                                    },
                                    data::client::Event::Notification(
                                        encoded,
                                        our_nick,
                                        notification,
                                    ) => {
                                        if let Some(message) = data::Message::received(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                        ) {
                                            dashboard.record_message(&server, message);
                                        }

                                        match notification {
                                            data::client::Notification::Highlight(
                                                user,
                                                channel,
                                            ) => {
                                                let notification =
                                                    &self.config.notifications.highlight;
                                                if notification.enabled {
                                                    notification::show(
                                                        "Highlight",
                                                        format!(
                                                            "{} highlighted you in {}",
                                                            user.nickname(),
                                                            channel
                                                        ),
                                                        notification.sound(),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    data::client::Event::FileTransferRequest(request) => {
                                        if let Some(event) = self.file_transfers.receive(request) {
                                            match event {
                                                file_transfer::manager::Event::RunTask(task) => {
                                                    commands.push(Command::run(
                                                        task,
                                                        Message::FileTransfer,
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            commands
                        })
                        .collect::<Vec<_>>();

                    // Must be called after receiving message batches to ensure
                    // user & channel lists are in sync
                    self.clients.sync(&server);

                    Command::batch(commands)
                }
            },
            Message::Event(event) => {
                if let Screen::Dashboard(dashboard) = &mut self.screen {
                    dashboard
                        .handle_event(
                            event,
                            &self.clients,
                            &self.version,
                            &self.config,
                            &mut self.theme,
                        )
                        .map(Message::Dashboard)
                } else if let event::Event::CloseRequested = event {
                    window::close(window::Id::MAIN)
                } else {
                    Command::none()
                }
            }
            Message::Tick(now) => {
                self.clients.tick(now);

                if let Screen::Dashboard(dashboard) = &mut self.screen {
                    dashboard.tick(now).map(Message::Dashboard)
                } else {
                    Command::none()
                }
            }
            Message::FileTransfer(update) => {
                self.file_transfers.update(update);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let content = match &self.screen {
            Screen::Dashboard(dashboard) => dashboard
                .view(
                    &self.clients,
                    &self.file_transfers,
                    &self.version,
                    &self.config,
                )
                .map(Message::Dashboard),
            Screen::Help(help) => help.view().map(Message::Help),
            Screen::Welcome(welcome) => welcome.view().map(Message::Welcome),
            Screen::Migration(migration) => migration.view().map(Message::Migration),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::container::primary)
            .into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn scale_factor(&self) -> f64 {
        self.config.scale_factor.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(Duration::from_secs(1)).map(Message::Tick);

        let streams =
            Subscription::batch(self.servers.entries().map(stream::run)).map(Message::Stream);

        Subscription::batch(vec![tick, streams, events().map(Message::Event)])
    }
}
