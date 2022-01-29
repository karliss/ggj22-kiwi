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
use std::io::{ErrorKind, Stdout, Write};
use std::path::Path;
use crossterm::style::style;
use std::thread::current;
use clap::{App, Arg};
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


fn editor_for_file(path: &str) -> std::io::Result<()>
{
    let file_path = Path::new(path);
    let mut stdout = stdout();
    let mut ui = ui::UiContext::create(&mut stdout).unwrap();
    let mut editor = game::LevelEditor::new_from_path(&mut ui, file_path)?;

    enable_raw_mode()?;
    execute!(ui.stdout, crossterm::terminal::EnterAlternateScreen)?;

    let res = ui.run(&mut editor);
    execute!(stdout, crossterm::terminal::LeaveAlternateScreen)?;
    disable_raw_mode()?;
    res
}





fn main() -> Result<()> {
    let matches = App::new("GGJ22-kiwi")
        .author("Kārlis Seņko <karlis3p70l1ij@gmail.com>, Rollick")
        .about("Puzzle game made for GGJ2022")
        .subcommand(
            App::new("edit")
                .about("Edit level file")
                .arg(Arg::new("path")
                    .help("File path for the level, used for loading and saving")
                    .takes_value(true)
                    .required(true))

        )
        .get_matches();

    let mut subcommand = matches.subcommand();
    let result = match subcommand {
        Some(("edit", cmd)) => {
            let path = cmd.value_of("path").ok_or(ErrorKind::Other)?;
            editor_for_file(path);
            Ok(())
        }
        _ =>  {
            enable_raw_mode()?;
            execute!(stdout(), crossterm::terminal::EnterAlternateScreen)?;
            run_empty_editor()?;
            execute!(stdout(), crossterm::terminal::LeaveAlternateScreen)?;
            disable_raw_mode()?;
            Ok(())
        }
    };
    return result
}
