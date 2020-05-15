use std::process::Command;

use crate::{application::get_process_output, select::Entry};

pub trait VersionControlActions: Send {
    /// Sets the root of the current repository
    fn set_root(&mut self) -> Result<(), String>;
    /// Get the root of the current repository
    fn get_root(&self) -> &str;

    fn get_current_changed_files(&mut self) -> Result<Vec<Entry>, String>;
    fn get_revision_changed_files(
        &mut self,
        target: &str,
    ) -> Result<Vec<Entry>, String>;

    fn version(&mut self) -> Result<String, String>;

    fn status(&mut self) -> Result<String, String>;
    /// Shows the header and all diffs for the current revision
    fn current_export(&mut self) -> Result<String, String>;
    fn log(&mut self, count: u32) -> Result<String, String>;

    fn current_diff_all(&mut self) -> Result<String, String>;
    fn current_diff_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Result<String, String>;

    fn revision_changes(&mut self, target: &str) -> Result<String, String>;
    fn revision_diff_all(&mut self, target: &str) -> Result<String, String>;
    fn revision_diff_selected(
        &mut self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Result<String, String>;

    fn commit_all(&mut self, message: &str) -> Result<String, String>;
    fn commit_selected(
        &mut self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Result<String, String>;
    fn revert_all(&mut self) -> Result<String, String>;
    fn revert_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Result<String, String>;
    fn update(&mut self, target: &str) -> Result<String, String>;
    fn merge(&mut self, target: &str) -> Result<String, String>;

    fn conflicts(&mut self) -> Result<String, String>;
    fn take_other(&mut self) -> Result<String, String>;
    fn take_local(&mut self) -> Result<String, String>;

    fn fetch(&mut self) -> Result<String, String>;
    fn pull(&mut self) -> Result<String, String>;
    fn push(&mut self) -> Result<String, String>;

    fn create_tag(&mut self, name: &str) -> Result<String, String>;
    fn list_branches(&mut self) -> Result<String, String>;
    fn create_branch(&mut self, name: &str) -> Result<String, String>;
    fn close_branch(&mut self, name: &str) -> Result<String, String>;
}

pub fn handle_command(command: &mut Command) -> Result<String, String> {
    match command.spawn() {
        Ok(mut child) => {
            match child.wait() {
                Ok(status) => get_process_output(&mut child, status),
                Err(e) => Err(e.to_string()),
            }
        }
        Err(e) => Err(e.to_string()),
    }
}
