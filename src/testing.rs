use serde::{Deserialize, Serialize};

use crate::theme::Theme;

#[derive(Serialize, Deserialize)]
pub struct Label {
    pub theme: Theme,
    pub items: Vec<String>,
}
