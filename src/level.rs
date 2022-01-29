use std::ops::{Index, IndexMut};
use crate::vecmath::V2;

#[derive(Copy, Clone)]
pub enum CellColor {
    White,
    Black,
}

#[derive(Copy, Clone)]
pub struct Cell {
    pub letter: char,
    pub background: CellColor,
    pub foreGround: CellColor,
}

impl Cell {
    pub fn empty(&self) -> bool { self.letter == '\0' }
}

static EMPTY_CELL: Cell = Cell {
    letter: '\0',
    background: CellColor::Black,
    foreGround: CellColor::Black,
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

pub struct Level {
    pub data: Vec<Vec<Cell>>,
    pub width: i32,
    pub height: i32,
}

impl Level {
    pub fn new(width: i32, height: i32) -> Level {
        return Level {
            data: vec![vec![Cell::make_empty(); width as usize]; height as usize],
            width,
            height,
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