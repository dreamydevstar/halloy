use data::Config;
use iced::widget::{column, combo_box, container, text};
use iced::Length;

use crate::theme;
use crate::widget::{double_pass, key_press, Element};

#[derive(Debug, Clone)]
pub struct CommandBar {
    state: combo_box::State<Command>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Command(Command),
    Unfocused,
    Ignored,
}

impl CommandBar {
    pub fn new() -> Self {
        let state = combo_box::State::new(Command::list());
        state.focus();

        Self { state }
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::Command(command) => Some(Event::Command(command)),
            Message::Unfocused => Some(Event::Unfocused),
            Message::Ignored => None,
        }
    }

    pub fn view<'a>(&'a self, config: &'a Config) -> Element<'a, Message> {
        // 1px larger than default
        let font_size = config.font.size.map(f32::from).unwrap_or(theme::TEXT_SIZE) + 1.0;

        let combo_box = combo_box(&self.state, "Type a command...", None, Message::Command)
            .on_close(Message::Unfocused)
            .style(theme::ComboBox::Default)
            .size(font_size)
            .padding([8, 8]);

        // Capture ESC so we can close the combobox manually from application
        // and prevent undesired effects
        let combo_box = key_press(
            combo_box,
            key_press::KeyCode::Escape,
            key_press::Modifiers::default(),
            Message::Ignored,
        );

        double_pass(
            // Layout should be based on the Shrink text size width of largest option
            column(
                std::iter::once(text("Type a command...").size(font_size))
                    .chain(
                        Command::list()
                            .iter()
                            .map(|command| text(command).size(font_size)),
                    )
                    .map(Element::from)
                    .collect(),
            )
            // Give it some extra width
            .padding([0, 20]),
            container(combo_box)
                .padding(1)
                .style(theme::Container::Context)
                .width(Length::Fill),
        )
    }
}

pub enum Event {
    Command(Command),
    Unfocused,
}

#[derive(Debug, Clone)]
pub enum Command {
    OpenConfig,
    ToggleSidebarVisibility,
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::OpenConfig => write!(f, "Configuration: Open Directory"),
            Command::ToggleSidebarVisibility => write!(f, "UI: Toggle Sidebar visibility"),
        }
    }
}

impl Command {
    pub fn list() -> Vec<Self> {
        vec![Command::OpenConfig, Command::ToggleSidebarVisibility]
    }
}
