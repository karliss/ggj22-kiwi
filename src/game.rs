use std::io::{Error, ErrorKind, Write};
use std::default::{self, Default};
use std::fs::File;
use std::path::{is_separator, Path};
use crossterm::{
    cursor::{self, position},
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, poll, read, KeyEvent, KeyModifiers},
    execute,
    queue,
    style::{self, Color, Attribute, Stylize},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use crossterm::terminal::{Clear, ClearType};

use level::Level;
use ui::UiWidget;

use crate::{level, ui, vecmath};
use crate::level::{Cell, CellColor, Trigger};
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
    path: Option<Box<std::path::Path>>,
    paintMode: PaintMode,
    test_runer: LevelRunner,
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
    ErrorMessage,
    Paint,
    SetMarkers,
    Play
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum PaintMode {
    BlackBackgroundNormal,
    WhiteBackgroundNormal,
    Invert,
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
            path: None,
            paintMode: PaintMode::WhiteBackgroundNormal,
            test_runer: LevelRunner::new(ui),
        };
        result.fill_level();
        result
    }

    pub fn new_from_path(ui: &mut UiContext, path: &Path) -> std::io::Result<LevelEditor> {
        let mut result = LevelEditor::new(ui);
        result.path = Some(path.into());
        if path.is_file() {
            let file = std::fs::File::open(path)?;
            let yaml: serde_yaml::Result<Level> = serde_yaml::from_reader(file);
            match yaml {
                Ok(res) => {
                    result.level = res
                }
                Err(e) => {
                    eprintln!("Failed to load level '{}': {}", path.to_string_lossy(), e);
                    return Err(Error::from(ErrorKind::InvalidData));
                }
            }
        } else {
            result.fill_level();
        }
        Ok(result)
    }

    pub fn save(&self) -> std::io::Result<()> {
        match &self.path {
            Some(path) => {
                let mut ofile = File::create(path)?;
                serde_yaml::to_writer(&ofile, &self.level).map_err(|e|
                    {
                        eprintln!("Can't save: {}", e);
                        ErrorKind::InvalidData
                    }
                )?;
                Ok(())
            }
            None => {
                eprintln!("Can't save, no path specified");
                Err(Error::from(ErrorKind::Other))
            }
        }
    }

    fn fill_level(&mut self)
    {
        for y in 0..self.level.height {
            for x in 0..self.level.width {
                let pos = V2::make(x, y);
                let mut cell = Cell::make_empty();
                cell.background = CellColor::Black;
                cell.foreground = CellColor::White;
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
        queue!(ui.stdout, cursor::MoveTo(0, size.1 - 2),
                style::ResetColor)?;
        queue!(ui.stdout, style::Print(format!("mode: {:?} ", self.mode)))?;
        match self.mode {
            EditorMode::View => {
                queue!(ui.stdout, style::Print(format!(" F2: view F3: text mode F4: corner F5: paint F6: markers F8: test F9: save [shift]+F8 test here " )))?;
            }
            EditorMode::Paint => {
                queue!(ui.stdout, style::Print(format!(" color: {:?} ", self.paintMode)))?;
                queue!(ui.stdout, style::Print(format!(" [ZXC]->colors, [SPACE]->paint here, [WASD] paint in direction")))?;
            }
            EditorMode::SetMarkers => {
                for trigger in &self.level.triggers {
                    if trigger.pos == self.cursor_pos {
                        queue!(ui.stdout, style::Print(format!(" here: {}", trigger.id)))?;
                    }
                }
                queue!(ui.stdout, style::Print(format!(" [z]->level start [xc]->exits")))?;
            }
            _ => {}
        }
        queue!(ui.stdout, Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    fn print_rect(&mut self, ui: &mut UiContext, rect: Rectangle, c: char) {
        let mut visible_rect = self.get_view_rect();
        for y in rect.top()..=rect.bottom() {
            for x in rect.left()..=rect.right() {
                let p = V2::make(x, y);
                if visible_rect.contains(p) {
                    let p2 = p - self.view_corner;

                    ui.goto(p2);
                    queue!(ui.stdout, style::PrintStyledContent(style::style(' ')
                        .with(Color::Black)
                        .on(Color::DarkRed)));
                }
            }
        }
    }

    fn print_at(&self, ui: &mut UiContext, ps: V2, c: char, tColor: Option<Color>, bColor: Option<Color>) -> std::io::Result<()> {
        let visible_rect = self.get_view_rect();
        if !visible_rect.contains(ps) {
            return Ok(())
        }

        ui.goto(ps - self.view_corner);
        let mut message = style::style(c);

        let cell = self.level[ps];
        if let Some(color) = tColor {
            message = message.with(color);
        }
        if let Some(color) = bColor {
            message = message.on(color);
        } else {
            message = message.on(get_color(cell.background));
        }
        queue!(ui.stdout, style::PrintStyledContent(message))?;
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
                    queue!(ui.stdout, cursor::MoveTo(x, y))?;
                    reposition = false;
                }
                let mut c = cell.letter;
                if cell.empty() {
                    c = ' '
                }
                queue!(ui.stdout, style::PrintStyledContent(style::style(c)
                        .with(get_color(cell.foreground))
                        .on(get_color(cell.background))))?;
            }
        }
        self.print_rect(ui, Rectangle { pos: V2::make(-1, -1), size: V2::make(self.level.width + 2, 1) }, ' ');
        self.print_rect(ui, Rectangle { pos: V2::make(-1, self.level.height), size: V2::make(self.level.width + 2, 1) }, ' ');
        self.print_rect(ui, Rectangle { pos: V2::make(-1, -1), size: V2::make(1, self.level.height + 2) }, ' ');
        self.print_rect(ui, Rectangle { pos: V2::make(self.level.width, -1), size: V2::make(1, self.level.height + 2) }, ' ');

        self.print_at(ui, self.level.p0, '$', Some(Color::DarkGreen), None);
        for trigger in &self.level.triggers {
            self.print_at(ui, trigger.pos, '?', Some(Color::Blue), None);
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

    fn switch_to_err(&mut self, ui: &mut UiContext) -> std::io::Result<()>
    {
        self.mode = EditorMode::ErrorMessage;
        execute!(ui.stdout, crossterm::terminal::LeaveAlternateScreen)?;
        disable_raw_mode()
    }

    fn show_err(&mut self, ui: &mut UiContext, text: &str) -> std::io::Result<()>
    {
        self.switch_to_err(ui)?;
        execute!(ui.stdout, cursor::MoveToNextLine(1))?;
        eprintln!("{}", text);
        Ok(())
    }

    fn switch_to_edit(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        self.mode = EditorMode::View;
        enable_raw_mode()?;
        execute!(ui.stdout, crossterm::terminal::EnterAlternateScreen)
    }

    fn start_level_test(&mut self, pos: V2) {
        self.test_runer.level = self.level.clone();
        self.test_runer.pos = pos;
        self.mode = EditorMode::Play;
    }

    fn start_level_test_normal(&mut self) {
        self.start_level_test(self.level.p0);
    }

    fn paint_cell_here(&mut self, pos: V2) {
        let mut cell = self.level[pos];
        match self.paintMode {
            PaintMode::BlackBackgroundNormal => {
                cell.background = CellColor::Black;
                cell.foreground = CellColor::White;
            }
            PaintMode::WhiteBackgroundNormal => {
                cell.background = CellColor::White;
                cell.foreground = CellColor::Black;
            }
            PaintMode::Invert => {
                if cell.background == CellColor::Black {
                    cell.background = CellColor::White;
                    cell.foreground = CellColor::Black;
                } else if cell.background == CellColor::White {
                    cell.background = CellColor::Black;
                    cell.foreground = CellColor::White;
                }
            }
        }
        self.level.set(pos, cell);
    }

    fn move_and_paint(&mut self, dir: V2) {
        self.cursor_pos = self.cursor_pos + dir;
        self.paint_cell_here(self.cursor_pos);
    }

    fn handle_test_play(&mut self, ev: Option<UiEvent>) -> Option<UiEvent> {
        match ev {
            Some(UiEvent{id: _, e : UiEventType::Canceled}) |
            Some(UiEvent{id: _, e : UiEventType::Ok}) |
            Some(UiEvent{id: _, e : UiEventType::Result(_)}) => {
                self.mode = EditorMode::View;
                self.event(UiEventType::Changed)
            }
            _ => ev
        }
    }
}

impl UiWidget for LevelEditor {
    fn print(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        if self.need_refresh() {
            match self.mode {
                EditorMode::ErrorMessage => {}
                EditorMode::Play => {
                    self.test_runer.print(ui);
                }
                _ => {
                    queue!(ui.stdout, terminal::Clear(terminal::ClearType::All), style::ResetColor)?;
                    self.print_level(ui)?;
                    ui.stdout.flush()?
                }
            }
        }
        Ok(())
    }

    fn input(&mut self, e: &Event, ui: &mut UiContext) -> Option<UiEvent> {
        self.mark_refresh(true);
        match self.mode {
            EditorMode::ErrorMessage => {
                // press any key to exit error mode
                return match e {
                    Event::Key(_) => {
                        self.switch_to_edit(ui);
                        self.event(UiEventType::Changed)
                    }
                    _ => None
                };
            }
            EditorMode::Play => {
                match e {
                    Event::Key(KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE }) => {
                        self.mode = EditorMode::View;
                        return self.event(UiEventType::Changed)
                    }
                    _ => {
                        let r = self.test_runer.input(e, ui);
                        return self.handle_test_play(r);
                    }
                }
            }
            _ => {}
        }
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
            Event::Key(KeyEvent { code: KeyCode::F(5), modifiers: KeyModifiers::NONE }) => {
                self.mode = EditorMode::Paint;
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::F(6), modifiers: KeyModifiers::NONE }) => {
                self.mode = EditorMode::SetMarkers;
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::F(8), modifiers: KeyModifiers::NONE }) => {
                self.start_level_test_normal();
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::F(8), modifiers: KeyModifiers::SHIFT }) => {
                self.start_level_test(self.cursor_pos);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::F(9), modifiers: KeyModifiers::NONE }) => {
                self.switch_to_err(ui);
                match self.save() {
                    Ok(_) => {
                        self.show_err(ui, "Saved!");
                    }
                    Err(_) => {
                        self.show_err(ui, "Failed to save");
                    }
                }


                self.event(UiEventType::Changed)
            }
            _ => None
        };
        if v.is_some() { return v; }

        if self.mode != EditorMode::WriteText && self.mode != EditorMode::Paint {
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
                _ => None
            };
            if v.is_some() {
                return v;
            }
        }


        let v = match self.mode {
            EditorMode::View => {
                match e {
                    Event::Key(KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::NONE }) => {
                        self.mode = EditorMode::WriteText;
                        self.wrap_pos = self.cursor_pos;
                        self.event(UiEventType::Changed)
                    }
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
                    Event::Key(KeyEvent { code: KeyCode::Char('h'), modifiers: KeyModifiers::CONTROL }) => {
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
            EditorMode::Paint => {
                match e {
                    Event::Key(KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE }) => {
                        self.mode = EditorMode::View;
                        self.event(UiEventType::Changed)
                    }

                    Event::Key(KeyEvent { code: KeyCode::Char('w'), modifiers: KeyModifiers::NONE }) => {
                        self.move_and_paint(V2::make(0, -1));
                        self.event(UiEventType::Changed)
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::NONE }) => {
                        self.move_and_paint(V2::make(0, 1));
                        self.event(UiEventType::Changed)
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::NONE }) => {
                        self.move_and_paint(V2::make(-1, 0));
                        self.event(UiEventType::Changed)
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char('d'), modifiers: KeyModifiers::NONE }) => {
                        self.move_and_paint(V2::make(1, 0));
                        self.event(UiEventType::Changed)
                    }

                    Event::Key(KeyEvent { code: KeyCode::Char('z'), modifiers: KeyModifiers::NONE }) => {
                        self.paintMode = PaintMode::BlackBackgroundNormal;
                        self.event(UiEventType::Changed)
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char('x'), modifiers: KeyModifiers::NONE }) => {
                        self.paintMode = PaintMode::WhiteBackgroundNormal;
                        self.event(UiEventType::Changed)
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::NONE }) => {
                        self.paintMode = PaintMode::Invert;
                        self.event(UiEventType::Changed)
                    }

                    Event::Key(KeyEvent { code: KeyCode::Char(' '), modifiers: KeyModifiers::NONE }) => {
                        self.paint_cell_here(self.cursor_pos);
                        self.event(UiEventType::Changed)
                    }
                    _ => None
                }
            }
            EditorMode::SetMarkers => {
                match e {
                    Event::Key(KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE }) => {
                        self.mode = EditorMode::View;
                        self.event(UiEventType::Changed)
                    }

                    Event::Key(KeyEvent { code: KeyCode::Char('z'), modifiers: KeyModifiers::NONE }) => {
                        self.level.p0 = self.cursor_pos;
                        self.event(UiEventType::Changed)
                    }

                    Event::Key(KeyEvent { code: KeyCode::Backspace, modifiers: KeyModifiers::NONE }) |
                    Event::Key(KeyEvent { code: KeyCode::Char('h'), modifiers: KeyModifiers::CONTROL }) => {
                        self.level.triggers.retain(|trigger| trigger.pos != self.cursor_pos);
                        self.event(UiEventType::Changed)
                    }

                    Event::Key(KeyEvent { code: KeyCode::Char('x'), modifiers: KeyModifiers::NONE }) => {
                        self.level.triggers.retain(|trigger| trigger.pos != self.cursor_pos);
                        self.level.triggers.push(Trigger {
                            pos: self.cursor_pos,
                            id: "exit1".into(),
                        });
                        self.event(UiEventType::Changed)
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::NONE }) => {
                        self.level.triggers.retain(|trigger| trigger.pos != self.cursor_pos);
                        self.level.triggers.push(Trigger {
                            pos: self.cursor_pos,
                            id: "exit2".into(),
                        });
                        self.event(UiEventType::Changed)
                    }
                    _ => None
                }
            }
            // already handled
            EditorMode::ErrorMessage => None,
            EditorMode::Play => None
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

