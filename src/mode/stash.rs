use std::thread;

use crate::{
    backend::{Backend, BackendResult, StashEntry},
    mode::{
        Filter, ModeContext, ModeKind, ModeResponse, ModeStatus, ModeTrait, Output, ReadLine,
        SelectMenu,
    },
    platform::Key,
    ui::{Color, Drawer, SelectEntryDraw, RESERVED_LINES_COUNT},
};

pub enum Response {
    Refresh(BackendResult<Vec<StashEntry>>),
    Details(String),
    Diff(String),
}

enum WaitOperation {
    Refresh,
    Discard,
    Pop,
}

enum State {
    Idle,
    Waiting(WaitOperation),
    ViewDetails(usize),
    ViewDiff,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl SelectEntryDraw for StashEntry {
    fn draw(&self, drawer: &mut Drawer, hovered: bool, _: bool) -> usize {
        fn color(color: Color, hovered: bool) -> Color {
            if hovered {
                Color::White
            } else {
                color
            }
        }

        drawer.fmt(format_args!(
            "{}[{}] {}{} {}{}",
            color(Color::DarkYellow, hovered),
            self.id,
            color(Color::DarkGreen, hovered),
            &self.branch,
            color(Color::White, hovered),
            &self.message
        ));
        1
    }
}
#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<StashEntry>,
    output: Output,
    select: SelectMenu,
    filter: Filter,
}

impl ModeTrait for Mode {
    fn on_enter(&mut self, ctx: &ModeContext, _revision: &str) {
        if let State::Waiting(_) = self.state {
            return;
        }
        self.state = State::Waiting(WaitOperation::Refresh);

        self.output.set(String::new());
        self.filter.filter(self.entries.iter());
        self.select
            .saturate_cursor(self.filter.visible_indices().len());

        request(ctx, |_| Ok(()));
    }

    fn on_key(&mut self, ctx: &ModeContext, key: Key, _revision: &str) -> ModeStatus {
        let pending_input = self.filter.has_focus();
        let available_height = (ctx.viewport_size.1 as usize).saturating_sub(RESERVED_LINES_COUNT);

        if self.filter.has_focus() {
            self.filter.on_key(key);
            self.filter.filter(self.entries.iter());
            self.select
                .saturate_cursor(self.filter.visible_indices().len());
        } else {
            match self.state {
                State::Idle | State::Waiting(_) => {
                    if self.output.text().is_empty() {
                        self.select.on_key(
                            self.filter.visible_indices().len(),
                            available_height,
                            key,
                        );
                    } else {
                        self.output.on_key(available_height, key);
                    }

                    let current_entry_index = self.filter.get_visible_index(self.select.cursor);
                    match key {
                        Key::Ctrl('f') => self.filter.enter(),
                        Key::Enter => {
                            if let Some(current_entry_index) = current_entry_index {
                                let entry = &self.entries[current_entry_index];
                                let id = entry.id;
                                self.state = State::ViewDetails(id);

                                let ctx = ctx.clone();
                                thread::spawn(move || match ctx.backend.stash_show(id) {
                                    Ok(info) => ctx.event_sender.send_response(
                                        ModeResponse::Stash(Response::Details(info)),
                                    ),
                                    Err(error) => ctx.event_sender.send_response(
                                        ModeResponse::Stash(Response::Refresh(Err(error))),
                                    ),
                                });
                            }
                        }
                        Key::Char('p') => {
                            if let Some(current_entry_index) = current_entry_index {
                                let entry = &self.entries[current_entry_index];
                                let id = entry.id;
                                let ctx = ctx.clone();

                                thread::spawn(move || match ctx.backend.stash_pop(id) {
                                    Ok(()) => {
                                        ctx.event_sender.send_mode_change(ModeKind::Status);
                                        ctx.event_sender.send_mode_refresh(ModeKind::Status);
                                    }
                                    Err(error) => ctx.event_sender.send_response(
                                        ModeResponse::Stash(Response::Refresh(Err(error))),
                                    ),
                                });
                            }
                        }
                        _ => (),
                    }
                }
                State::ViewDetails(id) => match key {
                    Key::Enter => {
                        self.output.set(String::new());
                        self.state = State::ViewDiff;

                        let ctx = ctx.clone();
                        thread::spawn(move || match ctx.backend.stash_diff(id) {
                            Ok(info) => ctx
                                .event_sender
                                .send_response(ModeResponse::Stash(Response::Diff(info))),
                            Err(error) => ctx
                                .event_sender
                                .send_response(ModeResponse::Stash(Response::Refresh(Err(error)))),
                        });
                    }
                    _ => self.output.on_key(available_height, key),
                },
                State::ViewDiff => self.output.on_key(available_height, key),
            }
        }

        ModeStatus { pending_input }
    }

