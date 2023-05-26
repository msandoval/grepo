mod git;
extern crate confy;

#[macro_use]
extern crate serde_derive;

use clap::{AppSettings, Parser, Subcommand, Arg};
use confy::ConfyError;
use dialoguer::Confirm;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs;
use tabled::{Table, Tabled, builder::Builder, settings::Style};
use tabled::settings::Disable;
use tabled::settings::object::LastColumn;
use tabled::tables::TableValue::Column;
use crate::git::BranchInfo;

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
#[clap(version = "0.1.2")]
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
        /// (true|false) - Clear the current saved watched repo list and add only those passed in
        reset_watched: Option<bool>,
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
    Curr {},
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
    #[clap(subcommand, alias = "w")]
    Watch(WatchCmds),

    /// Commands for repo branches
    #[clap(subcommand, alias = "b")]
    Branch(BranchCmds),

    /// Replaces the watched repo list with a list from current base directory
    #[clap(alias = "sbd")]
    ScanBaseDir {},

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
        Commands::Watch(WatchCmds::Add { names, reset_watched }) => {
            let mut repos: HashSet<String> = match reset_watched {
                Some(false) | None => { cfg.clone().repos.into_iter().collect() },
                Some(true) => HashSet::new()
            };
            let valid_repos = names.split(",")
                .map(|name| name.trim().to_string())
                .filter(|name| {
                    if git::get_valid_repo(cfg.clone(), name.clone()) {
                        true
                    } else {
                        println!("Skipping {}: Not a valid repo", name.clone());
                        false
                    }
                });
            let mut new_repos = repos;
            new_repos.extend(valid_repos);
            cfg.repos = new_repos.into_iter().collect();

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
            let mut tables = Vec::new();
            found_in_repo.iter().for_each(|(key,value)| {
                tables.extend(value)
            });

            println!(
                "Search Pattern '{}' found in repos:\n{}",
                name,
                Table::new(tables).with(Style::re_structured_text()).to_string()
            )
        }
        Commands::Branch(BranchCmds::List {}) => {
            git::get_repo_branches(cfg).into_iter().for_each(|(repo,branch_list)| {
                let branches = branch_list.join("\n");
                println!("Repo: {}\n--------------------------\n{}\n", repo, branches);
            })
        }
        Commands::Branch(BranchCmds::Curr {}) => {
            git::get_current_branches(cfg).into_iter().for_each(|data| {
                println!("Repo: {}\n--------------------------\n{}\n", data.0, data.1);
            })
        }
        Commands::ScanBaseDir {} => {
            if Confirm::new().with_prompt(format!("This will reset your current watched repos with directories found in the base path ({}). Are you sure?",cfg.base_path.clone())).interact().unwrap() {
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
                    .filter_map(|repo| {
                        if git::get_valid_repo(cfg.clone(), repo.to_owned()) {
                            println!("Found repo: {}", repo.clone());
                            Some(repo)
                        } else {
                            println!("Skipping {}: Not a valid repo", repo.clone());
                            None
                        }
                    })
                    .collect::<Vec<String>>();
                confy::store(env!("CARGO_PKG_NAME"), None, &new_config);
                println!("Watched repos:\n--------------------------\n{}\n", new_config.repos.join("\n"));
            }
        }
    }
}
