mod git;
extern crate confy;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate prettytable;
use prettytable::{Cell, Row, Table};

use clap::{AppSettings, Parser, Subcommand};
use confy::ConfyError;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs;

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
#[clap(about = "A utility to help organize and search for data in git repos")]

struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum RepoCmds {
    /// Add a new repo to watch
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Add {
        /// Name of repo to add to the watch
        name: String,
    },
    /// Remove a repo to watch
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Remove {
        /// Name of repo to remove from watch
        name: String,
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
    WatchList {},
}

#[derive(Subcommand, Debug)]
enum Commands {
    ///Show base directory to look for repos
    BaseDir {
        /// Optional: update the default base directory to search within
        name: Option<String>,
    },
    ///Show a list of settings saved
    ShowConfig {},
    ///Show location of config file
    ConfigPath {},
    #[clap(subcommand)]
    Repo(RepoCmds),
    #[clap(subcommand)]
    Branch(BranchCmds),
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

// fn create_output_table(title: String, rows: Vec<String>) -> Table {
//     let mut table = Table::new();
//     table.add_row(row![title]);
//     for row in rows {
//         table.add_row(row![row]);
//     }
//     table
// }

fn main() {
    let args = Cli::parse();
    let cfg = get_config().expect("Retrieving config file failed");
    match args.command {
        Commands::BaseDir { name } => match name {
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
        },
        Commands::ShowConfig {} => {
            println!("{:?}", cfg);
        }
        Commands::ConfigPath {} => {
            let file = confy::get_configuration_file_path(env!("CARGO_PKG_NAME"), None)
                .expect("Failed to retrieve config file path");
            println!("{}", file.to_string_lossy())
        }
        Commands::Repo(RepoCmds::Add { name }) => {
            let mut cfg = get_config().unwrap();
            cfg.repos.push(name);
            confy::store(env!("CARGO_PKG_NAME"), None, &cfg);
            println!("Updated repos: Now {:?}", cfg.repos);
        }
        Commands::Repo(RepoCmds::Remove { name }) => {
            let mut cfg = get_config().unwrap();
            let mut removed: bool = false;
            cfg.repos.retain(|s| {
                if s.to_owned() != name {
                    true
                } else {
                    removed = true;
                    false
                }
            });

            if removed {
                confy::store(env!("CARGO_PKG_NAME"), None, &cfg).unwrap();
                println!("Updated repos: Now {:?}", cfg.repos);
            } else {
                println!("Repo {} is not found", name);
            }
        }
        Commands::Repo(RepoCmds::List {}) => {
            let cfg = get_config().expect("Retrieving config file failed");
            //println!("{}", create_output_table("Repos".to_string(), cfg.repos))
            println!(
                "Watched Repos:\n--------------------------\n{}",
                cfg.repos.join("\n")
            )
        }
        Commands::Branch(BranchCmds::Search { name }) => {
            let found_in_repo = git::search_repos(cfg.clone(), name.clone());
            println!(
                "Search Pattern '{}' found in repos:\n--------------------------\n{}",
                name,
                found_in_repo.join("\n")
            )
        }
        Commands::Branch(BranchCmds::WatchList {}) => {
            let cfg = get_config().expect("Retrieving config file failed");
            git::get_repos(cfg)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn does_this_work() {
        assert_eq!(4, 4);
    }
}
