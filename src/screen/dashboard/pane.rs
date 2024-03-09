use data::{history, Config};
use iced::widget::{button, container, pane_grid, row, text};
use iced::Length;
use uuid::Uuid;

use crate::buffer::{self, Buffer};
use crate::{icon, theme, widget};

#[derive(Debug, Clone)]
pub enum Message {
    PaneClicked(pane_grid::Pane),
    PaneResized(pane_grid::ResizeEvent),
    PaneDragged(pane_grid::DragEvent),
    Buffer(pane_grid::Pane, buffer::Message),
    ClosePane,
    SplitPane(pane_grid::Axis),
    MaximizePane,
    ToggleShowUserList,
    ToggleShowTopic,
}

#[derive(Clone)]
pub struct Pane {
    pub id: Uuid,
    pub buffer: Buffer,
    title_bar: TitleBar,
    settings: buffer::Settings,
}

#[derive(Debug, Clone, Default)]
pub struct TitleBar {}

impl Pane {
    pub fn new(buffer: Buffer, config: &Config) -> Self {
        Self::with_settings(buffer, buffer::Settings::from(config.buffer.clone()))
    }

    pub fn with_settings(buffer: Buffer, settings: buffer::Settings) -> Self {
        Self {
            id: Uuid::new_v4(),
            buffer,
            title_bar: TitleBar::default(),
            settings,
        }
    }

    pub fn view<'a>(
        &'a self,
        id: pane_grid::Pane,
        panes: usize,
        is_focused: bool,
        maximized: bool,
        clients: &'a data::client::Map,
        history: &'a history::Manager,
        config: &'a Config,
    ) -> widget::Content<'a, Message> {
        let title_bar_text = match &self.buffer {
            Buffer::Empty => "".to_string(),
            Buffer::Channel(state) => {
                let channel = &state.channel;
                let server = &state.server;
                let users = clients
                    .get_channel_users(&state.server, &state.channel)
                    .len();

                format!("{channel} @ {server} - {users} users")
            }
            Buffer::Server(state) => state.server.to_string(),
            Buffer::Query(state) => {
                let nick = &state.nick;
                let server = &state.server;

                format!("{nick} @ {server}")
            }
        };

        let title_bar = self.title_bar.view(
            &self.buffer,
            title_bar_text,
            id,
            panes,
            is_focused,
            maximized,
            clients,
            &self.settings,
        );

        let content = self
            .buffer
            .view(clients, history, &self.settings, config, is_focused)
            .map(move |msg| Message::Buffer(id, msg));

        widget::Content::new(content)
            .style(theme::Container::PaneBody {
                selected: is_focused,
            })
            .title_bar(title_bar.style(theme::Container::PaneHeader))
    }

    pub fn resource(&self) -> Option<history::Resource> {
        match &self.buffer {
            Buffer::Empty => None,
            Buffer::Channel(channel) => Some(history::Resource {
                server: channel.server.clone(),
                kind: history::Kind::Channel(channel.channel.clone()),
            }),
            Buffer::Server(server) => Some(history::Resource {
                server: server.server.clone(),
                kind: history::Kind::Server,
            }),
            Buffer::Query(query) => Some(history::Resource {
                server: query.server.clone(),
                kind: history::Kind::Query(query.nick.clone()),
            }),
        }
    }

    pub fn update_settings(&mut self, f: impl FnOnce(&mut buffer::Settings)) {
        f(&mut self.settings);
    }
}

impl TitleBar {
    fn view<'a>(
        &'a self,
        buffer: &Buffer,
        value: String,
        _id: pane_grid::Pane,
        panes: usize,
        _is_focused: bool,
        maximized: bool,
        clients: &'a data::client::Map,
        settings: &'a buffer::Settings,
    ) -> widget::TitleBar<'a, Message> {
        // Pane controls.
        let mut controls = row![].spacing(2);

        if let Buffer::Channel(state) = &buffer {
            // Show topic button only if there is a topic to show
            if let Some(topic) = clients.get_channel_topic(&state.server, &state.channel) {
                if topic.text.is_some() {
                    let topic = button(
                        container(icon::topic())
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .center_x()
                            .center_y(),
                    )
                    .width(22)
                    .height(22)
                    .on_press(Message::ToggleShowTopic)
                    .style(theme::Button::Pane {
                        selected: settings.channel.topic.visible,
                    });

                    controls = controls.push(topic);
                }
            }

            let users = button(
                container(icon::people())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .width(22)
            .height(22)
            .on_press(Message::ToggleShowUserList)
            .style(theme::Button::Pane {
                selected: settings.channel.users.visible,
            });

            controls = controls.push(users);
        }

        // If we have more than one pane open, show maximize button.
        if panes > 1 {
            let maximize = button(
                container(if maximized {
                    icon::restore()
                } else {
                    icon::maximize()
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y(),
            )
            .width(22)
            .height(22)
            .on_press(Message::MaximizePane)
            .style(theme::Button::Pane {
                selected: maximized,
            });

            controls = controls.push(maximize);
        }

        // Add delete as long as it's not a single empty buffer
        if !(panes == 1 && matches!(buffer, Buffer::Empty)) {
            let delete = button(
                container(icon::close())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            )
            .width(22)
            .height(22)
            .on_press(Message::ClosePane)
            .style(theme::Button::Pane { selected: false });

            controls = controls.push(delete);
        }

        let title = container(text(value).style(theme::Text::Transparent))
            .height(22)
            .padding([0, 4])
            .align_y(iced::alignment::Vertical::Center);

        widget::TitleBar::new(title).controls(controls).padding(6)
    }
}

impl From<Pane> for data::Pane {
    fn from(pane: Pane) -> Self {
        let buffer = match pane.buffer {
            Buffer::Empty => return data::Pane::Empty,
            Buffer::Channel(state) => data::Buffer::Channel(state.server, state.channel),
            Buffer::Server(state) => data::Buffer::Server(state.server),
            Buffer::Query(state) => data::Buffer::Query(state.server, state.nick),
        };

        data::Pane::Buffer {
            buffer,
            settings: pane.settings,
        }
    }
}
