use std::sync::Arc;

use crate::{application::EventSender, backend::Backend, platform::Key, ui::Drawer};

pub mod branches;
pub mod log;
pub mod revision_details;
pub mod stash;
pub mod status;
pub mod tags;

pub enum ModeResponse {
    Status(status::Response),
    Log(log::Response),
    RevisionDetails(revision_details::Response),
    Branches(branches::Response),
    Tags(tags::Response),
    Stash(stash::Response),
}

#[derive(Clone)]
pub enum ModeKind {
    Status,
    Log,
    RevisionDetails(String),
    Branches,
    Tags,
    Stash,
}
impl Default for ModeKind {
    fn default() -> Self {
        Self::Status
    }
}

pub trait ModeTrait {
    fn on_enter(&mut self, ctx: &ModeContext, revision: &str);
    fn on_key(&mut self, ctx: &ModeContext, key: Key, revision: &str) -> ModeStatus;
    fn is_waiting_response(&self) -> bool;
    fn on_response(&mut self, response: ModeResponse);
    fn header(&self) -> (&str, &str, &str);
    fn draw(&self, drawer: &mut Drawer);
    fn save(&mut self);
    fn restore(&mut self);
}

#[derive(Clone)]
pub struct ModeContext {
    pub backend: Arc<dyn Backend>,
    pub event_sender: EventSender,
    pub viewport_size: (u16, u16),
}

pub struct ModeStatus {
    pub pending_input: bool,
}

#[derive(Default, Clone)]
pub struct Output {
    text: String,
    line_count: usize,
    scroll: usize,
}
impl Output {
    pub fn new(text: String) -> Self {
        let mut output = Output::default();
        output.set(text);
        output
    }

    pub fn set(&mut self, output: String) {
        self.text = output;
        self.line_count = self.text.lines().count();
        self.scroll = 0;
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn line_count(&self) -> usize {
        self.line_count
    }

    pub fn lines_from_scroll<'a>(&'a self) -> impl 'a + Iterator<Item = &'a str> {
        self.text.lines().skip(self.scroll)
    }

    pub fn on_key(&mut self, available_height: usize, key: Key) {
        let half_height = available_height / 2;

        self.scroll = match key {
            Key::Down | Key::Char('j') => self.scroll + 1,
            Key::Up | Key::Char('k') => self.scroll.saturating_sub(1),
            Key::Ctrl('h') | Key::Home => 0,
            Key::Ctrl('e') | Key::End => usize::MAX,
            Key::Ctrl('d') | Key::PageDown => self.scroll + half_height,
            Key::Ctrl('u') | Key::PageUp => self.scroll.saturating_sub(half_height),
            _ => self.scroll,
        };

        self.scroll = self.line_count.saturating_sub(available_height).min(self.scroll);
    }
}

#[derive(Default, Clone)]
pub struct ReadLine {
    input: String,
}
impl ReadLine {
    pub fn clear(&mut self) {
        self.input.clear();
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn on_key(&mut self, key: Key) {
        match key {
            Key::Home | Key::Ctrl('u') => self.input.clear(),
            Key::Ctrl('w') => {
                fn is_word(c: char) -> bool {
                    c.is_alphanumeric() || c == '_'
                }

                fn rfind_boundary(mut chars: std::str::Chars, f: fn(&char) -> bool) -> usize {
                    match chars.rfind(f) {
                        Some(c) => chars.as_str().len() + c.len_utf8(),
                        None => 0,
                    }
                }

                let mut chars = self.input.chars();
                if let Some(c) = chars.next_back() {
                    let len = if is_word(c) {
                        rfind_boundary(chars, |&c| !is_word(c))
                    } else if c.is_ascii_whitespace() {
                        rfind_boundary(chars, |&c| is_word(c) || !c.is_ascii_whitespace())
                    } else {
                        rfind_boundary(chars, |&c| is_word(c) || c.is_ascii_whitespace())
                    };
                    self.input.truncate(len);
                }
            }
            Key::Backspace => {
                if let Some((last_char_index, _)) = self.input.char_indices().next_back() {
                    self.input.truncate(last_char_index);
                }
            }
            Key::Char(c) => self.input.push(c),
            _ => (),
        }
    }
}

pub enum SelectMenuAction {
    None,
    Toggle(usize),
    ToggleAll,
}

#[derive(Default, Clone)]
pub struct SelectMenu {
    pub cursor: usize,
    pub scroll: usize, // index of the first line when scrolling
}
impl SelectMenu {
    pub fn saturate_cursor(&mut self, entries_len: usize) {
        self.cursor = entries_len.saturating_sub(1).min(self.cursor);
    }

