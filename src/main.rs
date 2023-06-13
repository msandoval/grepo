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
use std::rc::Rc;
use tabled::{
    settings::{
        object::Rows,
        Disable, Panel, Style, Format,
    },
    tables::ExtendedTable,
    Table, Tabled,
};
use tabled::settings::{Alignment, Modify, Padding};
use tabled::settings::object::Columns;


const BASE_PATH: &str = "/repos";

#[derive(Tabled, Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[tabled(rename = "Base Path")]
    base_path: Rc<str>,
    #[tabled(rename = "Repos", display_with = "concatenate_values")]
    repos: Vec<String>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            base_path: Rc::from(BASE_PATH),
            repos: Vec::new(),
        }
    }
}

#[derive(Parser, Debug)]
#[clap(name = "grepo")]
#[clap(version = "0.1.4")]
#[clap(author = "Manuel Sandoval")]
#[clap(about = "A utility to help organize and search for data in git repos")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum RepoCmds {
    /// Add a new repo to watch
    #[clap(arg_required_else_help = true)]
    Add {
        /// Name (or comma-delimited string) of repo(s)
        names: String,
        /// This flag will clear the current saved watched repos and add only those passed in
        #[clap(short, long)]
        reset_watched: bool,
    },
    /// Remove a watched repo
    #[clap(arg_required_else_help = true)]
    Remove {
        /// Name (or comma-delimited string) of repo(s) to remove from watch
        names: String,
    },
    /// List of watched repos
    List {},
}

#[derive(Subcommand, Debug)]
enum BranchCmds {
    /// View a list of all local branches in all watched repos
    List {},
    /// Get a list of current branches all watched repos are on
    #[clap(alias = "cur", alias = "curr")]
    Current {},
}

#[derive(Subcommand,Debug)]
enum SearchCmds {
    /// Branch search in all watched repos
    #[clap(alias = "-b", arg_required_else_help = true)]
    Branch {
        /// Search pattern
        pattern: String
    },
    /// Commit search in all watched repos
    #[clap(alias = "-c", arg_required_else_help = true)]
    Commit {
        /// Search pattern
        pattern: String,
        /// Optional: (true|false) include author name in search
        #[clap(short, long)]
        include_author: bool,
    }
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
    #[clap(subcommand, alias = "r")]
    Repo(RepoCmds),

    /// Commands for repo branches
    #[clap(subcommand, alias = "b")]
    Branch(BranchCmds),

