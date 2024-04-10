use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct DataPoint {
    pub x: f32,
    pub y: f32,
}
