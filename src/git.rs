use std::collections::HashMap;
use crate::ConfigFile;
use git2::{ErrorCode, Repository};
use std::ffi::CString;
use std::path::PathBuf;

struct GitRepo {
    config: ConfigFile,
    repo_name: String
}

impl GitRepo {
    /// Create new GitRepo
    fn new(config: ConfigFile, repo_name: String) -> GitRepo {
        Self {
            config,
            repo_name,
        }
    }
    /// Open a Git repository and return object
    fn open(&mut self) -> Repository {
        let repo_path = PathBuf::from(format!("{}/{}", self.config.base_path, self.repo_name.clone()));
        let found_repo = match Repository::open(repo_path) {
            Ok(repo) => repo,
            Err(e) => panic!("failed to open: {}", e),
        };
        found_repo
    }
    fn all_branches(&mut self) -> Vec<String> {
        self.open()
            .branches(Some(git2::BranchType::Local))
            .unwrap()
            .map(|b| b.unwrap().0.name().unwrap().unwrap().to_owned())
            .collect()
    }
    fn current_branch(&mut self) -> String {
        let repo = self.open();
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
}

pub fn get_repos(cfg: ConfigFile) -> Vec<(String, Vec<String>)> {
    cfg.repos
        .clone()
        .into_iter()
        .map(|repo| { let branches = GitRepo::new(cfg.clone(), repo.clone()).all_branches(); (repo, branches)})
        .collect::<Vec<(String,Vec<String>)>>()
}

pub fn search_repos(cfg: ConfigFile, name: String) -> HashMap<String, Vec<String>> {
    let repo_branches = cfg.clone().repos.into_iter().filter_map(|repo| {
        let branches = GitRepo::new(cfg.clone(), repo.clone()).all_branches();
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

pub fn get_current_branches(cfg: ConfigFile) -> Vec<(String, String)> {
    cfg.repos
        .clone()
        .into_iter()
        .map(|repo| { (repo.clone(), GitRepo::new(cfg.clone(),repo.clone()).current_branch() ) })
        .collect::<Vec<(String,String)>>()
}