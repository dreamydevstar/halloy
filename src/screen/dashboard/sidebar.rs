use data::dashboard::DefaultAction;
use data::{history, Buffer};
use iced::widget::{
    button, column, container, horizontal_space, pane_grid, row, scrollable, text, vertical_space,
    Scrollable,
};
use iced::Length;

use super::pane::Pane;
use crate::widget::{context_menu, Element};
use crate::{icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Open(Buffer),
    Replace(Buffer, pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
    Leave(Buffer),
    ToggleFileTransfers,
    ToggleCommandBar,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(Buffer),
    Replace(Buffer, pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
    Leave(Buffer),
    ToggleFileTransfers,
    ToggleCommandBar,
}

#[derive(Clone)]
pub struct Sidebar {
    hidden: bool,
}

impl Sidebar {
    pub fn new() -> Self {
        Self { hidden: false }
    }

    pub fn toggle_visibility(&mut self) {
        self.hidden = !self.hidden
    }

    pub fn update(&mut self, message: Message) -> Event {
        match message {
            Message::Open(source) => Event::Open(source),
            Message::Replace(source, pane) => Event::Replace(source, pane),
            Message::Close(pane) => Event::Close(pane),
            Message::Swap(from, to) => Event::Swap(from, to),
            Message::Leave(buffer) => Event::Leave(buffer),
            Message::ToggleFileTransfers => Event::ToggleFileTransfers,
            Message::ToggleCommandBar => Event::ToggleCommandBar,
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        history: &'a history::Manager,
        panes: &pane_grid::State<Pane>,
        focus: Option<pane_grid::Pane>,
        config: data::config::Sidebar,
    ) -> Option<Element<'a, Message>> {
        if self.hidden {
            return None;
        }

        let mut column = column![].spacing(1);

        for (server, state) in clients.iter() {
            match state {
                data::client::State::Disconnected => {
                    column = column.push(buffer_button(
                        panes,
                        focus,
                        Buffer::Server(server.clone()),
                        false,
                        false,
                        config.default_action,
                    ));
                }
                data::client::State::Ready(connection) => {
                    column = column.push(buffer_button(
                        panes,
                        focus,
                        Buffer::Server(server.clone()),
                        true,
                        false,
                        config.default_action,
                    ));

                    for channel in connection.channels() {
                        column = column.push(buffer_button(
                            panes,
                            focus,
                            Buffer::Channel(server.clone(), channel.clone()),
                            true,
                            history.has_unread(server, &history::Kind::Channel(channel.clone())),
                            config.default_action,
                        ));
                    }

                    let queries = history.get_unique_queries(server);
                    for user in queries {
                        column = column.push(buffer_button(
                            panes,
                            focus,
                            Buffer::Query(server.clone(), user.clone()),
                            true,
                            history.has_unread(server, &history::Kind::Query(user.clone())),
                            config.default_action,
                        ));
                    }

                    column = column.push(vertical_space().height(12));
                }
            }
        }

        let mut menu_buttons = row![].spacing(1).padding([0, 0, 4, 0]);

        if config.buttons.command_bar {
            let command_bar_button = button(
                container(icon::search())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .on_press(Message::ToggleCommandBar)
            .padding(5)
            .width(22)
            .height(22)
            .style(theme::button::side_menu);

            menu_buttons = menu_buttons.push(command_bar_button);
        }

        if config.buttons.file_transfer {
            let file_transfers_open = panes
                .iter()
                .any(|(_, pane)| matches!(pane.buffer, crate::buffer::Buffer::FileTransfers(_)));

            let file_transfers_button = button(
                container(icon::file_transfer())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .on_press(Message::ToggleFileTransfers)
            .padding(5)
            .width(22)
            .height(22)
            .style(if file_transfers_open {
                theme::button::side_menu_selected
            } else {
                theme::button::side_menu
            });

            menu_buttons = menu_buttons.push(file_transfers_button);
        }

        let content = column![Scrollable::with_direction(
            column,
            scrollable::Direction::Vertical(
                iced::widget::scrollable::Properties::default()
                    .width(0)
                    .scroller_width(0),
            ),
        ),];

        let body = column![menu_buttons, container(content).height(Length::Fill)];

        Some(
            container(body)
                .height(Length::Fill)
                .padding([8, 0, 6, 6])
                .center_x()
                .max_width(config.width)
                .into(),
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum Entry {
    NewPane,
    Replace(pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
    Leave,
}

impl Entry {
    fn list(
        num_panes: usize,
        open: Option<pane_grid::Pane>,
        focus: Option<pane_grid::Pane>,
    ) -> Vec<Self> {
        match (open, focus) {
            (None, None) => vec![Entry::NewPane, Entry::Leave],
            (None, Some(focus)) => vec![Entry::NewPane, Entry::Replace(focus), Entry::Leave],
            (Some(open), None) => (num_panes > 1)
                .then_some(Entry::Close(open))
                .into_iter()
                .chain(Some(Entry::Leave))
                .collect(),
            (Some(open), Some(focus)) => (num_panes > 1)
                .then_some(Entry::Close(open))
                .into_iter()
                .chain((open != focus).then_some(Entry::Swap(open, focus)))
                .chain(Some(Entry::Leave))
                .collect(),
        }
    }
}

fn buffer_button<'a>(
    panes: &pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    buffer: Buffer,
    connected: bool,
    has_unread: bool,
    default_action: DefaultAction,
) -> Element<'a, Message> {
    let open = panes
        .iter()
        .find_map(|(pane, state)| (state.buffer.data().as_ref() == Some(&buffer)).then_some(*pane));

    let row = match &buffer {
        Buffer::Server(server) => row![
            if connected {
                icon::globe()
            } else {
                icon::wifi_off()
            },
            text(server.to_string()).style(theme::text::primary)
        ]
        .spacing(8)
        .align_items(iced::Alignment::Center),
        Buffer::Channel(_, channel) => row![]
            .push(horizontal_space().width(3))
            .push_maybe(has_unread.then_some(icon::dot().size(6).style(theme::text::info)))
            .push(horizontal_space().width(if has_unread { 10 } else { 16 }))
            .push(text(channel).style(theme::text::primary))
            .align_items(iced::Alignment::Center),
        Buffer::Query(_, nick) => row![]
            .push(horizontal_space().width(3))
            .push_maybe(has_unread.then_some(icon::dot().size(6).style(theme::text::info)))
            .push(horizontal_space().width(if has_unread { 10 } else { 16 }))
            .push(text(nick).style(theme::text::primary))
            .align_items(iced::Alignment::Center),
    };

    let base = button(row)
        .padding(5)
        .width(Length::Fill)
        .style(if open.is_some() {
            theme::button::side_menu_selected
        } else {
            theme::button::side_menu
        })
        .on_press(match default_action {
            DefaultAction::NewPane => Message::Open(buffer.clone()),
            DefaultAction::ReplacePane => match focus {
                Some(pane) => Message::Replace(buffer.clone(), pane),
                None => Message::Open(buffer.clone()),
            },
        });

    let entries = Entry::list(panes.len(), open, focus);

    if entries.is_empty() || !connected {
        base.into()
    } else {
        context_menu(base, entries, move |entry, length| {
            let (content, message) = match entry {
                Entry::NewPane => ("Open in new pane", Message::Open(buffer.clone())),
                Entry::Replace(pane) => (
                    "Replace current pane",
                    Message::Replace(buffer.clone(), pane),
                ),
                Entry::Close(pane) => ("Close pane", Message::Close(pane)),
                Entry::Swap(from, to) => ("Swap with current pane", Message::Swap(from, to)),
                Entry::Leave => (
                    match &buffer {
                        Buffer::Server(_) => "Leave server",
                        Buffer::Channel(_, _) => "Leave channel",
                        Buffer::Query(_, _) => "Close query",
                    },
                    Message::Leave(buffer.clone()),
                ),
            };

            button(text(content).style(theme::text::primary))
                .width(length)
                .padding(5)
                .style(theme::button::context)
                .on_press(message)
                .into()
        })
    }
}