pub struct LevelRunner {
    pub level: Level,
    pub pos: V2,
    view_corner: V2,
    pub need_refresh: bool,
    id: UiId,
}

fn is_base_color(c: CellColor) -> bool {
    return c == CellColor::Black || c == CellColor::White;
}

impl LevelRunner {
    pub fn new(ui: &mut UiContext) -> LevelRunner {
        LevelRunner {
            id: ui.next_id(),
            level: Level::new(10, 10),
            pos: V2::make(2, 2),
            view_corner: V2::make(0, 0),
            need_refresh: true,
        }
    }
    pub fn new_with_level(ui: &mut UiContext, level: &Level) -> LevelRunner {
        let mut res = LevelRunner::new(ui);
        res.level = level.clone();
        res.pos = level.p0;

        res
    }


    fn get_view_rect(&self) -> Rectangle {
        let size = buffer_size();
        vecmath::Rectangle {
            pos: self.view_corner,
            size: V2::from(size),
        }
    }

    fn print_level(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        let size = ui.buffer_size();
        let mut visible_rect = self.get_view_rect();
        let level_rect = self.level.bounds();
        queue!(ui.stdout, cursor::Hide)?;
        for y in 0..size.1 {
            let mut reposition = true;
            for x in 0..size.0 {
                let mut pos = V2::make(x as i32, y as i32);

                pos = pos + self.view_corner;
                if !level_rect.contains(pos) {
                    continue;
                    reposition = true;
                }

                let cell = self.level[pos];
                if reposition {
                    queue!(ui.stdout, cursor::MoveTo(x, y))?;
                    reposition = false;
                }
                let mut c = cell.letter;
                if cell.empty() {
                    c = ' '
                }
                queue!(ui.stdout, style::PrintStyledContent(style::style(c)
                        .with(get_color(cell.foreground))
                        .on(get_color(cell.background))))?;
            }
        }
        if visible_rect.contains(self.pos) {
            ui.goto(self.pos - self.view_corner);
            let cell = self.level[self.pos];
            queue!(ui.stdout, style::PrintStyledContent(style::style('@')
                        .with(get_color(cell.foreground))
                        .on(get_color(cell.background))))?;
        }
        /*self.print_rect(ui, Rectangle { pos: V2::make(-1, -1), size: V2::make(self.level.width + 2, 1) }, ' ');
        self.print_rect(ui, Rectangle { pos: V2::make(-1, self.level.height), size: V2::make(self.level.width + 2, 1) }, ' ');
        self.print_rect(ui, Rectangle { pos: V2::make(-1, -1), size: V2::make(1, self.level.height + 2) }, ' ');
        self.print_rect(ui, Rectangle { pos: V2::make(self.level.width, -1), size: V2::make(1, self.level.height + 2) }, ' ');*/
        Ok(())
    }

