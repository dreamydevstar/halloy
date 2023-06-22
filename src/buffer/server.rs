use std::fmt;

use data::history;
use iced::widget::{column, container, row, vertical_space};
use iced::{Command, Length};

use super::{input_view, scroll_view};
use crate::theme;
use crate::widget::{selectable_text, Collection, Element};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(
    state: &'a Server,
    history: &'a history::Manager,
    buffer_config: &'a data::config::Buffer,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Server(&state.server),
            history,
            |message| {
                let timestamp = buffer_config.timestamp.clone().map(|timestamp| {
                    let content = &message.formatted_datetime(timestamp.format.as_str());
                    selectable_text(content_with_brackets(content, &timestamp.brackets))
                        .style(theme::Text::Alpha04)
                });
                let message = selectable_text(&message.text).style(theme::Text::Alpha04);

                Some(container(row![].push_maybe(timestamp).push(message)).into())
            },
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);
    let spacing = is_focused.then_some(vertical_space(4));
    let text_input = is_focused.then(|| {
        input_view::view(
            &state.input_view,
            data::Buffer::Server(state.server.clone()),
        )
        .map(Message::InputView)
    });

    let scrollable = column![messages]
        .push_maybe(spacing)
        .push_maybe(text_input)
        .height(Length::Fill);

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone)]
pub struct Server {
    pub server: data::server::Server,
    pub scroll_view: scroll_view::State,
    input_view: input_view::State,
}

impl Server {
    pub fn new(server: data::server::Server) -> Self {
        Self {
            server,
            scroll_view: scroll_view::State::new(),
            input_view: input_view::State::new(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let command = self.scroll_view.update(message);
                (command.map(Message::ScrollView), None)
            }
            Message::InputView(message) => {
                let (command, event) =
                    self.input_view
                        .update(message, &self.server, clients, history);
                let command = command.map(Message::InputView);

                match event {
                    Some(input_view::Event::InputSent) => {
                        let command = Command::batch(vec![
                            command,
                            self.scroll_view.scroll_to_end().map(Message::ScrollView),
                        ]);

                        (command, None)
                    }
                    None => (command, None),
                }
            }
        }
    }

    pub fn focus(&self) -> Command<Message> {
        self.input_view.focus().map(Message::InputView)
    }
}

fn content_with_brackets(
    content: impl std::fmt::Display,
    brackets: &data::config::Brackets,
) -> String {
    format!("{}{}{} ", brackets.left, content, brackets.right)
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.server)
    }
}
