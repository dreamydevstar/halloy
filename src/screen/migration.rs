use data::environment::MIGRATION_WEBSITE;
use data::Config;
use iced::widget::{button, column, container, text, vertical_space};
use iced::{alignment, Length};

use crate::widget::Element;
use crate::{font, theme};

#[derive(Debug, Clone)]
pub enum Message {
    RefreshConfiguration,
    OpenConfigurationDirectory,
    OpenMigrationWebsite,
}

#[derive(Debug, Clone)]
pub enum Event {
    RefreshConfiguration,
}

#[derive(Debug, Default, Clone)]
pub struct Migration;

impl Migration {
    pub fn new() -> Self {
        // Create template config file.
        Config::create_template_config();

        Migration
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::RefreshConfiguration => Some(Event::RefreshConfiguration),
            Message::OpenConfigurationDirectory => {
                let _ = open::that(Config::config_dir());

                None
            }
            Message::OpenMigrationWebsite => {
                let _ = open::that(MIGRATION_WEBSITE);

                None
            }
        }
    }

    pub fn view<'a>(&self) -> Element<'a, Message> {
        let config_button = button(
            container(text("Open Config Directory"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fill)
        .style(theme::button::secondary)
        .on_press(Message::OpenConfigurationDirectory);

        let wiki_button = button(
            container(text("Open Migration Guide"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fill)
        .style(theme::button::secondary)
        .on_press(Message::OpenMigrationWebsite);

        let refresh_button = button(
            container(text("Refresh Halloy"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(Length::Fill)
        .style(theme::button::primary)
        .on_press(Message::RefreshConfiguration);

        let content = column![]
            .spacing(1)
            .push(vertical_space().height(10))
            .push(text("Your configuration file is outdated :(").font(font::MONO_BOLD.clone()))
            .push(vertical_space().height(4))
            .push(text(
                "Halloy recently switched configuration file format from YAML to TOML. This was done in an effort to make it easier to work with as a user.",
            ))
            .push(vertical_space().height(8))
            .push(text("To migrate your configuration file, please visit the migration guide below."))
            .push(vertical_space().height(10))
            .push(
                column![]
                    .width(250)
                    .spacing(4)
                    .push(config_button)
                    .push(wiki_button)
                    .push(refresh_button),
            )
            .width(350)
            .align_items(iced::Alignment::Center);

        container(content)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
