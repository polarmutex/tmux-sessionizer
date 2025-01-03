use clap::CommandFactory;
use clap::Parser;
use clap_complete::CompleteEnv;
use error_stack::Report;
use error_stack::ResultExt;
use gix::Repository;
use skim::prelude::*;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io::Cursor;
use tms::cli::Cli;
use tms::cli::SubCommandGiven;
use tms::configs::Config;
use tms::configs::SearchDirectory;
use tms::error::Result;
use tms::error::Suggestion;
use tms::error::TmsError;
use tms::tmux::Tmux;

fn main() -> Result<()> {
    Report::install_debug_hook::<Suggestion>(|value, context| {
        context.push_body(format!("{value}"));
    });
    #[cfg(any(not(debug_assertions), test))]
    Report::install_debug_hook::<std::panic::Location>(|_value, _context| {});

    let bin_name = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.file_name().map(|exe| exe.to_string_lossy().to_string()))
        .unwrap_or("tms".into());
    match CompleteEnv::with_factory(Cli::command)
        .bin(bin_name)
        .try_complete(env::args_os(), None)
    {
        Ok(true) => return Ok(()),
        Err(e) => {
            panic!("failed to generate completions: {e}");
        }
        Ok(false) => {}
    };

    let cli_args = Cli::parse();

    let tmux = Tmux::default();

    let config = match cli_args.handle_sub_commands(&tmux)? {
        SubCommandGiven::Yes => return Ok(()),
        SubCommandGiven::No(config) => config, // continue
    };

    let repos = find_repos(&config)?;
    println!("repos {:?}", repos);
    // let repo_list: Vec<String> = repos.keys().map(|s| s.to_owned()).collect();

    let selected_str = if let Some(str) = get_single_selection(&repos)? {
        str
    } else {
        return Ok(());
    };

    if let Some(repo) = repos.get(&selected_str) {
        let repo_worktrees = repo.worktrees().unwrap();
        let path = if repo.head_tree().unwrap().id.is_empty_tree() {
            repo_worktrees
                .iter()
                .find(|r| {
                    let base = r.base().unwrap();
                    let base_filename = base.file_name().unwrap().to_str().unwrap();
                    base_filename == "master" || base_filename == "main"
                })
                .unwrap()
                .base()
                .unwrap()
        } else {
            repo.work_dir().unwrap().to_path_buf()
        };
        println!("switch to {:?}", path);
        let repo_name = selected_str.replace('.', "_");
        if !tmux.session_exists(&repo_name) {
            tmux.new_session(Some(&repo_name), Some(path.to_str().unwrap()));
        }
        tmux.switch_to_session(&repo_name);
    }

    Ok(())
}

fn find_repos(config: &Config) -> Result<HashMap<String, Repository>> {
    let directories = config.search_dirs().change_context(TmsError::ConfigError)?;
    let mut repos = HashMap::new();
    let mut to_search: VecDeque<SearchDirectory> = directories.into();

    while let Some(file) = to_search.pop_front() {
        println!("checking {:?}", &file.path);
        let repo_name = file
            .path
            .file_name()
            .expect("a file name")
            .to_str()
            .unwrap()
            .to_string();
        if let Ok(repo) = gix::open(&file.path) {
            // println!("found repo {:?}", &file.path);
            match repo.kind() {
                gix::repository::Kind::WorkTree { is_linked } => {
                    if !is_linked {
                        println!("found repo {:?}", &repo);
                        repos.insert(repo_name, repo);
                    }
                }
                gix::repository::Kind::Bare => {}
                gix::repository::Kind::Submodule => {}
            };
        } else if file.path.is_dir() && file.depth == 0 {
            let bare_path = file.path.join(".bare");
            if let Ok(repo) = gix::open(&bare_path) {
                match repo.kind() {
                    gix::repository::Kind::WorkTree { is_linked } => {
                        if !is_linked {
                            println!("found repo {:?}", &repo);
                            repos.insert(repo_name, repo);
                        }
                    }
                    gix::repository::Kind::Bare => {}
                    gix::repository::Kind::Submodule => {}
                };
            }
        } else if file.path.is_dir() && file.depth > 0 {
            match fs::read_dir(&file.path) {
                Err(ref e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    eprintln!(
                        "Warning: insufficient permissions to read '{:?}'. Skipping directory...",
                        file.path
                    );
                }
                result => {
                    let read_dir = result
                        .change_context(TmsError::IoError)
                        .attach_printable_lazy(|| {
                            format!("Could not read directory {:?}", file.path)
                        })?
                        .map(|dir_entry| dir_entry.expect("Found non-valid utf8 path").path());
                    for dir in read_dir {
                        to_search.push_back(SearchDirectory::new(dir, file.depth - 1))
                    }
                }
            }
        }
    }

    Ok(repos)
}

fn get_single_selection(repos: &HashMap<String, Repository>) -> Result<Option<String>> {
    let mut error: Option<Report<TmsError>> = None;
    let options = SkimOptionsBuilder::default()
        .height("50%".to_string())
        .multi(false)
        .color(Some("dark".to_string()))
        .build()
        .unwrap();
    let item_reader = SkimItemReader::default();
    let mut skim_str = String::new();
    let mut repos_vec: Vec<(&String, &Repository)> = repos.iter().collect();
    repos_vec.sort_by(|a, b| a.0.cmp(b.0));
    for name in repos_vec {
        skim_str.push_str(&format!("{}\n", name.0.clone()));
    }
    let item = item_reader.of_bufread(Cursor::new(skim_str));
    let skim_output = Skim::run_with(&options, Some(item)).unwrap();
    if skim_output.is_abort {
        error = Some(TmsError::TuiError("No selection made".into()).into());
    }
    if let Some(error) = error {
        return Err(error);
    }
    Ok(Some(skim_output.selected_items[0].output().to_string()))
}
