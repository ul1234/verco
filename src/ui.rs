use std::{
    fmt,
    io::{StdoutLock, Write},
};

use crossterm::{self, cursor, style, terminal};

use crate::mode::{HeaderInfo, Output, ReadLine, SelectMenu};

pub enum Color {
    White,
    Red,
    Green,
    Blue,
    Yellow,
}
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::White => f.write_str("\x1b[38;5;15m"),
            Self::Red => f.write_str("\x1b[38;5;1m"),
            Self::Green => f.write_str("\x1b[38;5;2m"),
            Self::Blue => f.write_str("\x1b[38;5;4m"),
            Self::Yellow => f.write_str("\x1b[38;5;3m"),
        }
    }
}

pub trait SelectEntryDraw {
    fn draw(&self, drawer: &mut Drawer, hovered: bool);
}

pub struct Drawer<'stdout, 'lock> {
    stdout: &'lock mut StdoutLock<'stdout>,
    pub viewport_size: (u16, u16),
}

impl<'stdout, 'lock> Drawer<'stdout, 'lock> {
    pub fn new(
        stdout: &'lock mut StdoutLock<'stdout>,
        viewport_size: (u16, u16),
    ) -> Self {
        Self {
            stdout,
            viewport_size,
        }
    }

    pub fn clear_to_bottom(&mut self) {
        crossterm::queue!(
            self.stdout,
            style::SetBackgroundColor(style::Color::Black),
            terminal::Clear(terminal::ClearType::FromCursorDown),
        )
        .unwrap();
    }

    pub fn header(&mut self, info: HeaderInfo, spinner_state: u8) {
        let background_color = style::Color::DarkYellow;
        let foreground_color = style::Color::Black;

        let spinner = ['-', '\\', '|', '/'];
        let spinner = match info.waiting_response {
            true => spinner[spinner_state as usize % spinner.len()],
            false => ' ',
        };

        crossterm::queue!(
            self.stdout,
            cursor::MoveTo(0, 0),
            style::SetBackgroundColor(background_color),
            style::SetForegroundColor(foreground_color),
            style::Print(' '),
            style::Print(spinner),
            style::Print(' '),
            style::SetBackgroundColor(foreground_color),
            style::SetForegroundColor(background_color),
            style::Print(' '),
            style::Print(info.name),
            style::Print(' '),
            style::SetBackgroundColor(background_color),
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1),
            style::ResetColor,
        )
        .unwrap();
    }

    pub fn write(&mut self, display: &dyn fmt::Display) {
        write!(self.stdout, "{}", display).unwrap();
    }

    pub fn next_line(&mut self) {
        crossterm::queue!(
            self.stdout,
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1),
        )
        .unwrap();
    }

    pub fn output(&mut self, output: &Output) {
        for line in output.lines_from_scroll() {
            crossterm::queue!(
                self.stdout,
                style::Print(line),
                terminal::Clear(terminal::ClearType::UntilNewLine),
                cursor::MoveToNextLine(1),
            )
            .unwrap();
        }
    }

    pub fn readline(&mut self, readline: &ReadLine) {
        crossterm::queue!(
            self.stdout,
            style::SetBackgroundColor(style::Color::Black),
            style::SetForegroundColor(style::Color::White),
            style::Print(readline.input()),
            style::SetBackgroundColor(style::Color::DarkRed),
            style::Print(' '),
            style::SetBackgroundColor(style::Color::Black),
        )
        .unwrap();
    }

    pub fn select_menu<'entries, I, E>(
        &mut self,
        select: &SelectMenu,
        header_height: u16,
        entries: I,
    ) where
        I: 'entries + Iterator<Item = &'entries E>,
        E: 'entries + SelectEntryDraw,
    {
        let cursor_index = select.cursor();

        crossterm::queue!(
            self.stdout,
            style::SetBackgroundColor(style::Color::Black),
            style::SetForegroundColor(style::Color::White),
        )
        .unwrap();

        let take_count =
            self.viewport_size.1.saturating_sub(1 + header_height) as usize;

        for (i, entry) in
            entries.enumerate().skip(select.scroll()).take(take_count)
        {
            let hovered = i == cursor_index;
            if hovered {
                crossterm::queue!(
                    self.stdout,
                    style::SetBackgroundColor(style::Color::DarkMagenta),
                )
                .unwrap();
            }

            entry.draw(self, hovered);

            crossterm::queue!(
                self.stdout,
                terminal::Clear(terminal::ClearType::UntilNewLine),
                cursor::MoveToNextLine(1),
            )
            .unwrap();

            if hovered {
                crossterm::queue!(
                    self.stdout,
                    style::SetBackgroundColor(style::Color::Black),
                )
                .unwrap();
            }
        }
    }
}