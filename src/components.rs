use shipyard::EntityId;
use std::collections::HashMap;

pub struct BlocksTile;

pub struct FieldOfView {
    pub tiles: HashMap<(i32, i32), bool>,
    pub range: i32,
    pub dirty: bool,
    pub mark: bool,
}

impl FieldOfView {
    pub fn new(range: i32) -> FieldOfView {
        assert!(range > 0);

        FieldOfView {
            tiles: HashMap::new(),
            range,
            dirty: true,
            mark: false,
        }
    }
}

pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn dist(&self, other: &Position) -> i32 {
        std::cmp::max((other.x - self.x).abs(), (other.y - self.y).abs())
    }
}

impl From<&Position> for (i32, i32) {
    fn from(pos: &Position) -> Self {
        (pos.x, pos.y)
    }
}

impl From<&mut Position> for (i32, i32) {
    fn from(pos: &mut Position) -> Self {
        (pos.x, pos.y)
    }
}

impl From<(i32, i32)> for Position {
    fn from((x, y): (i32, i32)) -> Self {
        Position { x, y }
    }
}

pub struct Renderable {
    pub ch: char,
    pub fg: [f32; 4],
    pub bg: [f32; 4],
}

pub struct Player;

pub struct PlayerId(pub EntityId);

pub struct Monster;

pub struct Name(pub String);
