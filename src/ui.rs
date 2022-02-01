use std::any::Any;
use std::char::decode_utf16;
use std::io::Stdout;
use std::io::Write;
use std::num::NonZeroU64;
use std::thread;
use std::time::Duration;
use clap::ErrorKind;

use crossterm::{
    cursor::{self, position},
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, self},
    Result,
    queue,
    style,
};
use crossterm::event::{KeyEvent, KeyModifiers};
use crossterm::style::Attribute;

use crate::vecmath::*;

pub enum UiEventType {
    Ok,
    Canceled,
    Result(Box<dyn Any>),
    Changed,
    None,
}

pub struct UiEvent {
    pub id: UiId,
    pub e: UiEventType,
}

pub trait UiWidget {
    fn print(&mut self, ui: &mut UiContext) -> std::io::Result<()>;
    fn input(&mut self, _e: &Event, ui: &mut UiContext) -> Option<UiEvent> {
        self.mark_refresh(true);
        return None;
    }
    fn child_widgets(&self) -> Vec<&dyn UiWidget>;
    // { Vec::new() }
    fn child_widgets_mut(&mut self) -> Vec<&mut dyn UiWidget>;
    // { Vec::new() }
    fn mark_refresh(&mut self, value: bool);
    fn need_refresh(&self) -> bool;

    fn resize(&mut self, widget_size: &Rectangle) {
        self.mark_refresh(true);
    }
    fn get_id(&self) -> UiId;
    fn event(&self, e: UiEventType) -> Option<UiEvent> {
        Some(UiEvent {
            id: self.get_id(),
            e,
        })
    }
    fn update(&mut self) -> Option<UiEvent> {
        for child in self.child_widgets_mut() {
            child.update();
        }
        None
    }
}

pub trait DataWidget<T>: UiWidget {
    fn print_data(&mut self, ui: &mut UiContext, data: T) -> std::io::Result<()>;
}

pub const DEFAULT_WINDOW_SIZE: Rectangle = Rectangle {
    pos: V2 { x: 0, y: 0 },
    size: V2 { x: 80, y: 24 },
};

pub struct Menu {
    id: UiId,
    entries: Vec<String>,
    cancelable: bool,
    selected: usize,
    result: Option<Option<usize>>,
    need_refresh: bool,
}

impl Menu {
    pub fn new(entries: Vec<String>, cancelable: bool, context: &mut UiContext) -> Menu {
        assert!(entries.len() > 0);
        Menu {
            id: context.next_id(),
            entries,
            cancelable,
            selected: 0,
            result: None,
            need_refresh: true,
        }
    }

    fn get_selected(&self) -> usize {
        return self.selected;
    }
    fn result(&self) -> Option<Option<usize>> {
        return self.result;
    }
}

impl UiWidget for Menu {
    fn print(&mut self, ui: &mut UiContext) -> ::std::io::Result<()> {
        //TODO: respect size
        if !self.need_refresh() {
            return Ok(());
        }
        queue!(
            ui.stdout,
            style::Print("{}{}{}"),
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(1, 1),
            cursor::Hide
        )?;
        for (i, entry) in self.entries.iter().enumerate() {
            if self.selected != i {
                queue!(ui.stdout, style::Print(format!("({}) {}\r\n", i, entry)))?;
            } else {
                queue!(ui.stdout, style::Print(format!("> ({}) {}\r\n", i, entry)))?;
            }
        }
        Ok(())
    }

    fn input(&mut self, e: &Event, ui: &mut UiContext) -> Option<UiEvent> {
        self.mark_refresh(true);
        match e {
            Event::Key(KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE }) => {
                self.selected += 1;
                if self.selected >= self.entries.len() {
                    self.selected = 0;
                }
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE }) => {
                self.selected = if self.selected > 0 {
                    self.selected - 1
                } else {
                    self.entries.len() - 1
                };
                self.event(UiEventType::Changed)
            }
            Event::Key(KeyEvent { code: KeyCode::Char(number @ '0'..='9'), modifiers: KeyModifiers::NONE }) =>
            //Event::Key(Key::Char(number @ '0'...'9')) =>
                {
                    let n = number.to_digit(10).unwrap() as usize;
                    if n < self.entries.len() {
                        self.selected = n;
                    }
                    self.event(UiEventType::Changed)
                }
            event if event == &Event::Key(KeyCode::Char('\n').into()) => {
                self.result = Some(Some(self.selected));
                self.event(UiEventType::Result(Box::new(self.selected)))
            }

