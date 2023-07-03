use data::server::Server;
use data::{buffer, client, history};
use iced::widget::{column, container, row, vertical_space};
use iced::{Command, Length};

use super::{input_view, scroll_view, user_context};
use crate::theme;
use crate::widget::{selectable_text, Collection, Element};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
    UserContext(user_context::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
}

pub fn view<'a>(
    state: &'a Channel,
    status: client::Status,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    settings: &'a buffer::Settings,
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
                        let timestamp =
                            settings
                                .format_timestamp(&message.server_time)
                                .map(|timestamp| {
                                    selectable_text(timestamp).style(theme::Text::Alpha04)
                                });
                        let nick = user_context::view(
                            selectable_text(settings.nickname.brackets.format(user)).style(
                                theme::Text::Nickname(user.color_seed(&settings.nickname.color)),
                            ),
                            user.clone(),
                        )
                        .map(scroll_view::Message::UserContext);
                        let row_style = match clients.connection(&state.server) {
                            Some(conn)
                                if user.nickname() != conn.nickname()
                                    && message.text.contains(&conn.nickname().to_string()) =>
                            {
                                theme::Container::Highlight
                            }
                            _ => theme::Container::Default,
                        };
                        let message = selectable_text(&message.text);
                        Some(
                            container(row![].push_maybe(timestamp).push(nick).push(message))
                                .style(row_style)
                                .into(),
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
    let text_input = (is_focused && status.connected()).then(|| {
        input_view::view(
            &state.input_view,
            data::Buffer::Channel(state.server.clone(), state.channel.clone()),
        )
        .map(Message::InputView)
    });

    let users = clients.get_channel_users(&state.server, &state.channel);
    let nick_list = nick_list::view(users).map(Message::UserContext);

    let content = match (
        settings.channel.users.visible,
        settings.channel.users.position,
    ) {
        (true, data::channel::Position::Left) => {
            row![nick_list, messages]
        }
        (true, data::channel::Position::Right) => {
            row![messages, nick_list]
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
                let (command, event) = self.scroll_view.update(message);

                let event = event.map(|event| match event {
                    scroll_view::Event::UserContext(event) => Event::UserContext(event),
                });

                (command.map(Message::ScrollView), event)
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
            Message::UserContext(message) => (
                Command::none(),
                Some(Event::UserContext(user_context::update(message))),
            ),
        }
    }

    pub fn focus(&self) -> Command<Message> {
        self.input_view.focus().map(Message::InputView)
    }
}

mod nick_list {
    use data::User;
    use iced::widget::{column, container, row, scrollable, text};
    use iced::Length;
    use user_context::Message;

    use crate::buffer::user_context;
    use crate::theme;
    use crate::widget::Element;

    pub fn view<'a>(users: Vec<User>) -> Element<'a, Message> {
        let column = column(
            users
                .iter()
                .map(|user| {
                    let content = container(row![].padding([0, 4]).push(text(format!(
                        "{}{}",
                        user.highest_access_level(),
                        user.nickname()
                    ))));

                    user_context::view(content, user.clone())
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
        .into()
    }
}
