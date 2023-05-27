mod git;
extern crate confy;

#[macro_use]
extern crate serde_derive;

use clap::{Parser, Subcommand};
use confy::ConfyError;
use dialoguer::Confirm;
use std::collections::HashSet;
use std::fmt::Debug;
use std::fs;
use tabled::{
    settings::{
        object::Rows,
        Disable, Panel, Style,
    },
    tables::ExtendedTable,
    Table, Tabled,
};

const BASE_PATH: &str = "/repos";

#[derive(Tabled, Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[tabled(rename = "Base Path")]
    base_path: String,
    #[tabled(rename = "Repos", display_with = "concatenate_values")]
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
#[clap(version = "0.1.3")]
#[clap(author = "Manuel Sandoval")]
#[clap(about = "A utility to help organize and search for data in git repos")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum WatchCmds {
    /// Add a new repo to watch
    #[clap(arg_required_else_help = true)]
    Add {
        /// Name (or comma-delimited string) of repo(s) to add to the watch
        names: String,
        /// This flag will clear the current saved watched repos and add only those passed in
        #[clap(short, long)]
        reset_watched: bool,
    },
    /// Remove a repo to watch
    #[clap(arg_required_else_help = true)]
    Remove {
        /// Name (or comma-delimited string) of repo(s) to remove from watch
        names: String,
    },
    /// View a list of watched repos
    List {},
}

#[derive(Subcommand, Debug)]
enum BranchCmds {
    /// Search for a branch name in all watched repos
    #[clap(arg_required_else_help = true)]
    Search {
        /// Pattern to look for in branch name in local branches
        pattern: String,
    },
    /// View a list of all local branches in all watched repos
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

    /// Commands for repo commits
    #[clap(subcommand, alias = "c")]
    Commit(CommitCmds),

    /// Replaces the watched repo list with a list from current base directory
    #[clap(alias = "sbd")]
    ScanBaseDir {},
}

