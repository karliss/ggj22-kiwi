use std::ops::{Add, Sub};
use crossterm::terminal::size;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct V2 {
    pub x: i32,
    pub y: i32,
}

impl V2 {
    pub fn new() -> V2 {
        V2 { x: 0, y: 0 }
    }

    pub fn from(size: (u16, u16)) -> V2 {
        V2 {
            x: size.0 as i32,
            y: size.1 as i32
        }
    }

pub fn make(x: i32, y: i32) -> V2 {
    V2 { x, y }
}


}

impl Add for V2 {
    type Output = V2;
    fn add(self, other: V2) -> V2 {
        V2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for V2 {
    type Output = V2;
    fn sub(self, other: V2) -> V2 {
        V2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Rectangle {
    pub pos: V2,
    pub size: V2,
}

impl Rectangle {
    pub fn bottom_right(&self) -> V2 {
        self.pos + self.size + V2::make(-1, -1)
    }

    pub fn left(&self) -> i32 {
        self.pos.x
    }

    pub fn right(&self) -> i32 {
        self.pos.x + self.size.x - 1
    }

    pub fn top(&self) -> i32 {
        self.pos.y
    }
    pub fn bottom(&self) -> i32 {
        self.pos.y + self.size.y - 1
    }

    pub fn bottom_left(&self) -> V2 {
        V2::make(self.left(), self.bottom())
    }

    pub fn top_right(&self) -> V2 {
        V2::make(self.right(), self.top())
    }

    pub fn grow(&self, size: i32) -> Rectangle {
        Rectangle {
            pos: self.pos - V2::make(size, size),
            size: self.size + V2::make(2 * size, 2 * size),
        }
    }

    pub fn width(&self) -> i32 {
        self.size.x
    }

    pub fn height(&self) -> i32 {
        self.size.y
    }

    pub fn contains(&self, pos: V2) -> bool {
        pos.x >= self.left() && pos.x <= self.right() &&
            pos.y >= self.top() && pos.y <= self.bottom()
    }

    pub fn normalized(&self) -> Rectangle {
        if self.size.x > 0 && self.size.y > 0 {
            return *self;
        }
        let mut r2 = *self;
        if r2.size.x <= 0 {
            let l = self.right();
            let r = self.left();
            r2.pos.x = l;
            r2.size.x = r - l + 1;
        }
        if r2.size.y <= 0 {
            let t = r2.bottom();
            let b = r2.top();
            r2.pos.y = t;
            r2.size.y = b - t + 1;
        }
        r2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2_add() {
        assert_eq!(V2::make(0, 0) + V2::make(0, 0), V2::make(0, 0));
        assert_eq!(V2::make(0, 10) + V2::make(0, 0), V2::make(0, 10));
        assert_eq!(V2::make(10, 0) + V2::make(0, 0), V2::make(10, 0));
        assert_eq!(V2::make(0, 10) + V2::make(10, 0), V2::make(10, 10));
        assert_eq!(V2::make(1, 2) + V2::make(4, 8), V2::make(5, 10));
    }

    #[test]
    fn rect_sides() {
        let r = Rectangle {
            pos: V2::make(1, 2),
            size: V2::make(3, 4),
        };
        assert_eq!(r.top(), 2);
        assert_eq!(r.left(), 1);
        assert_eq!(r.bottom(), 5);
        assert_eq!(r.right(), 3);
        assert_eq!(r.bottom_right(), V2::make(3, 5));
        assert_eq!(r.bottom_left(), V2::make(1, 5));
        assert_eq!(r.top_right(), V2::make(3, 2));
    }


    #[test]
    fn inside() {
        let r = Rectangle {
            pos: V2::make(1, 2),
            size: V2::make(3, 4),
        };
        assert_eq!(true, r.contains(V2::make(1, 2)));
        assert_eq!(false, r.contains(V2::make(0, 2)));
        assert_eq!(true, r.contains(r.bottom_right()));
        assert_eq!(false, r.contains(r.bottom_right() + V2::make(1, 0)));
        assert_eq!(false, r.contains(r.bottom_right() + V2::make(0, 1)));
    }
}
