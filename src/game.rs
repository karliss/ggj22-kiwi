use std::io::Write;
use std::default::{self, Default};
use crossterm::{
    cursor::{self, position},
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, poll, read, KeyEvent, KeyModifiers},
    execute,
    queue,
    Result,
    style::{self, Color, Attribute, Stylize},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};

use level::Level;
use ui::UiWidget;

use crate::{level, ui, vecmath};
use crate::level::{Cell, CellColor};
use crate::ui::{UiContext, UiEvent, UiEventType, UiId};
use crate::vecmath::{Rectangle, V2};

pub struct LevelEditor
{
    id: UiId,
    level: Level,
    cursor_pos: V2,
    view_corner: V2,
    need_refresh: bool,
}

fn buffer_size() -> (u16, u16)
{
    if let Ok(size) = crossterm::terminal::size() {
        return size;
    }
    (80, 20)
}

fn get_color(c: CellColor) -> Color {
    match c {
        CellColor::Black => Color::Black,
        CellColor::White => Color::White,
    }
}

impl LevelEditor {
    pub fn new(ui: &mut UiContext) -> LevelEditor {
        let mut result = LevelEditor {
            id: ui.next_id(),
            level: Level::new(50, 50),
            cursor_pos: V2::new(),
            view_corner: V2::new(),
            need_refresh: true,
        };
        result.fill_level();
        result
    }

    fn fill_level(&mut self)
    {
        for y in 0..self.level.height {
            for x in 0..self.level.width {
                let pos = V2::make(x, y);
                let mut cell = Cell::make_empty();
                if x % 2 == 1 {
                    cell.background = CellColor::White;
                    cell.foreGround = CellColor::Black;
                }
                if x % 2 == y % 2 {
                    cell.letter = '#';
                }
                self.level.set(pos, cell);
            }
        }
    }


    fn print_level(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        let size = ui.buffer_size();
        let mut visible_rect = vecmath::Rectangle {
            pos: self.view_corner,
            size: V2::from(size),
        };
        queue!(ui.stdout, cursor::Hide)?;
        for y in 0..size.1 {
            let mut reposition = true;
            for x in 0..size.0 {
                let mut pos = V2::make(x as i32, y as i32);
                pos = pos + self.view_corner;
                let cellData = self.level[pos];
                if reposition {
                    queue!(ui.stdout, cursor::MoveTo(x, y));
                    reposition = false;
                }
                let mut c = cellData.letter;
                if cellData.empty() {
                    c = ' '
                }
                queue!(ui.stdout, style::PrintStyledContent(style::style(c)
                        .with(get_color(cellData.foreGround))
                        .on(get_color(cellData.background))))?;
            }
        }
        if visible_rect.contains(self.cursor_pos) {
            let cpos = self.cursor_pos - self.view_corner;
            queue!(ui.stdout, cursor::MoveTo(cpos.x as u16, cpos.y as u16),
                cursor::SetCursorShape(cursor::CursorShape::UnderScore), cursor::Show)?
        } else {
            queue!(ui.stdout, cursor::Hide)?
        }
        Ok(())
    }
}

impl UiWidget for LevelEditor {
    fn print(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        if self.need_refresh() {
            queue!(ui.stdout, terminal::Clear(terminal::ClearType::All), style::ResetColor)?;
            self.print_level(ui)?;
            ui.stdout.flush()?
        }
        Ok(())
    }

    fn input(&mut self, e: &Event) -> Option<UiEvent> {
        self.mark_refresh(true);
        match e {
            Event::Key(KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE }) => {
                self.cursor_pos = self.cursor_pos + V2::make(0, -1);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE }) => {
                self.cursor_pos = self.cursor_pos + V2::make(0, 1);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::NONE }) => {
                self.cursor_pos = self.cursor_pos + V2::make(-1, 0);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::NONE }) => {
                self.cursor_pos = self.cursor_pos + V2::make(1, 0);
                self.event(UiEventType::Changed)
            }

            Event::Key(KeyEvent { code: KeyCode::Char('w'), modifiers: KeyModifiers::NONE }) => {
                self.view_corner = self.view_corner + V2::make(0, -1);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::NONE }) => {
                self.view_corner = self.view_corner + V2::make(0, 1);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::NONE }) => {
                self.view_corner = self.view_corner + V2::make(-1, 0);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Char('d'), modifiers: KeyModifiers::NONE }) => {
                self.view_corner = self.view_corner + V2::make(1, 0);
                self.event(UiEventType::Changed)
            }
            _ => None
        }
    }

    fn child_widgets(&self) -> Vec<&dyn UiWidget> {
        vec![]
    }

    fn child_widgets_mut(&mut self) -> Vec<&mut dyn UiWidget> {
        vec![]
    }

    fn mark_refresh(&mut self, value: bool) {
        self.need_refresh = value
    }

    fn need_refresh(&self) -> bool {
        self.need_refresh
    }

    fn resize(&mut self, widget_size: &Rectangle) {
        self.need_refresh = true;
    }

    fn get_id(&self) -> UiId {
        return self.id;
    }
}