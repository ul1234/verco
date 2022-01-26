use std::thread;

use crate::{
    backend::{Backend, BackendResult, FileStatus, RevisionEntry, StatusInfo},
    mode::{
        Filter, ModeContext, ModeKind, ModeResponse, ModeStatus, ModeTrait, Output, ReadLine,
        SelectMenu, SelectMenuAction,
    },
    platform::Key,
    ui::{Color, Drawer, SelectEntryDraw, RESERVED_LINES_COUNT},
};

pub enum Response {
    Refresh(Vec<String>),
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
    ViewDiff,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<RevisionEntry>,
    output: Output,
    select: SelectMenu,
    filter: Filter,
    readline: ReadLine,
}

impl Mode {
    fn get_selected_entries(&self) -> Vec<RevisionEntry> {
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|&e| e.selected)
            .cloned()
            .collect();
        entries
    }

    fn remove_selected_entries(&mut self) {
        let previous_len = self.entries.len();

        for i in (0..self.entries.len()).rev() {
            if self.entries[i].selected {
                self.entries.remove(i);
                self.filter.on_remove_entry(i);
                let i = match self.filter.visible_indices().binary_search(&i) {
                    Ok(i) => i,
                    Err(i) => i,
                };
                self.select.on_remove_entry(i);
            }
        }

        if self.entries.len() == previous_len {
            self.entries.clear();
            self.select.cursor = 0;
            self.filter.clear();
        }
    }
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
        self.readline.clear();

        request(ctx, |_| Ok(()));
    }

    fn on_key(&mut self, ctx: &ModeContext, key: Key, _revision: &str) -> ModeStatus {
        let pending_input = false;
        ModeStatus { pending_input }
    }

    fn on_response(&mut self, response: ModeResponse) {
        let response = as_variant!(response, ModeResponse::Stash).unwrap();
        match response {
            Response::Refresh(info) => {
                let s = info.into_iter().collect();
                self.output.set(s);

                if let State::Waiting(_) = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {}

                self.filter.filter(self.entries.iter());
                self.select
                    .saturate_cursor(self.filter.visible_indices().len());
            }
            _ => unimplemented!(),
        }
    }

    fn is_waiting_response(&self) -> bool {
        match self.state {
            State::Idle => false,
            State::Waiting(_) => true,
            State::ViewDiff => self.output.text().is_empty(),
        }
    }

    fn header(&self) -> (&str, &str, &str) {
        let name = match self.state {
            State::Idle | State::Waiting(WaitOperation::Refresh) => "stash list",
            State::Waiting(WaitOperation::Discard) => "discard",
            State::Waiting(WaitOperation::Pop) => "pop",
            State::ViewDiff => "diff",
        };

        let left_help = "[p]pop [enter]details [d]discard";
        let right_help = "[tab]full message [arrows]move [ctrl+f]filter";
        (name, left_help, right_help)
    }

    fn draw(&self, drawer: &mut Drawer) {
        let filter_line_count = drawer.filter(&self.filter);
        if self.output.text().is_empty() {
            drawer.select_menu(
                &self.select,
                filter_line_count,
                false,
                self.filter
                    .visible_indices()
                    .iter()
                    .map(|&i| &self.entries[i]),
            );
        } else {
            drawer.output(&self.output);
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

        let info = match f(ctx.backend.deref()).and_then(|_| ctx.backend.stash_list()) {
            Ok(info) => info,
            Err(error) => vec![error],
        };

        ctx.event_sender
            .send_response(ModeResponse::Stash(Response::Refresh(info)));
    });
}
