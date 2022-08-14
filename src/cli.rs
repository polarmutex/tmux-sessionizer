use clap::{ArgMatches, Command};

pub fn create_app() -> ArgMatches {
    Command::new("tmux-sessionizer")
        .version("0.1.0")
        .about("open tmux-session for selected project")
        .get_matches()
}
