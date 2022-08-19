use anyhow::anyhow;
use anyhow::Result;
use git2::Repository;
use shellexpand::tilde;
use skim::prelude::Skim;
use skim::prelude::SkimItemReader;
use skim::prelude::SkimOptionsBuilder;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process;
use tmux_sessionizer::cli::create_app;

fn main() -> Result<(), Box<dyn Error>> {
    let _cli_args = create_app();

    let mut repo_list = Vec::new();
    repo_list.push(PathBuf::from(
        tilde("~/repos/personal").to_string().as_str(),
    ));
    repo_list.push(PathBuf::from(tilde("~/repos/work").to_string().as_str()));

    let repos = find_repos(repo_list)?;
    let repo_name = get_single_selection(&repos)?;

    //let found_repo = repos
    //    .find_repo(&repo_name)
    //    .context("Could not find the internal representation of the selected repository")
    //    .unwrap();
    let sessions = String::from_utf8(execute_tmux_command("tmux list-sessions -F #S")?.stdout)?;
    let mut sessions = sessions.lines();
    let session_previously_existed = sessions.any(|line| {
        // tmux will return the output with extra ' and \n characters
        line.to_owned().retain(|char| char != '\'' && char != '\n');
        line == repo_name
    });
    let found_repo = repos.get(&OsString::from(&repo_name)).unwrap();

    let head = found_repo.head()?;
    let path_to_default_tree = format!(
        "{}/{}",
        found_repo.path().parent().unwrap().to_string_lossy(),
        head.shorthand().unwrap()
    );

    let path = if found_repo.path().file_name().unwrap() == OsStr::new(".bare") {
        if found_repo.worktrees()?.is_empty() {
            std::path::Path::new(&path_to_default_tree)
        } else {
            found_repo.path().parent().unwrap()
        }
    } else {
        found_repo.path().parent().unwrap()
    };

    let path_str = path.to_string_lossy();
    if !session_previously_existed {
        execute_tmux_command(&format!("tmux new-session -ds {repo_name} -c {path_str}",))?;
        //set_up_tmux_env(found_repo, &repo_name)?;
    }

    match env::var("TMUX") {
        Ok(_val) => {
            execute_tmux_command(&format!(
                "tmux switch-client -t {}",
                repo_name.replace('.', "_"),
            ))?;
        }
        Err(_e) => {
            execute_tmux_command(&format!(
                "tmux attach-session -t {} -d",
                repo_name.replace('.', "_"),
            ))?;
        }
    }
    //println!("{}", found_repo.path().display());
    //println!("is-bare {}", found_repo.is_bare());
    //println!("is-worktree {}", found_repo.is_worktree());
    //println!("worktrees {}", found_repo.worktrees()?.get(0).unwrap());
    //println!("path {}", path.display());
    Ok(())
}

fn find_repos(paths: Vec<PathBuf>) -> Result<HashMap<OsString, Repository>> {
    let mut repos = HashMap::new();
    let mut to_search = VecDeque::new();

    paths
        .iter()
        //.for_each(|path| to_search.push_back(std::path::PathBuf::from(path)));
        .for_each(|path| to_search.push_back(PathBuf::from(path)));

    while let Some(file) = to_search.pop_front() {
        // check if dir exists
        if file.is_dir() {
            //println!("file: {}", file.clone().to_str().unwrap());
            fs::read_dir(&file)?.for_each(|path| {
                // path to add to fuzzy finder
                let name = path.as_ref().unwrap().file_name().to_os_string();
                //let path_str = path.as_ref().unwrap().path().into_os_string();
                let path_with_dot_bar_str =
                    PathBuf::from(path.as_ref().unwrap().path().join(".bare"));
                //println!("{}", name.to_string_lossy());
                //println!(".bare - {}", path_with_dot_bar_str.to_string_lossy());

                if let Ok(repo) = git2::Repository::open(path.unwrap().path().clone()) {
                    //println!("repo");
                    // if the dir is arepo add it
                    repos.insert(name, repo);
                } else if path_with_dot_bar_str.is_dir() {
                    //println!(".bare");
                    // check for .bare folder
                    let repo =
                        git2::Repository::open(PathBuf::from(path_with_dot_bar_str)).unwrap();
                    repos.insert(name, repo);
                } else {
                    // unhandled type
                }
            });
        }
    }

    //println!("Done");

    Ok(repos)
}

fn get_single_selection(repos: &HashMap<OsString, Repository>) -> Result<String> {
    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .color(Some("dark"))
        .build()
        .unwrap();
    let item_reader = SkimItemReader::default();
    let mut skim_str = String::new();
    let mut repos_vec: Vec<(&OsString, &Repository)> = repos.iter().collect();
    repos_vec.sort_by(|a, b| a.0.cmp(b.0));
    for name in repos_vec {
        skim_str.push_str(&format!("{}\n", name.0.clone().into_string().unwrap()));
    }
    let item = item_reader.of_bufread(Cursor::new(skim_str));
    let skim_output = Skim::run_with(&options, Some(item)).unwrap();
    if skim_output.is_abort {
        return Err(anyhow!("No selection made"));
    }
    Ok(skim_output.selected_items[0].output().to_string())
}

fn execute_tmux_command(command: &str) -> Result<process::Output> {
    let args: Vec<&str> = command.split(' ').skip(1).collect();
    Ok(process::Command::new("tmux")
        .args(args)
        .stdin(process::Stdio::inherit())
        .output()
        .unwrap_or_else(|_| panic!("Failed to execute the tmux command `{command}`")))
}
