use std::collections::HashMap;
use crate::ConfigFile;
use git2::{ErrorCode, Repository};
use std::path::PathBuf;
use chrono::{NaiveDateTime, Utc, DateTime, Local};
use tabled::{Table, Tabled};

#[derive(Tabled)]
pub struct BranchInfo {
    pub repo: String,
    pub branch: String,
}

struct GitRepo {
    config: ConfigFile,
    repo_name: String
}
#[derive(Debug)]
pub enum GitRepoErr {
    OpenFailure
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
    fn open(&mut self) -> Result<Repository, GitRepoErr> {
        let repo_path = PathBuf::from(format!("{}/{}", self.config.base_path, self.repo_name.clone()));
        let found_repo = match Repository::open(repo_path) {
            Ok(repo) => Ok(repo),
            Err(e) => Err(GitRepoErr::OpenFailure),
        };
        found_repo
    }
    fn all_branches(&mut self) -> Vec<BranchInfo> {
        self.open()
            .expect("Failed to open git repo")
            .branches(Some(git2::BranchType::Local))
            .unwrap()
            .map(|b| {
                let (branch, _) = b.expect("Expected branch error");
                let branch_name = branch.name().unwrap().unwrap().to_owned();

                BranchInfo{
                    repo: self.repo_name.clone(),
                    branch: branch_name,
                }
            })
            .collect()
    }
    fn current_branch(&mut self) -> String {
        let repo = self.open().expect("Failed to open git repo");
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

pub fn get_repo_branches(cfg: ConfigFile) -> Vec<(String, Vec<String>)> {
    cfg.repos
        .clone()
        .into_iter()
        .map(|repo| {
            let branches = GitRepo::new(cfg.clone(), repo.clone()).all_branches();
            (repo, branches.into_iter().map(|b| b.branch).collect())
        })
        .collect::<Vec<(String,Vec<String>)>>()
}

pub fn search_repos(cfg: ConfigFile, name: String) -> HashMap<String, Vec<BranchInfo>> {
    let repo_branches = cfg.clone().repos.into_iter().filter_map(|repo| {
        let branches = GitRepo::new(cfg.clone(), repo.clone()).all_branches();
        let filtered_branches: Vec<BranchInfo> = branches.into_iter()
            .filter(|branch| {
                branch.branch.contains(&name)
            })
            .collect();
        if !filtered_branches.is_empty() {
            Some((repo, filtered_branches))
        } else {
            None
        }
    }).collect::<HashMap<String, Vec<BranchInfo>>>();
    repo_branches
}

pub fn get_current_branches(cfg: ConfigFile) -> Vec<(String, String)> {
    cfg.repos
        .clone()
        .into_iter()
        .map(|repo| { (repo.clone(), GitRepo::new(cfg.clone(),repo.clone()).current_branch() ) })
        .collect::<Vec<(String,String)>>()
}

pub fn get_valid_repo(cfg: ConfigFile, repo_name: String) -> bool {
    match GitRepo::new(cfg, repo_name).open() {
        Ok(_) => { true }
        Err(_) => { false }
    }
}