#[derive(Subcommand, Debug)]
enum CommitCmds {
    /// Search for a commits messages in all watched repos
    #[clap(arg_required_else_help = true)]
    Search {
        /// Pattern to look for in commit message
        pattern: String,
        #[clap(short, long)]
        include_author: bool,
    },
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
fn concatenate_values(values: &Vec<String>) -> String {
    values.join("\n")
}

fn table_mini(header: &str, data: &str) {
    let padding = 2;
    println!(
        "{}\n {}\n{}\n {}\n{}",
        "=".repeat(data.len() + padding),
        header,
        "=".repeat(data.len() + padding),
        data,
        "=".repeat(data.len() + padding)
    )
}

fn main() {
    let args = Cli::parse();
    let mut cfg = get_config().expect("Retrieving config file failed");
    match args.command {
        Commands::BaseDir { path } => match path {
            None => {
                table_mini("Base Directory:", &cfg.base_path)
            }
            Some(new_path) => {
                let new_cfg = ConfigFile {
                    base_path: new_path,
                    ..cfg
                };
                confy::store(env!("CARGO_PKG_NAME"), None, new_cfg).expect("Error writing to config file");
                let updated_cfg = get_config().expect("Config file update failed");
                println!(
                    "Updated base path from {} to {}",
                    cfg.base_path, updated_cfg.base_path
                );
            }
        }

        Commands::ShowConfig {} => {
            println!(
                "{}", 
                Table::new(vec![cfg])
                .with(Style::re_structured_text())
            )
        }

        Commands::ConfigPath {} => {
            let file = confy::get_configuration_file_path(env!("CARGO_PKG_NAME"), None)
                .expect("Failed to retrieve config file path");
            println!(
                "{}", 
                Table::new(vec![String::from(file.to_string_lossy())])
                .with(Style::re_structured_text())
                .with(Panel::header("Config Path:"))
                .with(Disable::row(Rows::single(1)))
            )
        }

        Commands::Watch(WatchCmds::Add { names, reset_watched }) => {
            let mut repos = HashSet::new();
            if !reset_watched {
                repos = cfg.clone().repos.into_iter().collect()
            }

            let valid_repos = names.split(',')
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

            let mut output_repos = cfg.repos.clone();
            output_repos.sort();

            println!(
                "{}", 
                Table::new(output_repos)
                .with(Style::re_structured_text())
                .with(Panel::header("Updated Repo List:"))
                .with(Disable::row(Rows::single(1)))
            )
        }

        Commands::Watch(WatchCmds::Remove { names }) => {
            for name in names.split(',') {
                if let Some(pos) = cfg.repos.iter().position(|s| *s == name) {
                    cfg.repos.remove(pos);
                } else {
                    println!("Repo {} is not found", name);
                }
            }
            
            confy::store(env!("CARGO_PKG_NAME"), None, &cfg).unwrap();

            let mut output_repos = cfg.repos;
            output_repos.is_empty().then(|| output_repos.push("** No Repos Found **".to_string()));
            output_repos.sort();

            println!(
                "{}", 
                Table::new(output_repos)
                .with(Style::re_structured_text())
                .with(Panel::header("Updated Repo List:"))
                .with(Disable::row(Rows::single(1)))
            )
        }

        Commands::Watch(WatchCmds::List {}) => {
            let mut output_repos = cfg.repos;
            output_repos.is_empty().then(|| output_repos.push("** No Repos Found **".to_string()));
            output_repos.sort();
            
            println!(
                "{}", 
                Table::new(output_repos)
                .with(Style::re_structured_text())
                .with(Panel::header("Watched Repos:"))
                .with(Disable::row(Rows::single(1)))
            )
        }

        Commands::Branch(BranchCmds::Search { pattern }) => {
            let found_in_repo = git::search_repos(cfg.clone(), pattern.clone());
            let mut tables = Vec::new();
            found_in_repo.iter().for_each(|(_,value)| {
                tables.extend(value)
            });

            println!(
                "Search Pattern '{}' found in repos:\n{}",
                pattern,
                Table::new(tables).with(Style::re_structured_text())
            )
        }

        Commands::Branch(BranchCmds::List {}) => {
            git::get_repo_branch_names(cfg).into_iter().for_each(|blist| {
                let mut output_branches = blist.branch_names();
                output_branches.is_empty().then(|| output_branches.push("** No Branches Found **".to_string()));
                output_branches.sort();

                println!(
                    "{}",
                    Table::new(output_branches)
                        .with(Style::re_structured_text())
                        .with(Panel::header(format!("Repo: {}", blist.repo)))
                        .with(Disable::row(Rows::single(1)))
                )
            })
        }

        Commands::Branch(BranchCmds::Curr {}) => {
            git::get_current_branch_name(cfg).into_iter().for_each(|branch_info| {
                println!(
                    "{}",
                    ExtendedTable::new(vec![branch_info])
                );
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
                    .filter_map(|path|
                        if path.as_ref().unwrap().path().is_dir() {
                            let out = &path.unwrap().file_name().into_string().unwrap();
                            Some(out.clone())
                        } else {
                            None
                        }
                    )
                    .filter_map(|repo| {
                        if git::get_valid_repo(cfg.clone(), repo.to_owned()) {
                            println!("Found repo: {}", repo);
                            Some(repo)
                        } else {
                            println!("Skipping {}: Not a valid repo", repo);
                            None
                        }
                    })
                    .collect::<Vec<String>>();
                confy::store(env!("CARGO_PKG_NAME"), None, &new_config).expect("Error writing to config file");

                let mut output_repos = new_config.repos;
                output_repos.is_empty().then(|| output_repos.push("** No Repos Found **".to_string()));
                output_repos.sort();
                
                println!(
                    "{}", 
                    Table::new(output_repos)
                    .with(Style::re_structured_text())
                    .with(Panel::header("Watched Repos:"))
                    .with(Disable::row(Rows::single(1)))
                )
            }
        }

        Commands::Commit(CommitCmds::Search{ pattern, include_author }) => {
            match git::search_commits(cfg.clone(), pattern.clone(), include_author) {
                Ok(results) => {
                    println!(
                        "Search Pattern '{}' found in repos:\n{}",
                        pattern,
                        ExtendedTable::new(results)
                    )
                 },
                Err(e) => println!("Grepo Error: {}", e)
            };

        },
    }
}
