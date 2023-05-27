use std::{collections::HashMap, fmt};
use crate::ConfigFile;
use git2::{ErrorCode, Repository, Commit, ObjectType};
use std::path::PathBuf;
use tabled::Tabled;

#[derive(Tabled, Debug)]
pub struct RepoBranchCommit {
    pub repo: String,
    pub branch: String,
    pub commit: String,
    pub author: String,
    pub message: String,
}
#[derive(Tabled, Clone)]
pub struct BranchInfo {
    pub repo: String,
    pub branch: String,
}

#[derive(Clone)]
pub struct BranchInfoList {
    pub repo: String,
    pub collection: Vec<BranchInfo>
}
impl BranchInfoList {
    pub fn branch_names(&self) -> Vec<String> {
        self.collection.iter().map(|bi| bi.branch.clone()).collect()
    }
}

struct GitRepo {
    config: ConfigFile,
    repo_name: String
}

#[derive(Debug)]
pub enum RepoError {
    OpenFailure(String),
}
impl fmt::Display for RepoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RepoError::OpenFailure(repo) => write!(f, "Could not open git repo at {}", repo),
        }
    }
}

#[derive(Debug)]
pub enum GrepoError {
    Repo(RepoError),
    Branch(BranchError),
    Commit(CommitError),
}
impl fmt::Display for GrepoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GrepoError::Repo(ref error) => write!(f, "Repo failed: {}", error),
            GrepoError::Branch(ref error) => write!(f, "Branch error: {}", error),
            GrepoError::Commit(ref error) => write!(f, "Commit error: {}", error),
        }
    }
}

#[derive(Debug)]
pub enum BranchError {
    NameError(String, String),
}
impl fmt::Display for BranchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BranchError::NameError(error, repo) => write!(f, "Could not open branch in repo {}: {}", repo, error),
        }
    }
}

#[derive(Debug)]
pub enum CommitError {
    RevWalkFailure(String)
}
impl fmt::Display for CommitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommitError::RevWalkFailure(error) => write!(f, "Commit search failed: {}", error),
        }
    }
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
    fn open(&mut self) -> Result<Repository, GrepoError> {
        let repo_path_str = format!("{}/{}", self.config.base_path, self.repo_name.clone());
        let repo_path = PathBuf::from(repo_path_str.clone());
        match Repository::open(repo_path) {
            Ok(repo) => Ok(repo),
            Err(_) => Err(GrepoError::Repo(RepoError::OpenFailure(repo_path_str))),
        }
    }
    /// Get all local branches
    fn all_branches(&mut self) -> BranchInfoList {
        BranchInfoList { 
            repo: self.repo_name.clone(),
            collection: self.open()
                .expect("Failed to open git repo")
                .branches(Some(git2::BranchType::Local))
                .unwrap()
                .map(|b| {
                    let (branch, _) = b.expect("Expected branch error");
                    let branch_name = branch.name().unwrap().unwrap().to_owned();

                    BranchInfo {
                        repo: self.repo_name.clone(),
                        branch: branch_name,
                    }

                })
                .collect()
        }
    }
    /// Get current checked out branch for repo
    fn current_branch_name(&mut self) -> String {
        let repo = self.open().expect("Failed to open git repo");
        let head = match repo.head() {
            Ok(head) => Some(head),
            Err(ref e) if e.code() == ErrorCode::UnbornBranch || e.code() == ErrorCode::NotFound => {
                None
            }
            Err(e) => panic!("Error occurred: {}", e) //return Err(e),
        };
        let head = head.as_ref().and_then(|h| h.shorthand());
        head.unwrap_or("** Not currently on any branch **").to_string()
    }
}


pub fn get_repo_branch_names(cfg: ConfigFile) -> Vec<BranchInfoList> {
    cfg.repos
        .clone()
        .into_iter()
        .map(|repo| {
            GitRepo::new(cfg.clone(), repo.clone()).all_branches()
        })
        .collect()
}

pub fn search_repos(cfg: ConfigFile, name: String) -> HashMap<String, Vec<BranchInfo>> {
    cfg.clone().repos.into_iter().filter_map(|repo| {
        let branches = GitRepo::new(cfg.clone(), repo.clone()).all_branches();
        let filtered_branches: Vec<BranchInfo> = branches.collection.into_iter()
            .filter(|branch| {
                branch.branch.contains(&name)
            })
            .collect();
        if !filtered_branches.is_empty() {
            Some((repo, filtered_branches))
        } else {
            None
        }
    }).collect::<HashMap<String, Vec<BranchInfo>>>()
}

pub fn get_current_branch_name(cfg: ConfigFile) -> Vec<BranchInfo> {
    cfg.repos
        .clone()
        .into_iter()
        .map(|repo| BranchInfo { 
            repo: repo.clone(), 
            branch: GitRepo::new(cfg.clone(), repo).current_branch_name() 
        } )
        .collect::<Vec<BranchInfo>>()
}

pub fn get_valid_repo(cfg: ConfigFile, repo_name: String) -> bool {
    match GitRepo::new(cfg, repo_name).open() {
        Ok(_) => { true }
        Err(_) => { false }
    }
}

pub fn search_commits(cfg: ConfigFile, search_string: String, include_author: bool) -> Result<Vec<RepoBranchCommit>, GrepoError> {
    let repo_names = cfg.repos.clone();
    let mut found_commits = Vec::new();

    for repo_name in repo_names {
        let mut watchobj = GitRepo::new(cfg.clone(), repo_name.clone());
        let repo = match watchobj.open() {
            Ok(r) => { r },
            Err(_) => { continue },
        };
        
        for branches in repo.branches(Some(git2::BranchType::Local)).unwrap() {
            let branch = match branches {
                Ok((b,_)) => { b },
                Err(_) => { continue },
            };

            let branch_name = match branch.name() {
                Ok(n) => { 
                    match n {
                        Some(name) => { name.to_string() },
                        None => { continue },
                    }},
                Err(e) => { return Err(GrepoError::Branch(BranchError::NameError(e.to_string(), repo_name)))},
            };

            let commit_id = branch.into_reference().peel(ObjectType::Commit).expect("peeling branch failed!").id();
            let mut revwalk = repo.revwalk().map_err(|e| GrepoError::Commit(CommitError::RevWalkFailure(e.to_string())))?;
            revwalk.push(commit_id).unwrap();

            let commits: Vec<Commit> = revwalk
                .filter_map(|oid| oid.ok())
                .filter_map(|oid| {
                    repo.find_commit(oid).ok()
                })
                .collect();


            found_commits.extend(commits.into_iter().filter( |commit| {
                    let message = commit.message().unwrap_or("");
                    (include_author && commit.author().to_string().contains(&search_string))
                        || message.contains(&search_string)
                })
                .map(|commit| {
                    RepoBranchCommit {
                        repo: repo_name.clone(),
                        branch: branch_name.clone(),
                        message: commit.message().unwrap_or("").trim().to_string(),
                        author: commit.author().to_string(),
                        commit: commit.id().to_string(),
                    }
                })
                .collect::<Vec<RepoBranchCommit>>());
        }
    }
    Ok(found_commits)
}
