use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    pub users: Users,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub enum Position {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Users {
    pub visible: bool,
    #[serde(default)]
    pub position: Position,
}

impl Default for Users {
    fn default() -> Self {
        Self {
            visible: true,
            position: Position::Left,
        }
    }
}

impl Users {
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible
    }
}