    fn on_response(&mut self, response: ModeResponse) {
        let response = as_variant!(response, ModeResponse::Stash).unwrap();
        match response {
            Response::Refresh(result) => {
                self.entries = Vec::new();
                self.output.set(String::new());

                if let State::Waiting(_) = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    match result {
                        Ok(entries) => self.entries = entries,
                        Err(error) => self.output.set(error),
                    }
                }

                self.filter.filter(self.entries.iter());
                self.select
                    .saturate_cursor(self.filter.visible_indices().len());
            }
            Response::Details(mut info) | Response::Diff(mut info) => {
                if info.is_empty() {
                    info.push('\n');
                }
                self.output.set(info);
            }
        }
    }

    fn is_waiting_response(&self) -> bool {
        match self.state {
            State::Idle => false,
            State::Waiting(_) => true,
            State::ViewDetails(_) | State::ViewDiff => self.output.text().is_empty(),
        }
    }

    fn header(&self) -> (&str, &str, &str) {
        let name = match self.state {
            State::Idle | State::Waiting(WaitOperation::Refresh) => "stash list",
            State::Waiting(WaitOperation::Discard) => "discard",
            State::Waiting(WaitOperation::Pop) => "pop",
            State::ViewDetails(_) => "stash details",
            State::ViewDiff => "diff",
        };

        let (left_help, right_help) = match self.state {
            State::Idle | State::Waiting(_) => (
                "[p]pop [enter]details [d]discard",
                "[arrows]move [ctrl+f]filter",
            ),
            State::ViewDetails(_) => ("[enter]details", "[arrows]move"),
            State::ViewDiff => ("", "[arrows]move"),
        };

        (name, left_help, right_help)
    }

    fn draw(&self, drawer: &mut Drawer) {
        let filter_line_count = drawer.filter(&self.filter);
        match self.state {
            State::Idle | State::Waiting(_) => {
                if self.output.text.is_empty() {
                    if self.entries.is_empty() {
                        if let State::Idle = self.state {
                            drawer.output(&Output::new("No Stashes!".to_owned()));
                        }
                    } else {
                        drawer.select_menu(
                            &self.select,
                            filter_line_count,
                            false,
                            self.filter
                                .visible_indices()
                                .iter()
                                .map(|&i| &self.entries[i]),
                        );
                    }
                } else {
                    drawer.output(&self.output);
                }
            }
            State::ViewDetails(_) => {
                drawer.stash_details(&self.output);
            }
            State::ViewDiff => {
                drawer.diff(&self.output);
            }
        }
    }
}

fn request<F>(ctx: &ModeContext, f: F)
where
    F: 'static + Send + Sync + FnOnce(&dyn Backend) -> BackendResult<()>,
{
    let ctx = ctx.clone();
    thread::spawn(move || {
        use std::ops::Deref;

        let result = f(ctx.backend.deref()).and_then(|_| ctx.backend.stash_list());

        ctx.event_sender
            .send_response(ModeResponse::Stash(Response::Refresh(result)));
    });
}
