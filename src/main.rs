mod git;
extern crate confy;

#[macro_use]
extern crate serde_derive;

use clap::{AppSettings, Parser, Subcommand};
use confy::ConfyError;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs;
use dialoguer::Confirm;

const BASE_PATH: &str = "/repos";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    base_path: String,
    repos: Vec<String>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            base_path: BASE_PATH.to_string(),
            repos: Vec::new(),
        }
    }
}

#[derive(Parser, Debug)]
#[clap(name = "grepo")]
#[clap(version = "0.1.0")]
#[clap(about = "A utility to help organize and search for data in git repos")]

#[clap(setting = AppSettings::InferSubcommands)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum WatchCmds {
    /// Add a new repo to watch
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Add {
        /// Name of repo to add to the watch
        names: String,
    },
    /// Remove a repo to watch
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Remove {
        /// Name of repo to remove from watch
        names: String,
    },
    /// View a list of watched repos
    List {},
}

#[derive(Subcommand, Debug)]
enum BranchCmds {
    /// Search for a branch name in all watched repos
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Search {
        /// Pattern to look for in branch name
        name: String,
    },
    /// View a list of all branches in all watched repos
    List {},
    /// Get a list of current branches all watched repos are on
    Curr {}
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show/set base directory of repos
    BaseDir {
        /// Optional: update the default base directory of watched repos
        path: Option<String>,
    },
    /// Show a list of settings saved
    ShowConfig {},
    /// Show location of config file
    ConfigPath {},
    /// Commands for watched repos
    #[clap(subcommand, alias="w")]
    Watch(WatchCmds),
    /// Commands for repo branches
    #[clap(subcommand, alias="b")]
    Branch(BranchCmds),
    /// Replaces the watched repo list with a list from current base directory
    #[clap(alias="sbd")]
    ScanBaseDir {}
}

fn has_config(file_path: &str) -> bool {
    if let Ok(contents) = fs::read_to_string(file_path) {
        if contents.contains("base_path") && contents.contains("repos") {
            true
        } else {
            println!("File exists but is missing data");
            false
        }
    } else {
        println!("File does not exist");
        false
    }
}

fn get_config() -> Result<ConfigFile, ConfyError> {
    match confy::load(env!("CARGO_PKG_NAME"), None) {
        Ok(cf) => Ok(cf),
        Err(ConfyError::BadYamlData(_)) => {
            let newcfg = ConfigFile {
                ..Default::default()
            };
            confy::store("grepo", None, &newcfg)?;
            Ok(newcfg)
        }
        Err(e) => Err(e),
    }
}

fn main() {
    let args = Cli::parse();
    let mut cfg = get_config().expect("Retrieving config file failed");
    match args.command {
        Commands::BaseDir { path } => match path {
            None => {
                println!("{}", cfg.base_path);
            }
            Some(new_path) => {
                let new_cfg = ConfigFile {
                    base_path: new_path,
                    ..cfg
                };
                confy::store(env!("CARGO_PKG_NAME"), None, &new_cfg);
                let updated_cfg = get_config().expect("Config file update failed");
                println!(
                    "Updated base path from {} to {}",
                    cfg.base_path, updated_cfg.base_path
                );
            }
        }
        Commands::ShowConfig {} => {
            println!("{:?}", cfg);
        }
        Commands::ConfigPath {} => {
            let file = confy::get_configuration_file_path(env!("CARGO_PKG_NAME"), None)
                .expect("Failed to retrieve config file path");
            println!("{}", file.to_string_lossy())
        }
        Commands::Watch(WatchCmds::Add { names }) => {
            let mut repos: HashSet<String> = cfg.clone().repos.into_iter().collect();
            repos.extend(names.split(",").map(|name| name.trim().to_string()) );
            cfg.repos = repos.into_iter().collect::<Vec<String>>();
            confy::store(env!("CARGO_PKG_NAME"), None, &cfg).unwrap();
            println!("Updated repos: Now {:?}", cfg.repos);
        }
        Commands::Watch(WatchCmds::Remove { names }) => {
            for name in names.split(",") {
                if let Some(pos) = cfg.repos.iter().position(|s| *s == name) {
                    cfg.repos.remove(pos);
                } else {
                    println!("Repo {} is not found", name);
                }
            }
            confy::store(env!("CARGO_PKG_NAME"), None, &cfg).unwrap();
            println!("Updated repos: Now {:?}", cfg.repos);
        }
        Commands::Watch(WatchCmds::List {}) => {
            println!(
                "Watched Repos:\n--------------------------\n{}",
                cfg.repos.join("\n")
            )
        }
        Commands::Branch(BranchCmds::Search { name }) => {
            let mut found_in_repo = git::search_repos(cfg.clone(), name.clone());
            let mut repo_branch_concat = found_in_repo.iter()
                .map(|(k,v)| format!("{} - {}\n", k.clone(), v.join(",")))
                .collect::<String>();
            println!(
                "Search Pattern '{}' found in repos:\n--------------------------\n{}",
                name,
                repo_branch_concat
            )
        }
        Commands::Branch(BranchCmds::List {}) => {
            git::get_repos(cfg).into_iter().for_each(|data| {
                let branches = data.1.join("\n");
                println!("Repo: {}\n--------------------------\n{}\n", data.0, branches);
            })
        }
        Commands::Branch(BranchCmds::Curr {}) => {
            git::get_current_branches(cfg).into_iter().for_each(|data| {
                println!("Repo: {}\n--------------------------\n{}\n", data.0, data.1);
            })
        }
        Commands::ScanBaseDir {} => {
            if Confirm::new().with_prompt("This will reset your current watched repos with directories found in the base path. Are you sure?").interact().unwrap() {
                let mut new_config = ConfigFile {
                    base_path: cfg.base_path.clone(),
                    repos: vec![],
                };
                new_config.repos = fs::read_dir(cfg.clone().base_path)
                    .unwrap()
                    .into_iter()
                    .filter_map(|path|
                        if path.as_ref().unwrap().path().is_dir() {
                            let out = &path.unwrap().file_name().to_owned().into_string().unwrap();
                            Some(out.clone())
                        } else {
                            None
                        }
                    )
                    .filter(|repo| git::get_valid_repo(cfg.clone(), repo.to_owned()))
                    .collect::<Vec<String>>();
                confy::store(env!("CARGO_PKG_NAME"), None, &new_config);
                println!("Found repos:\n--------------------------\n{}\n", new_config.repos.join("\n"));
            }
        }
    }
}

