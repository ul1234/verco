use std::{fs, io::Write};

const LOG_TO_FILE_ENABLE: bool = false;

pub fn log<S: Into<String>>(info: S) {
    if LOG_TO_FILE_ENABLE {
        let filename = "test.txt";
        let mut file = fs::OpenOptions::new().write(true).append(true).open(filename).expect("log file open failed!");
        file.write_all(info.into().as_bytes()).unwrap();
    }
}
