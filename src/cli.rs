use crate::configs::Config;
use crate::error::Result;
use crate::error::TmsError;
use crate::tmux::Tmux;
use clap::{Parser, Subcommand};
use error_stack::ResultExt;

#[derive(Debug, Parser)]
#[command(author, version)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {}

impl Cli {
    pub fn handle_sub_commands(&self, _tmux: &Tmux) -> Result<SubCommandGiven> {
        // Get the configuration from the config file
        let config = Config::new().change_context(TmsError::ConfigError)?;
        match &self.command {
            None => Ok(SubCommandGiven::No(config.into())),
            _ => Ok(SubCommandGiven::No(config.into())),
        }
    }
}

pub enum SubCommandGiven {
    Yes,
    No(Box<Config>),
}
