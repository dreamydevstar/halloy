mod config;
mod screen;
mod style;
mod theme;

use config::Config;
use iced::{
    executor,
    pure::{container, Application, Element},
    Command, Length, Settings,
};
use screen::dashboard;
use theme::Theme;

pub fn main() -> iced::Result {
    Halloy::run(Settings::default())
}

struct Halloy {
    screen: Screen,
    theme: Theme,
}

enum Screen {
    Dashboard(screen::Dashboard),
}

#[derive(Debug, Clone)]
enum Message {
    Dashboard(dashboard::Message),
    ConfigSaved(Result<(), config::Error>),
}

impl Application for Halloy {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Halloy, Command<Self::Message>) {
        let screen = screen::Dashboard::new();
        let config = Config::load().unwrap_or_default();

        (
            Halloy {
                screen: Screen::Dashboard(screen),
                theme: config.theme,
            },
            Command::perform(config.save(), Message::ConfigSaved),
        )
    }

    fn title(&self) -> String {
        String::from("Halloy")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Dashboard(message) => match &mut self.screen {
                Screen::Dashboard(dashboard) => {
                    if let Some((_event, _command)) = dashboard.update(message) {
                        // Handle events and commands.
                    }
                }
            },
            Message::ConfigSaved(_) => {
                println!("config saved.")
            }
        }

        Command::none()
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        let content = match &self.screen {
            Screen::Dashboard(dashboard) => dashboard.view(&self.theme).map(Message::Dashboard),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
