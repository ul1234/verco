use std::{
    io,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use crate::{
    backend::Backend,
    mode::{self, ModeContext, ModeKind, ModeResponse, ModeTrait},
    platform::{Key, Platform, PlatformEventReader},
    ui::Drawer,
};

enum Event {
    Key(Key),
    Resize(u16, u16),
    Response(ModeResponse),
    ModeChange(ModeKind),
    ModeRefresh(ModeKind),
}

#[derive(Clone)]
pub struct EventSender(mpsc::SyncSender<Event>);
impl EventSender {
    pub fn send_response(&self, result: ModeResponse) {
        self.0.send(Event::Response(result)).unwrap();
    }

    pub fn send_mode_change(&self, mode: ModeKind) {
        self.0.send(Event::ModeChange(mode)).unwrap();
    }

    pub fn send_mode_refresh(&self, mode: ModeKind) {
        self.0.send(Event::ModeRefresh(mode)).unwrap();
    }
}

#[derive(Default)]
struct Application {
    current_mode_kind: ModeKind,

    status_mode: mode::status::Mode,
    log_mode: mode::log::Mode,
    revision_details_mode: mode::revision_details::Mode,
    branches_mode: mode::branches::Mode,
    tags_mode: mode::tags::Mode,
    stash_mode: mode::stash::Mode,

    spinner_state: u8,
}
impl Application {
    fn current_mode(&mut self) -> &mut dyn ModeTrait {
        match &self.current_mode_kind {
            ModeKind::Status => &mut self.status_mode,
            ModeKind::Log => &mut self.log_mode,
            ModeKind::RevisionDetails(_) => &mut self.revision_details_mode,
            ModeKind::Branches => &mut self.branches_mode,
            ModeKind::Tags => &mut self.tags_mode,
            ModeKind::Stash => &mut self.stash_mode,
        }
    }

    fn response_mode(&mut self, response: &ModeResponse) -> &mut dyn ModeTrait {
        match response {
            ModeResponse::Status(_) => &mut self.status_mode,
            ModeResponse::Log(_) => &mut self.log_mode,
            ModeResponse::RevisionDetails(_) => &mut self.revision_details_mode,
            ModeResponse::Branches(_) => &mut self.branches_mode,
            ModeResponse::Tags(_) => &mut self.tags_mode,
            ModeResponse::Stash(_) => &mut self.stash_mode,
        }
    }

    fn revision(&self) -> &str {
        match &self.current_mode_kind {
            ModeKind::RevisionDetails(revision) => revision,
            _ => "",
        }
    }

    pub fn enter_mode(&mut self, ctx: &ModeContext, mode_kind: ModeKind) {
        self.current_mode_kind = mode_kind;
        let revision = self.revision().to_owned();
        self.current_mode().on_enter(ctx, &revision);
    }

    pub fn refresh_mode(&mut self, ctx: &ModeContext, mode_kind: ModeKind) {
        if std::mem::discriminant(&self.current_mode_kind) == std::mem::discriminant(&mode_kind) {
            self.enter_mode(ctx, mode_kind);
        }
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> bool {
        let revision = self.revision().to_owned();
        let status = self.current_mode().on_key(ctx, key, &revision);

        if !status.pending_input {
            if key.is_cancel() {
                return false;
            }

            match key {
                Key::Char('s') => self.enter_mode(ctx, ModeKind::Status),
                Key::Char('l') => self.enter_mode(ctx, ModeKind::Log),
                Key::Char('b') => self.enter_mode(ctx, ModeKind::Branches),
                Key::Char('t') => self.enter_mode(ctx, ModeKind::Tags),
                Key::Char('S') => self.enter_mode(ctx, ModeKind::Stash),
                _ => (),
            }
        }

        true
    }

    pub fn on_response(&mut self, response: ModeResponse) {
        self.response_mode(&response).on_response(response);
    }

    pub fn is_waiting_response(&mut self) -> bool {
        self.current_mode().is_waiting_response()
    }

    pub fn draw_header(&mut self, drawer: &mut Drawer) {
        let spinner = [b'-', b'\\', b'|', b'/'];
        self.spinner_state = (self.spinner_state + 1) % spinner.len() as u8;
        let spinner = match self.is_waiting_response() {
            true => spinner[self.spinner_state as usize],
            false => b' ',
        };

        let (mode_name, left_help, right_help) = self.current_mode().header();
        drawer.header(mode_name, left_help, right_help, spinner);
    }

    pub fn draw_body(&mut self, drawer: &mut Drawer) {
        self.current_mode().draw(drawer);
        drawer.clear_to_bottom();
    }
}

fn terminal_event_loop(mut event_reader: PlatformEventReader, sender: mpsc::SyncSender<Event>) {
    event_reader.init();

    let mut keys = Vec::new();
    loop {
        keys.clear();
        let mut resize = None;

        event_reader.read_terminal_events(&mut keys, &mut resize);

        for &key in &keys {
            if sender.send(Event::Key(key)).is_err() {
                break;
            }
        }
        if let Some(resize) = resize {
            if sender.send(Event::Resize(resize.0, resize.1)).is_err() {
                break;
            }
        }
    }
}

pub fn run(platform_event_reader: PlatformEventReader, backend: Arc<dyn Backend>) {
    let (event_sender, event_receiver) = mpsc::sync_channel(1);

    let mut ctx = ModeContext {
        backend,
        event_sender: EventSender(event_sender.clone()),
        viewport_size: Platform::terminal_size(),
    };

    let _ = thread::spawn(move || {
        terminal_event_loop(platform_event_reader, event_sender);
    });

    let mut application = Application::default();
    application.enter_mode(&ctx, ModeKind::default());

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut stdout_buf = Vec::new();

    const TIMEOUT: Duration = Duration::from_millis(100);

    loop {
        let event = if application.is_waiting_response() {
            event_receiver.recv_timeout(TIMEOUT)
        } else {
            event_receiver
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        };

        let mut draw_body = true;

        match event {
            Ok(Event::Key(key)) => {
                if !application.on_key(&ctx, key) {
                    break;
                }
            }
            Ok(Event::Resize(width, height)) => {
                ctx.viewport_size = (width, height);
            }
            Ok(Event::Response(response)) => application.on_response(response),
            Ok(Event::ModeChange(mode)) => application.enter_mode(&ctx, mode),
            Ok(Event::ModeRefresh(mode)) => application.refresh_mode(&ctx, mode),
            Err(mpsc::RecvTimeoutError::Timeout) => draw_body = false,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        let mut drawer = Drawer::new(stdout_buf, ctx.viewport_size);
        application.draw_header(&mut drawer);
        if draw_body {
            application.draw_body(&mut drawer);
        }
        stdout_buf = drawer.take_buf();

        use io::Write;
        stdout.write_all(&stdout_buf).unwrap();
        stdout.flush().unwrap();
    }
}
