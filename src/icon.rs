use iced::widget::text;
use iced::widget::text::LineHeight;

use crate::widget::Text;
use crate::{font, theme};

// Based off https://github.com/iced-rs/iced_aw/blob/main/src/graphics/icons/bootstrap.rs

pub fn dot<'a>() -> Text<'a> {
    to_text('\u{f287}')
}

pub fn error<'a>() -> Text<'a> {
    to_text('\u{f33a}')
}

pub fn globe<'a>() -> Text<'a> {
    to_text('\u{f3ef}')
}

pub fn wifi_off<'a>() -> Text<'a> {
    to_text('\u{f61b}')
}

pub fn close<'a>() -> Text<'a> {
    to_text('\u{f659}')
}

pub fn maximize<'a>() -> Text<'a> {
    to_text('\u{f14a}')
}

pub fn restore<'a>() -> Text<'a> {
    to_text('\u{f149}')
}

pub fn people<'a>() -> Text<'a> {
    to_text('\u{f4db}')
}

pub fn topic<'a>() -> Text<'a> {
    to_text('\u{f5af}')
}

pub fn file_transfer<'a>() -> Text<'a> {
    to_text('\u{f30a}')
}

pub fn arrow_down<'a>() -> Text<'a> {
    to_text('\u{f128}')
}

pub fn arrow_up<'a>() -> Text<'a> {
    to_text('\u{f148}')
}

pub fn download<'a>() -> Text<'a> {
    to_text('\u{f30a}')
}

pub fn trashcan<'a>() -> Text<'a> {
    to_text('\u{f5de}')
}

pub fn folder<'a>() -> Text<'a> {
    to_text('\u{f3d8}')
}

pub fn search<'a>() -> Text<'a> {
    to_text('\u{f52a}')
}

pub fn secure<'a>() -> Text<'a> {
    to_text('\u{f538}')
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .style(theme::text::primary)
        .line_height(LineHeight::Relative(1.0))
        .size(theme::ICON_SIZE)
        .font(font::ICON)
}
