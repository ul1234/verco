use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    None,
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    Delete,
    F(u8),
    Char(char),
    Ctrl(char),
    Alt(char),
    Esc,
}

#[derive(Clone, Copy)]
pub enum ProcessTag {
    None, // TODO: something
}

#[derive(Clone, Copy)]
pub struct ProcessHandle(pub usize);

pub enum PlatformEvent {
    Resize(u16, u16),
    Key(Key),
    ProcessSpawned {
        tag: ProcessTag,
        handle: ProcessHandle,
    },
    ProcessOutput {
        tag: ProcessTag,
        buf: Vec<u8>,
    },
    ProcessExit {
        tag: ProcessTag,
    },
}

pub enum PlatformRequest {
    SpawnProcess {
        tag: ProcessTag,
        command: Command,
        buf_len: usize,
    },
    WriteToProcess {
        handle: ProcessHandle,
        buf: Vec<u8>,
    },
    CloseProcessInput {
        handle: ProcessHandle,
    },
    KillProcess {
        handle: ProcessHandle,
    },
}
