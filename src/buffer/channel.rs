use std::fmt;

use data::history;
use data::server::Server;
use iced::widget::{column, container, row, scrollable, text, vertical_space};
use iced::{Command, Length};

use super::{input_view, scroll_view};
use crate::theme;
use crate::widget::{selectable_text, Collection, Column, Element};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(
    state: &'a Channel,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    channel_config: &data::channel::Config,
    buffer_config: &'a data::config::Buffer,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Channel(&state.server, &state.channel),
            history,
            |message| match &message.source {
                data::message::Source::Channel(_, kind) => match kind {
                    data::message::Sender::User(user) => {
                        let timestamp = buffer_config.timestamp.clone().map(|timestamp| {
                            let content = &message.formatted_datetime(timestamp.format.as_str());
                            selectable_text(content_with_brackets(content, &timestamp.brackets))
                                .style(theme::Text::Alpha04)
                        });
                        let nick = selectable_text(content_with_brackets(
                            user,
                            &buffer_config.nickname.brackets,
                        ))
                        .style(theme::Text::Nickname(
                            user.color_seed(&buffer_config.nickname.color),
                        ));
                        let message = selectable_text(&message.text);

                        Some(
                            container(row![].push_maybe(timestamp).push(nick).push(message)).into(),
                        )
                    }
                    data::message::Sender::Server => Some(
                        container(selectable_text(&message.text).style(theme::Text::Server)).into(),
                    ),
                    data::message::Sender::Action => Some(
                        container(selectable_text(&message.text).style(theme::Text::Accent)).into(),
                    ),
                },
                _ => None,
            },
        )
        .map(Message::ScrollView),
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let spacing = is_focused.then_some(vertical_space(4));
    let text_input = is_focused.then(|| {
        input_view::view(
            &state.input_view,
            data::Buffer::Channel(state.server.clone(), state.channel.clone()),
        )
        .map(Message::InputView)
    });

    let user_column = {
        let users = clients.get_channel_users(&state.server, &state.channel);
        let column = Column::with_children(
            users
                .iter()
                .map(|user| {
                    container(row![].padding([0, 4]).push(text(format!(
                        "{}{}",
                        user.highest_access_level(),
                        user.nickname()
                    ))))
                    .into()
                })
                .collect(),
        )
        .padding(4)
        .spacing(1);

        container(
            scrollable(column)
                .vertical_scroll(
                    iced::widget::scrollable::Properties::new()
                        .width(1)
                        .scroller_width(1),
                )
                .style(theme::Scrollable::Hidden),
        )
        .width(Length::Shrink)
        .max_width(120)
        .height(Length::Fill)
    };

    let content = match (channel_config.users.visible, channel_config.users.position) {
        (true, data::channel::Position::Left) => {
            row![user_column, messages]
        }
        (true, data::channel::Position::Right) => {
            row![messages, user_column]
        }
        (false, _) => { row![messages] }.height(Length::Fill),
    };

    let scrollable = column![container(content).height(Length::Fill)]
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
pub struct Channel {
    pub server: Server,
    pub channel: String,
    pub topic: Option<String>,
    pub scroll_view: scroll_view::State,
    input_view: input_view::State,
}

impl Channel {
    pub fn new(server: Server, channel: String) -> Self {
        Self {
            server,
            channel,
            topic: None,
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

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channel = self.channel.to_string();

        write!(f, "{} ({})", channel, self.server)
    }
}
