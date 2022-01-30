use std::ops::{Index, IndexMut};
use crate::vecmath::{Rectangle, V2};
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum CellColor {
    White,
    Black,
    LightGray,
    DarkGray,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub letter: char,
    pub background: CellColor,
    pub foreground: CellColor,
}

impl Cell {
    pub fn empty(&self) -> bool { self.letter == '\0' || self.letter == ' ' }
}

static EMPTY_CELL: Cell = Cell {
    letter: '\0',
    background: CellColor::Black,
    foreground: CellColor::White,
};

impl Cell {
    pub fn make_empty() -> Cell {
        return EMPTY_CELL;
        /*Cell {
            letter: '\0',
            background: CellColor::Black,
            foreGround: CellColor::Black,
        }*/
    }
}


#[derive(Serialize, Deserialize, Clone)]
pub struct Trigger {
    pub pos: V2,
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Level {
    pub width: i32,
    pub height: i32,
    pub p0: V2,
    pub triggers: Vec<Trigger>,
    pub data: Vec<Vec<Cell>>,
}

impl Level {
    pub fn new(width: i32, height: i32) -> Level {
        return Level {
            data: vec![vec![Cell::make_empty(); width as usize]; height as usize],
            width,
            height,
            p0: V2::make(2, 2),
            triggers: vec![],
        };
    }

    pub fn size(&self) -> V2 {
        V2::make(self.width, self.height)
    }

    pub fn contains(&self, pos: V2) -> bool {
        pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.height
    }

    pub fn set(&mut self, pos: V2, value: Cell) {
        if self.contains(pos) {
            self.data[pos.y as usize][pos.x as usize] = value;
        }
    }

    pub fn bounds(&self) -> Rectangle {
        Rectangle{pos: V2::make(0, 0), size: self.size()}
    }
}

impl Index<V2> for Level {
    type Output = Cell;
    fn index(&self, idx: V2) -> &Cell {
        if self.contains(idx) {
            return &self.data[idx.y as usize][idx.x as usize];
        }
        &EMPTY_CELL
    }
}