            Event::Key(KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE }) => {
                if self.cancelable {
                    self.result = None;
                    self.event(UiEventType::Canceled)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn get_id(&self) -> UiId {
        self.id
    }

    fn resize(&mut self, _widget_size: &Rectangle) {}

    fn child_widgets(&self) -> Vec<&dyn UiWidget> {
        Vec::new()
    }

    fn child_widgets_mut(&mut self) -> Vec<&mut dyn UiWidget> {
        Vec::new()
    }

    fn mark_refresh(&mut self, value: bool) {
        self.need_refresh = value
    }

    fn need_refresh(&self) -> bool {
        self.need_refresh
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct UiId(NonZeroU64);

pub struct UiContext<'a> {
    pub stdout: &'a mut Stdout,
    id_counter: UiId,
}

impl<'a> UiContext<'a> {
    pub fn create(out: &'a mut Stdout) -> Option<UiContext<'a>> {
        Some(UiContext {
            stdout: out,
            id_counter: UiId(NonZeroU64::new(1).unwrap()),
        })
    }

    pub fn goto(&mut self, p: V2) -> std::io::Result<()> {
        //TODO: sanity check
        if p.x < 0 || p.y < 0 || p.x >= u16::MAX as i32 || p.y >= u16::MAX as i32 {
            return Err(std::io::ErrorKind::Other.into());
        }
        queue!(self.stdout, cursor::MoveTo((p.x) as u16, (p.y) as u16))
    }

    fn should_exit(&mut self, main_id: UiId, event: Option<UiEvent>) -> bool {
        if let Some(ui_event) = event {
            if ui_event.id == main_id {
                return match ui_event.e {
                    UiEventType::Canceled => true,
                    UiEventType::Ok => true,
                    UiEventType::Result(_) => true,
                    _ => false
                };
            }
        }
        return false;
    }

    pub fn run(&mut self, widget: &mut dyn UiWidget) -> std::io::Result<()> {
        let initial_size = terminal::size()?;
        widget.resize(&Rectangle {
            pos: V2::make(0, 0),
            size: V2::make(initial_size.0 as i32, initial_size.1 as i32),
        });
        widget.print(self)?;
        let main_id = widget.get_id();
        let mut last_size = (0u16, 0u16);
        loop {
            let mut has_input = false;
            let mut retry = 25;
            while !has_input && retry > 0 {
                if let Ok(true) = poll(Duration::from_millis(100)) {
                    // Process all available events in one go, to reduce the chance of situation
                    // where a lot of events are queued waiting for redraws. Otherwise likely
                    // to happen when scrolling mouse.
                    while let Ok(true) = poll(Duration::from_secs(0)) {
                        let event = read()?;
                        match event {
                            Event::Key(KeyEvent {
                                           code: KeyCode::Char('c'),
                                           modifiers: KeyModifiers::CONTROL
                                       }) => {
                                return Ok(());
                            }
                            _ => {
                                let r = widget.input(&event, self);
                                if self.should_exit(main_id, r) {
                                    return Ok(());
                                }
                            }
                        }
                        has_input = true;
                    }
                }
                let new_size = terminal::size()?;
                if new_size != last_size {
                    let window_size = V2::make(new_size.0 as i32, new_size.1 as i32);
                    widget.resize(&Rectangle {
                        pos: V2::make(0, 0),
                        size: window_size,
                    });
                    last_size = new_size;
                    break;
                }
                retry -= 1;
            }


            if self.should_exit(main_id, widget.update()) {
                return Ok(());
            }

            widget.print(self)?;
        }
    }

    pub fn buffer_size(&self) -> (u16, u16)
    {
        if let Ok(size) = crossterm::terminal::size() {
            return size;
        }
        (80, 20)
    }

    pub fn next_id(&mut self) -> UiId {
        let result = self.id_counter;
        self.id_counter = UiId(NonZeroU64::new(u64::from(self.id_counter.0) + 1u64).unwrap());
        result
    }

    pub fn restore_normal(&mut self) {
        execute!(self.stdout, crossterm::terminal::LeaveAlternateScreen);
        disable_raw_mode();
        execute!(self.stdout, crossterm::cursor::Show, style::ResetColor, style::SetAttribute(Attribute::Reset), crossterm::event::DisableMouseCapture);
        execute!(self.stdout, cursor::MoveToNextLine(1));
    }
}
