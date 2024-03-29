use iced::widget::button::{Appearance, DefaultStyle, Status};
use iced::{Background, Border, Color};

use super::Theme;

impl DefaultStyle for Theme {
    fn default_style(&self, status: Status) -> Appearance {
        primary(self, status)
    }
}

pub fn primary(theme: &Theme, status: Status) -> Appearance {
    match status {
        Status::Active | Status::Pressed => Appearance {
            background: Some(Background::Color(theme.colors().text.high_alpha)),
            text_color: theme.colors().text.base,
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        Status::Hovered => Appearance {
            background: Some(Background::Color(theme.colors().text.med_alpha)),
            text_color: theme.colors().text.base,
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        Status::Disabled => {
            let active = primary(theme, Status::Active);

            Appearance {
                text_color: Color {
                    a: 0.2,
                    ..active.text_color
                },
                border: Border {
                    color: Color {
                        a: 0.2,
                        ..active.text_color
                    },
                    ..Default::default()
                },
                ..active
            }
        }
    }
}

pub fn secondary(theme: &Theme, status: Status) -> Appearance {
    match status {
        Status::Active | Status::Pressed => Appearance {
            background: Some(Background::Color(theme.colors().accent.high_alpha)),
            text_color: theme.colors().accent.base,
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        Status::Hovered => Appearance {
            background: Some(Background::Color(theme.colors().accent.med_alpha)),
            text_color: theme.colors().accent.base,
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        Status::Disabled => {
            let active = secondary(theme, Status::Active);

            Appearance {
                text_color: Color {
                    a: 0.2,
                    ..active.text_color
                },
                border: Border {
                    color: Color {
                        a: 0.2,
                        ..active.text_color
                    },
                    ..Default::default()
                },
                ..active
            }
        }
    }
}

pub fn tertiary(theme: &Theme, status: Status, selected: bool) -> Appearance {
    match status {
        Status::Active | Status::Pressed => Appearance {
            background: Some(Background::Color(if selected {
                theme.colors().action.med_alpha
            } else {
                theme.colors().background.dark
            })),
            border: Border {
                color: if selected {
                    theme.colors().action.low_alpha
                } else {
                    if theme.colors().is_dark_theme() {
                        theme.colors().background.lightest
                    } else {
                        theme.colors().background.darkest
                    }
                },
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        },
        Status::Hovered => {
            let active = tertiary(theme, Status::Active, selected);

            Appearance {
                background: Some(Background::Color(if selected {
                    theme.colors().action.high_alpha
                } else {
                    if theme.colors().is_dark_theme() {
                        theme.colors().background.light
                    } else {
                        theme.colors().background.darker
                    }
                })),
                ..active
            }
        }
        Status::Disabled => {
            let active = tertiary(theme, Status::Active, selected);

            Appearance {
                text_color: Color {
                    a: 0.2,
                    ..active.text_color
                },
                border: Border {
                    color: Color {
                        a: 0.2,
                        ..active.text_color
                    },
                    ..Default::default()
                },
                ..active
            }
        }
    }
}

pub fn context(theme: &Theme, status: Status) -> Appearance {
    match status {
        Status::Active | Status::Pressed => Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        Status::Hovered => {
            let active = context(theme, Status::Active);

            Appearance {
                background: Some(Background::Color(theme.colors().background.darker)),
                ..active
            }
        }
        Status::Disabled => {
            let active = context(theme, Status::Active);

            Appearance {
                text_color: Color {
                    a: 0.2,
                    ..active.text_color
                },
                border: Border {
                    color: Color {
                        a: 0.2,
                        ..active.text_color
                    },
                    ..Default::default()
                },
                ..active
            }
        }
    }
}

pub fn bare(_theme: &Theme, status: Status) -> Appearance {
    match status {
        Status::Active | Status::Pressed | Status::Hovered => Appearance {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        },
        Status::Disabled => {
            let active = bare(_theme, Status::Active);

            Appearance {
                text_color: Color {
                    a: 0.2,
                    ..active.text_color
                },
                border: Border {
                    color: Color {
                        a: 0.2,
                        ..active.text_color
                    },
                    ..Default::default()
                },
                ..active
            }
        }
    }
}

pub fn side_menu(theme: &Theme, status: Status) -> Appearance {
    match status {
        Status::Active | Status::Pressed => Appearance {
            background: None,
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        Status::Hovered => {
            let active = side_menu(theme, Status::Active);

            Appearance {
                background: Some(Background::Color(theme.colors().background.dark)),
                ..active
            }
        }
        Status::Disabled => {
            let active = side_menu(theme, Status::Active);

            Appearance {
                text_color: Color {
                    a: 0.2,
                    ..active.text_color
                },
                border: Border {
                    color: Color {
                        a: 0.2,
                        ..active.text_color
                    },
                    ..Default::default()
                },
                ..active
            }
        }
    }
}

pub fn side_menu_selected(theme: &Theme, status: Status) -> Appearance {
    match status {
        Status::Active | Status::Pressed => Appearance {
            background: Some(Background::Color(theme.colors().background.darker)),
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        Status::Hovered => {
            let active = side_menu_selected(theme, Status::Active);

            Appearance {
                background: Some(Background::Color(theme.colors().background.darkest)),
                ..active
            }
        }
        Status::Disabled => {
            let active = side_menu_selected(theme, Status::Active);

            Appearance {
                text_color: Color {
                    a: 0.2,
                    ..active.text_color
                },
                border: Border {
                    color: Color {
                        a: 0.2,
                        ..active.text_color
                    },
                    ..Default::default()
                },
                ..active
            }
        }
    }
}