    pub fn on_remove_entry(&mut self, index: usize) {
        if index <= self.cursor {
            self.cursor = self.cursor.saturating_sub(1);
        }
    }

    pub fn on_key(&mut self, entries_len: usize, available_height: usize, key: Key) -> SelectMenuAction {
        let half_height = available_height / 2;

        self.cursor = match key {
            Key::Down | Key::Ctrl('n') | Key::Char('j') => self.cursor + 1,
            Key::Up | Key::Ctrl('p') | Key::Char('k') => self.cursor.saturating_sub(1),
            Key::Ctrl('h') | Key::Home => 0,
            Key::Ctrl('e') | Key::End => usize::MAX,
            Key::Ctrl('d') | Key::PageDown => self.cursor + half_height,
            Key::Ctrl('u') | Key::PageUp => self.cursor.saturating_sub(half_height),
            _ => self.cursor,
        };

        self.saturate_cursor(entries_len);

        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor >= self.scroll + available_height {
            self.scroll = self.cursor + 1 - available_height;
        }

        match key {
            Key::Char(' ') if self.cursor < entries_len => SelectMenuAction::Toggle(self.cursor),
            Key::Char('a') => SelectMenuAction::ToggleAll,
            _ => SelectMenuAction::None,
        }
    }
}

pub trait FilterEntry {
    fn fuzzy_matches(&self, pattern: &str) -> bool;
}

#[derive(Default, Clone)]
pub struct Filter {
    has_focus: bool,
    readline: ReadLine,
    visible_indices: Vec<usize>,
}
impl Filter {
    pub fn clear(&mut self) {
        self.has_focus = false;
        self.readline.clear();
        self.visible_indices.clear();
    }

    pub fn enter(&mut self) {
        self.has_focus = true;
        self.readline.clear();
    }

    pub fn on_key(&mut self, key: Key) {
        if key.is_submit() || key == Key::Ctrl('f') {
            self.has_focus = false;
        } else if key.is_cancel() {
            self.has_focus = false;
            self.readline.clear();
        } else {
            self.readline.on_key(key);
        }
    }

    pub fn filter<'entries, I, E>(&mut self, entries: I)
    where
        I: 'entries + Iterator<Item = &'entries E>,
        E: 'entries + FilterEntry,
    {
        self.visible_indices.clear();
        for (i, entry) in entries.enumerate() {
            if entry.fuzzy_matches(self.as_str()) {
                self.visible_indices.push(i);
            }
        }
    }

    pub fn on_remove_entry(&mut self, entry_index: usize) {
        for i in (0..self.visible_indices.len()).rev() {
            if entry_index < i {
                self.visible_indices[i] -= 1;
            } else if entry_index == i {
                self.visible_indices.remove(i);
            } else {
                break;
            }
        }
    }

    pub fn get_visible_index(&self, index: usize) -> Option<usize> {
        self.visible_indices.get(index).cloned()
    }

    pub fn visible_indices(&self) -> &[usize] {
        &self.visible_indices
    }

    pub fn is_filtering(&self) -> bool {
        self.has_focus || !self.readline.input().is_empty()
    }

    pub fn has_focus(&self) -> bool {
        self.has_focus
    }

    pub fn as_str(&self) -> &str {
        self.readline.input()
    }
}

pub fn fuzzy_matches(text: &str, pattern: &str) -> bool {
    let mut pattern_chars = pattern.chars();
    let mut pattern_char = match pattern_chars.next() {
        Some(c) => c,
        None => return true,
    };

    let mut previous_matched_index = 0;
    let mut was_alphanumeric = false;

    for (i, text_char) in text.char_indices() {
        if text_char.eq_ignore_ascii_case(&pattern_char) {
            let is_alphanumeric = text_char.is_ascii_alphanumeric();
            let matched = !is_alphanumeric || !was_alphanumeric || previous_matched_index + 1 == i;
            was_alphanumeric = is_alphanumeric;

            if matched {
                previous_matched_index = i;
                pattern_char = match pattern_chars.next() {
                    Some(c) => c,
                    None => return true,
                };
            }
        }
    }

    false
}
