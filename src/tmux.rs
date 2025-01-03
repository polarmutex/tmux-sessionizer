use std::env;
use std::os::unix::process::CommandExt;
use std::process;

#[derive(Clone)]
pub struct Tmux {
    socket_name: String,
}

impl Default for Tmux {
    fn default() -> Self {
        let socket_name = env::var("TMS_TMUX_SOCKET")
            .ok()
            .unwrap_or(String::from("default"));

        Self { socket_name }
    }
}

impl Tmux {
    fn execute_tmux_command(&self, args: &[&str]) -> process::Output {
        process::Command::new("tmux")
            .args(["-L", &self.socket_name])
            .args(args)
            .stdin(process::Stdio::inherit())
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute the tmux command `{args:?}`"))
    }

    fn replace_with_tmux_command(&self, args: &[&str]) -> std::io::Error {
        process::Command::new("tmux")
            .args(["-L", &self.socket_name])
            .args(args)
            .stdin(process::Stdio::inherit())
            .exec()
    }

    pub fn capture_pane(&self, target_pane: &str) -> process::Output {
        self.execute_tmux_command(&["capture-pane", "-ep", "-t", target_pane])
    }

    fn stdout_to_string(output: process::Output) -> String {
        String::from_utf8(output.stdout)
            .expect("The output of a `tmux` command should always be valid utf-8")
    }

    pub fn list_sessions(&self, format: &str) -> String {
        let output = self.execute_tmux_command(&["list-sessions", "-F", format]);
        Tmux::stdout_to_string(output)
    }

    pub fn session_exists(&self, repo_short_name: &str) -> bool {
        // Get the tmux sessions
        let sessions = self.list_sessions("'#S'");

        // If the session already exists switch to it, else create the new session and then switch
        sessions.lines().any(|line| {
            let mut cleaned_line = line.to_owned();
            // tmux will return the output with extra ' and \n characters
            cleaned_line.retain(|char| char != '\'' && char != '\n');

            cleaned_line == repo_short_name
        })
    }

    pub fn new_session(&self, name: Option<&str>, path: Option<&str>) -> process::Output {
        let mut args = vec!["new-session", "-d"];

        if let Some(name) = name {
            args.extend(["-s", name]);
        };

        if let Some(path) = path {
            args.extend(["-c", path]);
        }

        self.execute_tmux_command(&args)
    }

    pub fn switch_to_session(&self, repo_short_name: &str) {
        if !is_in_tmux_session() {
            self.attach_session(Some(repo_short_name), None);
        } else {
            let result = self.switch_client(repo_short_name);
            if !result.status.success() {
                self.attach_session(Some(repo_short_name), None);
            }
        }
    }

    pub fn attach_session(&self, session_name: Option<&str>, path: Option<&str>) -> std::io::Error {
        let mut args = vec!["attach-session"];

        if let Some(name) = session_name {
            args.extend(["-t", name]);
        };

        if let Some(path) = path {
            args.extend(["-c", path]);
        }

        self.replace_with_tmux_command(&args)
    }

    pub fn switch_client(&self, session_name: &str) -> process::Output {
        let output = self.execute_tmux_command(&["switch-client", "-t", session_name]);
        if !output.status.success() {
            self.execute_tmux_command(&["attach-session", "-t", session_name])
        } else {
            output
        }
    }
}

fn is_in_tmux_session() -> bool {
    std::env::var("TERM_PROGRAM").is_ok_and(|program| program == "tmux")
}
