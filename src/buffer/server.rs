use std::fmt;

use data::server::Server;
use data::theme::Theme;
use iced::{
    pure::{self, container, widget::Column, Element},
    Length,
};

use crate::{style, widget::sticky_scrollable::scrollable};

#[derive(Debug, Clone)]
pub enum Message {}

pub fn view<'a>(
    state: &'a State,
    clients: &data::client::Map,
    _is_focused: bool,
    _theme: &'a Theme,
) -> Element<'a, Message> {
    let messages: Vec<Element<'a, Message>> = clients
        .get_server_messages(&state.server)
        .into_iter()
        .filter_map(|message| {
            Some(container(pure::text(message.text()?).size(style::TEXT_SIZE)).into())
        })
        .collect();

    container(scrollable(
        Column::with_children(messages).width(Length::Fill),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding([0, 8])
    .into()
}

#[derive(Debug, Clone)]
pub struct State {
    server: Server,
}

impl State {
    pub fn new(server: Server) -> Self {
        Self { server }
    }

    pub fn _update(&mut self, _message: Message, _clients: &data::client::Map) {}
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.server)
    }
}
