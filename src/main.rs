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
use crate::game::MultiLevelRunner;
use crate::level::LevelList;

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
    execute!(ui.stdout, crossterm::terminal::EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;


    let res = ui.run(&mut editor);
    ui.restore_normal();
    res
}

fn good_level_path(path: &Path) -> bool {
    let pb = path.join("levels");
    return pb.exists() && pb.is_dir();
}

fn play_levels() -> std::io::Result<()>
{
    let mut levels = LevelList{
        files: vec!["levels/l1".into(), "levels/l2".into()]
    };


    if let Ok(exe_path) = std::env::current_exe() {
        let folder = exe_path.parent().unwrap();
        let mut top_folder = folder.to_owned();
        let folder_2 = folder.join("../../");
        if good_level_path(folder) {
            //top_folder = folder.to_owned();
        } else if good_level_path(&folder_2) {
            top_folder = folder_2;
        } else {
            eprintln!("Can't find level data");
            return Err(std::io::ErrorKind::Other.into());
        }

        let list_reader = std::fs::File::open(top_folder.join("levels/list.yaml")).map_err(|e| {
            eprintln!("Failed to load level list 'levels/list.yaml': {}", e);
            e
        })?;
        let yaml: serde_yaml::Result<LevelList> = serde_yaml::from_reader(list_reader);

        match yaml {
            Ok(res) => {
                levels = res;
            }
            Err(e) => {
                eprintln!("Failed to load level list 'levels/list.yaml': {}", e);
                return Err(ErrorKind::Other.into());
            }
        }

        for path in &mut levels.files {
            let p2 = top_folder.join(&path);
            path.replace_range(.., p2.to_str().unwrap());
        }
    }
    let mut stdout = stdout();
    let mut ui = ui::UiContext::create(&mut stdout).unwrap();
    let mut runner = MultiLevelRunner::new(&mut ui, levels);

    enable_raw_mode()?;
    execute!(ui.stdout, crossterm::terminal::EnterAlternateScreen)?;

    runner.start_next_level();
    let res = ui.run(&mut runner);
    ui.restore_normal();
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
            play_levels()
        }
    };
    return result
}
