use std::{io::stdout, time::Duration};

use crossterm::{
    cursor::{self, position},
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, self},
    Result,
    queue,
    style,
};
use std::io::{Stdout, Write};
use crossterm::style::style;
use std::thread::current;
use crossterm::terminal::ClearType;

pub enum UiEvent {
    Ok,
    Stopped,
    None,
}

pub trait UiWidget
{
    fn print(&mut self, out: &mut Stdout) -> std::io::Result<()>;
    fn input(&mut self, _e: &Event) -> Option<UiEvent> { return None; }
    fn process(&mut self);
}

#[derive(Copy, Clone)]
pub enum CellColor {
    White,
    Black,
}

#[derive(Copy, Clone)]
struct Cell {
    letter: char,
    background: CellColor,
    foreGround: CellColor,
}

impl Cell {
    fn empty() -> Cell {
        Cell {
            letter: '\0',
            background: CellColor::Black,
            foreGround: CellColor::Black,
        }
    }
}

struct Level {
    data: Vec<Vec<Cell>>,
    width: i32,
    height: i32,
}

impl Level {
    pub fn new(width: i32, height: i32) -> Level {
        return Level {
            data: vec![vec![Cell::empty(); width as usize]; height as usize],
            width,
            height,
        };
    }
}

pub fn buffer_size() -> (u16, u16)
{
    if let Ok(size) = crossterm::terminal::size() {
        return size;
    }
    (80, 20)
}

pub fn test_print(stdout: &mut Stdout)
{
    let sz = buffer_size();
    queue!(stdout, terminal::Clear(ClearType::All), style::ResetColor, cursor::MoveTo(0, 0));
    for y in 0..sz.1 {
        if (y > 0) {
            queue!(stdout, cursor::MoveToNextLine(1));
        }
        for x in 0..sz.0 {
            if x == 0 || y == 0 || x + 1 == sz.0 || y + 1 == sz.1 {
                queue!(stdout, style::Print('#'));
            } else {
                let c = if x % 2 == y % 2 {
                    ' '
                } else {
                    'X'
                };
                queue!(stdout, style::Print(c));
            }
        }
    }
    stdout.flush().unwrap();
}

fn main() -> Result<()> {
    println!("Starting game");
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnableMouseCapture, crossterm::terminal::EnterAlternateScreen);

    execute!(stdout, DisableMouseCapture, crossterm::terminal::LeaveAlternateScreen);
    disable_raw_mode();
    test_print(&mut stdout);
    Ok(())
}
