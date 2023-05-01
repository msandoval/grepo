use std::collections::HashMap;
use crate::ConfigFile;
use git2::{ErrorCode, Repository};
use std::ffi::CString;
use std::path::PathBuf;

struct GitRepo {
    repo_path: PathBuf,
}

impl GitRepo {
    /// Open a Git repo
    fn open(&self) -> Repository {
        let found_repo = match Repository::open(&self.repo_path) {
            Ok(repo) => repo,
            Err(e) => panic!("failed to open: {}", e),
        };
        found_repo
    }
}

fn get_branches(cfg: ConfigFile, repo: String) -> Vec<String> {
    let repo_path = PathBuf::from(format!("{}/{}", cfg.base_path, repo));
    GitRepo { repo_path }
        .open()
        .branches(Some(git2::BranchType::Local))
        .unwrap()
        .map(|b| b.unwrap().0.name().unwrap().unwrap().to_owned())
        .collect()
}
fn get_repo_branch(cfg: ConfigFile, repo: String) -> String {
    let repo_path = PathBuf::from(format!("{}/{}", cfg.base_path, repo));
    let repo = GitRepo { repo_path }.open();
    let head = match repo.head() {
        Ok(head) => Some(head),
        Err(ref e) if e.code() == ErrorCode::UnbornBranch || e.code() == ErrorCode::NotFound => {
            None
        }
        Err(e) => panic!("Error occurred: {}", e) //return Err(e),
    };
    let head = head.as_ref().and_then(|h| h.shorthand());
    head.unwrap_or("Not currently on any branch").to_string()
}


pub fn get_repos(cfg: ConfigFile) {
    cfg.repos.clone().into_iter().for_each(|repo| {
        let branches = get_branches(cfg.clone(), repo.clone()).join("\n");
        println!("Repo: {}\n--------------------------\n{}\n", repo, branches);
    });
}

pub fn search_repos(cfg: ConfigFile, name: String) -> HashMap<String, Vec<String>> {
    let repo_branches = cfg.clone().repos.into_iter().filter_map(|repo| {
        let branches = get_branches(cfg.clone(), repo.clone());
        let filtered_branches: Vec<String> = branches.into_iter()
            .filter(|branch| branch.contains(&name))
            .collect();
        if !filtered_branches.is_empty() {
            Some((repo, filtered_branches))
        } else {
            None
        }
    }).collect::<HashMap<String, Vec<String>>>();
    repo_branches
}
pub fn get_curr_branches(cfg: ConfigFile) -> Vec<(String, String)> {
    cfg.repos
        .clone()
        .into_iter()
        .map(|repo| { (repo.clone(), get_repo_branch(cfg.clone(), repo.clone()) ) })
        .collect::<Vec<(String,String)>>()
}