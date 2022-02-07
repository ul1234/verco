use std::{thread, time};

use crate::{
    backend::{RevisionEntry, RevisionInfo},
    mode::*,
    platform::Key,
    ui::{Drawer, RESERVED_LINES_COUNT},
};

pub enum Response {
}

#[derive(Clone, Debug)]
enum State {
    Idle,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default, Clone, Debug)]
pub struct Mode {
    state: State,
    output: Output,
    readline: ReadLine,
    from: ModeKind,
}

impl ModeTrait for Mode {
    fn on_enter(&mut self, _ctx: &ModeContext, info: ModeChangeInfo) {
        self.state = State::Idle;

        self.output.set(String::new());
        self.readline.clear();
        self.from = info.from;
    }

    fn on_key(&mut self, ctx: &ModeContext, key: Key) -> ModeStatus {
        self.readline.on_key(key);
        if key.is_submit() || key.is_cancel() {
            ctx.event_sender.send_mode_revert();
           
            if key.is_submit() {
                let message = self.readline.input().to_string();

                log(format!("commit message:\n {}\n", message));

                 if let ModeKind::Status = self.from {
                    //thread::sleep(time::Duration::from_millis(2000));
                    ctx.event_sender.send_response(ModeResponse::Status(status::Response::Commit(message)));
                    //ctx.event_sender.send_response(ModeResponse::Status(status::Response::Idle));
                 }

                // ctx.event_sender
                //     .send_mode_change(ModeKind::Status, ModeChangeInfo::new(ModeKind::MessageInput));

                    

                    //ctx.event_sender.send_response(ModeResponse::Status(status::Response::Idle));

            }
        }

        ModeStatus { pending_input: true }
    }

    fn on_response(&mut self, _ctx: &ModeContext, _response: ModeResponse) {
    }

    fn is_waiting_response(&self) -> bool {
        match self.state {
            State::Idle => false,
        }
    }

    fn header(&self) -> (&str, &str, &str) {
        match self.state {
            State::Idle => (
                "message input",
                "[enter]submit [Esc]cancel",
                "",
            ),
        }
    }

    fn draw(&self, drawer: &mut Drawer) {
        if let ModeKind::Status = self.from {
            drawer.readline(&self.readline, "type in the commit message...");
        } else if let ModeKind::Stash = self.from {
            drawer.readline(&self.readline, "type in the stash message...");
        }
    }
}
