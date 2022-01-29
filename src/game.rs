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
use crossterm::terminal::{Clear, ClearType};

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
    wrap_pos: V2,
    need_refresh: bool,
    mode: EditorMode,
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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum EditorMode {
    View,
    WriteText,
}

impl LevelEditor {
    pub fn new(ui: &mut UiContext) -> LevelEditor {
        let mut result = LevelEditor {
            id: ui.next_id(),
            level: Level::new(50, 50),
            cursor_pos: V2::new(),
            view_corner: V2::new(),
            wrap_pos: V2::new(),
            need_refresh: true,
            mode: EditorMode::View,
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
                if x % 20 == 1 {
                    cell.background = CellColor::White;
                    cell.foreGround = CellColor::Black;
                } else {
                    cell.background = CellColor::Black;
                    cell.foreGround = CellColor::White;
                }
                self.level.set(pos, cell);
            }
        }
    }


    fn get_view_rect(&self) -> Rectangle {
        let size = buffer_size();
        vecmath::Rectangle {
            pos: self.view_corner,
            size: V2::from(size),
        }
    }

    fn print_status_bar(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        let size = ui.buffer_size();
        queue!(ui.stdout, cursor::MoveTo(0, size.1 - 1),
                style::ResetColor)?;
        queue!(ui.stdout, style::Print(format!("mode: {:?}", self.mode)))?;
        queue!(ui.stdout, Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    fn print_level(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        let size = ui.buffer_size();
        let mut visible_rect = self.get_view_rect();
        queue!(ui.stdout, cursor::Hide)?;
        for y in 0..size.1 {
            let mut reposition = true;
            for x in 0..size.0 {
                let mut pos = V2::make(x as i32, y as i32);
                pos = pos + self.view_corner;
                let cell = self.level[pos];
                if reposition {
                    queue!(ui.stdout, cursor::MoveTo(x, y));
                    reposition = false;
                }
                let mut c = cell.letter;
                if cell.empty() {
                    c = ' '
                }
                queue!(ui.stdout, style::PrintStyledContent(style::style(c)
                        .with(get_color(cell.foreGround))
                        .on(get_color(cell.background))))?;
            }
        }
        self.print_status_bar(ui)?;

        if visible_rect.contains(self.cursor_pos) {
            let cpos = self.cursor_pos - self.view_corner;
            queue!(ui.stdout, cursor::MoveTo(cpos.x as u16, cpos.y as u16),
                cursor::SetCursorShape(cursor::CursorShape::UnderScore), cursor::Show)?
        } else {
            queue!(ui.stdout, cursor::Hide)?
        }
        Ok(())
    }

    fn keep_cursor_in_view(&mut self) {
        let PADDING = 2;
        let mut view = self.get_view_rect();
        view = view.grow(-PADDING);
        if view.contains(self.cursor_pos) {
            return;
        }
        let size = V2::from(buffer_size());
        let pos = self.cursor_pos;
        if pos.x < view.left() {
            self.view_corner.x = pos.x - PADDING;
        }
        if pos.x > view.right() {
            self.view_corner.x = pos.x + PADDING - size.x;
        }
        if pos.y < view.top() {
            self.view_corner.y = pos.y - PADDING;
        }
        if pos.y > view.bottom() {
            self.view_corner.y = pos.y + PADDING - size.y;
        }
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
        let v = match e {
            Event::Key(KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE }) => {
                self.cursor_pos = self.cursor_pos + V2::make(0, -1);
                self.keep_cursor_in_view();
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE }) => {
                self.cursor_pos = self.cursor_pos + V2::make(0, 1);
                self.keep_cursor_in_view();
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::NONE }) => {
                self.cursor_pos = self.cursor_pos + V2::make(-1, 0);
                self.keep_cursor_in_view();
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::NONE }) => {
                self.cursor_pos = self.cursor_pos + V2::make(1, 0);
                self.keep_cursor_in_view();
                self.event(UiEventType::Changed)
            }

            Event::Key(KeyEvent { code: KeyCode::F(2), modifiers: KeyModifiers::NONE }) => {
                self.mode = EditorMode::View;
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::F(3), modifiers: KeyModifiers::NONE }) => {
                self.mode = EditorMode::WriteText;
                self.wrap_pos = self.cursor_pos;
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::F(4), modifiers: KeyModifiers::NONE }) => {
                self.wrap_pos = self.cursor_pos;
                self.event(UiEventType::Changed)
            }
            _ => None
        };
        if v.is_some() { return v; }

        if self.mode != EditorMode::WriteText {
            let v = match e {
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

                Event::Key(KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::NONE }) => {
                    self.mode = EditorMode::WriteText;
                    self.wrap_pos = self.cursor_pos;
                    self.event(UiEventType::Changed)
                }
                _ => None
            };
            if v.is_some() {
                return v;
            }
        }

        let v = match self.mode {
            EditorMode::View => {
                match e {
                    _ => None
                }
            }
            EditorMode::WriteText => {
                match e {
                    Event::Key(KeyEvent {
                                   code: KeyCode::Enter, modifiers: KeyModifiers::NONE
                               }) => {
                        self.cursor_pos.x = self.wrap_pos.x;
                        self.cursor_pos.y += 1;
                        self.event(UiEventType::Changed)
                    }
                    Event::Key(KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE }) => {
                        self.mode = EditorMode::View;
                        self.event(UiEventType::Changed)
                    }
                    Event::Key(KeyEvent { code: KeyCode::Backspace, modifiers: KeyModifiers::NONE }) |
                    Event::Key(KeyEvent { code: KeyCode::Char('h'), modifiers: KeyModifiers::CONTROL })=> {
                        self.cursor_pos.x -= 1;
                        let mut data = self.level[self.cursor_pos];
                        data.letter = '\0';
                        self.level.set(self.cursor_pos, data);
                        self.event(UiEventType::Changed)
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char(c), modifiers: m }) if
                    !c.is_control() && (m == &KeyModifiers::NONE || m == &KeyModifiers::SHIFT) => {
                        let mut data = self.level[self.cursor_pos];
                        data.letter = *c;
                        self.level.set(self.cursor_pos, data);
                        self.cursor_pos.x += 1;
                        self.event(UiEventType::Changed)
                    }
                    _ => None
                }
            }
        };
        None
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