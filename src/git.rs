use std::ffi::CString;
use std::path::PathBuf;
use git2::Repository;
use crate::ConfigFile;

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

pub fn get_repos(cfg: ConfigFile) {
    cfg.repos
        .clone()
        .into_iter()
        .for_each(|repo| {
            let branches = get_branches(cfg.clone(), repo.clone()).join("\n");
            println!("Repo: {}\n--------------------------\n{}\n", repo, branches);
        });
}

pub fn search_repos(cfg: ConfigFile, name: String) -> Vec<String> {
    cfg.clone().repos
        .into_iter()
        .filter(|repo| get_branches(cfg.clone(), repo.clone()).into_iter().any(|branch| branch.contains(&name)))
        .collect()
}