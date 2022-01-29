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

pub mod vecmath;
pub mod ui;
pub mod game;
pub mod level;


fn run_empty_editor() -> std::io::Result<()>
{
    let mut stdout = stdout();
    let mut ui = ui::UiContext::create(&mut stdout).unwrap();

    let mut editor = game::LevelEditor::new(&mut ui);
    ui.run(&mut editor)?;
    Ok(())
}

fn main() -> Result<()> {
    println!("Starting game");
    enable_raw_mode()?;
    execute!(stdout(), crossterm::terminal::EnterAlternateScreen)?;

    run_empty_editor();

    execute!(stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
