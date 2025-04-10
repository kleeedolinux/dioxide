use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use std::process;

mod analyzer;
mod parser;
mod fixes;
mod config;

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Lint {
        #[clap(value_parser)]
        path: PathBuf,
        #[clap(long, short)]
        fix: bool,
        #[clap(long, short, value_parser)]
        config: Option<PathBuf>,
    },
    Init {
        #[clap(value_parser)]
        path: Option<PathBuf>,
    },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Lint { path, fix, config } => {
            println!("{} Analyzing Go code at: {}", "DIOXIDE ".green().bold(), path.display());
            let config_path = match config {
                Some(path) => path,
                None => config::find_default_config(),
            };
            
            let config = match config::load_config(&config_path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("{} Failed to load configuration: {}", "ERROR ".red().bold(), e);
                    process::exit(1);
                }
            };
            match analyzer::run_analysis(&path, &config) {
                Ok(issues) => {
                    if issues.is_empty() {
                        println!("{} No issues found!", "SUCCESS ".green().bold());
                    } else {
                        println!("{} Found {} issues ", "WARNING ".yellow().bold(), issues.len());
                        
                        for issue in &issues {
                            issue.print();
                        }
                        
                        if fix {
                            println!("{} Attempting to fix issues...", "AUTOFIX ".blue().bold());
                            match fixes::apply_fixes(&path, &issues, &config) {
                                Ok(fixed) => {
                                    if fixed > 0 {
                                        println!("{} Fixed {}/{} issues ", "SUCCESS ".green().bold(), fixed, issues.len());
                                    } else {
                                        println!("{} No issues could be fixed automatically. This may be due to complex code patterns or issues that require manual intervention.", "WARNING ".yellow().bold());
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{} Failed to apply fixes: {}", "ERROR ".red().bold(), e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{} Analysis failed: {}", "ERROR ".red().bold(), e);
                    process::exit(1);
                }
            }
        }
        Commands::Init { path } => {
            let config_path = path.unwrap_or_else(|| PathBuf::from("dioxide.toml "));
            match config::create_default_config(&config_path) {
                Ok(_) => {
                    println!("{} Created configuration file at: {}", "SUCCESS ".green().bold(), config_path.display());
                }
                Err(e) => {
                    eprintln!("{} Failed to create configuration: {}", "ERROR ".red().bold(), e);
                    process::exit(1);
                }
            }
        }
    }
}