    /// Search commands
    #[clap(subcommand, alias = "s")]
    Search(SearchCmds),

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
fn concatenate_values(values: &[String]) -> String {
    values.join("\n")
}

fn main() {
    let args = Cli::parse();
    let mut cfg = get_config().expect("Retrieving config file failed");
    match args.command {
        Commands::BaseDir { path } => match path {
            None => {
                let bold = ansi_term::Style::new().bold();
                println!("\n{} {}\n", bold.paint("Base Directory:"),&cfg.base_path);
            }
            Some(new_path) => {
                let new_cfg = ConfigFile {
                    base_path: Rc::from(new_path),
                    ..cfg
                };
                confy::store(env!("CARGO_PKG_NAME"), None, new_cfg).expect("Error writing to config file");
                let updated_cfg = get_config().expect("Config file update failed");
                println!(
                    "\nUpdated base path from {} to {}",
                    cfg.base_path, updated_cfg.base_path
                );
            }
        }

        Commands::ShowConfig {} => {
            let bold = ansi_term::Style::new().bold();
            println!("\n{} {}\n{}\n{}", bold.paint("Base Path:"), cfg.base_path, bold.paint("Watched Repos:"),cfg.repos.join("\n"))
        }

        Commands::ConfigPath {} => {
            let file = confy::get_configuration_file_path(env!("CARGO_PKG_NAME"), None)
                .expect("Failed to retrieve config file path");
            let bold = ansi_term::Style::new().bold();
            println!("\n{} {}\n", bold.paint("Config Path:"), file.to_string_lossy());
        }

        Commands::Repo(RepoCmds::Add { names, reset_watched }) => {
            let mut repos = HashSet::new();
            if !reset_watched {
                repos = cfg.clone().repos.into_iter().collect()
            }

            let valid_repos = names.split(',')
                .map(|name| name.trim().to_string())
                .filter(|name| {
                    if git::get_valid_repo(cfg.clone(), name.to_string()) {
                        true
                    } else {
                        println!("Skipping {}: Not a valid repo", name);
                        false
                    }
                })
                .collect::<HashSet<String>>();
            let mut new_repos = repos;
            new_repos.extend(valid_repos);
            cfg.repos = new_repos.into_iter().collect();

            confy::store(env!("CARGO_PKG_NAME"), None, &cfg).unwrap();

            let mut output_repos = cfg.repos.clone();
            output_repos.sort();

            let bold = ansi_term::Style::new().bold();
            println!("{}\n{}",bold.paint("Updated Watched Repos:"), output_repos.join("\n"))
        }

        Commands::Repo(RepoCmds::Remove { names }) => {
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


            let bold = ansi_term::Style::new().bold();
            println!("{}\n{}",bold.paint("Updated Watched Repos:"), output_repos.join("\n"))

        }

        Commands::Repo(RepoCmds::List {}) => {
            let mut output_repos = cfg.repos;
            output_repos.is_empty().then(|| output_repos.push("** No Repos Found **".to_string()));
            output_repos.sort();
            
            let bold = ansi_term::Style::new().bold();
            println!("{}\n{}",bold.paint("Watched Repos:"), output_repos.join("\n"))

        }

        Commands::Branch(BranchCmds::List {}) => {
            git::get_repo_branch_names(cfg).into_iter().for_each(|blist| {
                let mut output_branches = blist.branch_names();
                output_branches.is_empty().then(|| output_branches.push("** No Branches Found **".to_string()));
                output_branches.sort();

                let bold = ansi_term::Style::new().bold();
                println!(
                    "\n{}",
                    Table::new(output_branches)
                        .with(Style::empty())
                        .with(Panel::header(format!("{} {}", bold.paint("Repo:"), bold.paint(blist.repo.to_string()))))
                        .with(Disable::row(Rows::single(1)))
                        .with(Modify::new(Columns::first()).with(Padding::new(0,0,0,0)))
                )
            })
        }

        Commands::Branch(BranchCmds::Current {}) => {
            let bold = ansi_term::Style::new().bold();
            println!(
                "{}",
                Table::new(git::get_current_branch_name(cfg))
                    .with(Style::empty())
                    .with(Disable::row(Rows::single(0)))
                    .with(Modify::new(Columns::single(0))
                    .with(Alignment::left()))
                    .with(Modify::new(Columns::first()).with(Format::content(|s| bold.paint(s).to_string())))
                    .with(Modify::new(Columns::first()).with(Padding::new(0,0,0,0)))
            );
        }

        Commands::ScanBaseDir {} => {
            if Confirm::new().with_prompt(format!("This will reset your current watched repos with directories found in the base path ({}). Are you sure?",cfg.base_path.clone())).interact().unwrap() {
                let mut new_config = ConfigFile {
                    base_path: cfg.base_path.clone(),
                    repos: vec![],
                };
                new_config.repos = fs::read_dir(cfg.clone().base_path.to_string())
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
                    .with(Modify::new(Columns::first()).with(Padding::new(0,0,0,0)))
                )
            }
        }
        Commands::Search(SearchCmds::Branch { pattern}) => {
            let found_in_repo = git::search_repos(cfg.clone(), pattern.clone());
            let mut tables = Vec::new();
            found_in_repo.iter().for_each(|(_,value)| {
                tables.extend(value)
            });
            tables.sort();

            let bold = ansi_term::Style::new().bold();
            println!(
                " {} '{}' {}\n{}",
                bold.paint("Search Pattern"),
                pattern,
                bold.paint("found in repos:"),
                Table::new(tables)
                    .with(Style::empty())
                    .with(Disable::row(Rows::single(0)))
                    .with(Modify::new(Columns::first()).with(Padding::new(0,0,0,0)))
            )
        }
        Commands::Search(SearchCmds::Commit{ pattern, include_author }) => {
            let bold = ansi_term::Style::new().bold();
            match git::search_commits(cfg.clone(), pattern.clone(), include_author) {
                Ok(results) => {
                    println!(
                        "{} '{}' {}\n{}",
                        bold.paint("Search Pattern"),
                        pattern,
                        bold.paint("found in repos:"),
                        ExtendedTable::new(results)
                    )
                 },
                Err(e) => println!("Grepo Error: {}", e)
            };

        },
    }
}
