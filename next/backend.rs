use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
};

pub mod git;

pub trait Backend: 'static + Send + Sync {
    fn name(&self) -> &str;

    fn status(&self) -> Result<String, String>;
}

/*
pub trait VersionControlActions: Send {
    fn executable_name(&self) -> &'static str;
    fn current_dir(&self) -> &str;

    fn command(&self) -> Command {
        let mut command = Command::new(self.executable_name());
        command.current_dir(self.current_dir());
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command
    }

    /// Sets the root of the current repository
    fn set_root(&mut self) -> Result<(), String>;
    /// Get the root of the current repository
    fn get_root(&self) -> &str;

    fn get_current_changed_files(&self) -> Result<Vec<Entry>, String>;
    fn get_revision_changed_files(
        &self,
        target: &str,
    ) -> Result<Vec<Entry>, String>;

    fn version(&self) -> Result<String, String>;

    fn status(&self) -> Box<dyn ActionTask>;
    /// Shows the header and all diffs for the current revision
    fn current_export(&self) -> Box<dyn ActionTask>;
    fn log(&self, count: usize) -> Box<dyn ActionTask>;

    fn current_diff_all(&self) -> Box<dyn ActionTask>;
    fn current_diff_selected(
        &self,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask>;

    fn revision_changes(&self, target: &str) -> Box<dyn ActionTask>;
    fn revision_diff_all(&self, target: &str) -> Box<dyn ActionTask>;
    fn revision_diff_selected(
        &self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask>;

    fn commit_all(&self, message: &str) -> Box<dyn ActionTask>;
    fn commit_selected(
        &self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask>;
    fn revert_all(&self) -> Box<dyn ActionTask>;
    fn revert_selected(&self, entries: &Vec<Entry>) -> Box<dyn ActionTask>;
    fn update(&self, target: &str) -> Box<dyn ActionTask>;
    fn merge(&self, target: &str) -> Box<dyn ActionTask>;

    fn conflicts(&self) -> Box<dyn ActionTask>;
    fn take_other(&self) -> Box<dyn ActionTask>;
    fn take_local(&self) -> Box<dyn ActionTask>;

    fn fetch(&self) -> Box<dyn ActionTask>;
    fn pull(&self) -> Box<dyn ActionTask>;
    fn push(&self) -> Box<dyn ActionTask>;

    fn create_tag(&self, name: &str) -> Box<dyn ActionTask>;
    fn list_branches(&self) -> Box<dyn ActionTask>;
    fn create_branch(&self, name: &str) -> Box<dyn ActionTask>;
    fn close_branch(&self, name: &str) -> Box<dyn ActionTask>;
}
*/

pub struct Process(Child);
impl Process {
    pub fn spawn(command_name: &str, args: &[&str]) -> Result<Self, String> {
        let mut command = Command::new(command_name);
        command.args(args);
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        match command.spawn() {
            Ok(child) => Ok(Self(child)),
            Err(error) => Err(format!(
                "could not spawn process '{}': {}",
                command_name, error
            )),
        }
    }

    pub fn wait(self) -> Result<String, String> {
        let output = match self.0.wait_with_output() {
            Ok(output) => output,
            Err(error) => {
                return Err(format!("could not wait for process: {}", error))
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            Ok(stdout.into())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut error = String::new();
            error.push_str(&stdout);
            error.push('\n');
            error.push_str(&stderr);
            Err(error)
        }
    }
}

pub fn backend_from_current_repository() -> Option<(PathBuf, Arc<dyn Backend>)>
{
    if let Some((root, git)) = git::Git::try_new() {
        Some((root, Arc::new(git)))
    } else {
        None
    }
}