    fn keep_cursor_in_view(&mut self) -> bool {
        let PADDING = 5;
        let mut view = self.get_view_rect();
        view = view.grow(-PADDING);
        if view.contains(self.pos) {
            return false;
        }
        let size = V2::from(buffer_size());
        let pos = self.pos;
        let mut moved = false;
        if pos.x < view.left() {
            self.view_corner.x = pos.x - PADDING;
            moved = true;
        }
        if pos.x > view.right() {
            self.view_corner.x = pos.x + PADDING - size.x;
            moved = true;
        }
        if pos.y < view.top() {
            self.view_corner.y = pos.y - PADDING;
            moved = true;
        }
        if pos.y > view.bottom() {
            self.view_corner.y = pos.y + PADDING - size.y;
            moved = true;
        }
        return moved;
    }



    fn walk(&mut self, dir: V2) {
        let target = self.pos + dir;
        let bounds = self.level.bounds();
        if !bounds.contains(target) {
            return;
        }
        let here = self.level[self.pos];
        let target_cell = self.level[target];
        let next_cell = self.level[target + dir];
        if target_cell.background == here.background {
            if target_cell.empty() {
                self.pos = target;
                return;
            }
        } else {
            if is_base_color(here.background) && is_base_color(target_cell.background) && target_cell.letter == '@' {
                let mut target2 = target_cell;
                target2.letter = ' ';
                let mut here2 = here;
                here2.letter = '@';
                self.level.set(self.pos, here2);
                self.level.set(target, target2);
                self.pos = target;
            }
        }



    }

    fn move_with_ui(&mut self, dir: V2, ui: &mut UiContext)  {
        self.walk(dir);
        self.keep_cursor_in_view();
        self.mark_refresh(true);
    }

}

impl UiWidget for LevelRunner {
    fn print(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        if self.need_refresh {
            queue!(ui.stdout,Clear(ClearType::All));
            self.print_level(ui)?;
            ui.stdout.flush();
        }
        Ok(())
    }

    fn input(&mut self, e: &Event, ui: &mut UiContext) -> Option<UiEvent> {
        match e {
            Event::Key(KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE }) => {
                self.move_with_ui(V2::make(0, -1), ui);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE }) => {
                self.move_with_ui(V2::make(0, 1), ui);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::NONE }) => {
                self.move_with_ui(V2::make(-1, 0), ui);
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::NONE }) => {
                self.move_with_ui(V2::make(1, 0), ui);
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
        self.need_refresh = value;
    }

    fn need_refresh(&self) -> bool {
        self.need_refresh
    }

    fn get_id(&self) -> UiId {
        return self.id
    }

    fn update(&mut self) -> Option<UiEvent> {
        if self.keep_cursor_in_view() {
            self.mark_refresh(true);
            return self.event(UiEventType::Changed);
        }
        None
    }
}