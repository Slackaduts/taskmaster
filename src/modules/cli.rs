use anyhow::{Result, anyhow};

use clap::{ArgMatches, Command, value_parser, command, ColorChoice, arg};
use clap::builder::styling::{Styles, AnsiColor};

use rfd::AsyncFileDialog;
use serde_yml::Value;
use tokio::fs::create_dir;
use std::path::PathBuf;

use log::{error, info};
use indoc::indoc;

use crate::modules::utils::{create_file, 
    file_contents, 
    handle_logged_result
};
use crate::modules::task::{
    execute_tasks,
    get_task_str_sequence,
    task_defs_contents,
    get_by_path
};
use crate::modules::docs::format_element;

/// Style of the CLI.
const STYLE: Styles = Styles::styled()
.header(AnsiColor::BrightYellow.on_default())
.usage(AnsiColor::BrightYellow.on_default())
.literal(AnsiColor::White.on_default())
.placeholder(AnsiColor::BrightCyan.on_default()
);

/// Gets the clap::Command for the taskmaster cli
fn cli_cmds() -> Command {
    command!()
        .color(ColorChoice::Auto)
        .styles(STYLE)
        .arg( // Default input parameter
            clap::Arg::new("file_input")          
            .index(1)
            .required(false)
            .value_parser(value_parser!(PathBuf))
            .hide(true)
        )
        .arg(
            arg!(
            -v --verbose "Enables verbose logging"
            )
        )
        .subcommand(
            Command::new("run")
                .about("Prompts for a task file and executes the specified tasks")
                .arg(arg!(-r --report <FILE> "Custom report file")
                    .required(false)
                    .value_parser(value_parser!(PathBuf))
                )
                .arg(arg!( -f --file <FILE> "Custom task YAML file")
                    .required(false)
                    .value_parser(value_parser!(PathBuf)))
        )
        .subcommand(
            Command::new("generate-docs")
                .about("Generates documentation for defined tasks")
                .arg(
                    arg!(-t --task <NAMES> "Which task(s) to generate documentation for")
                    .num_args(1..)
                    .value_delimiter(',')
                    .value_parser(value_parser!(String))
                    .id("tasks")
                )
                .arg(
                    arg!(-o --output <DIR> "Directory to output Obsidian documentation")
                    .required(true)
                    .value_parser(value_parser!(PathBuf))
                )
        )
}

/// Top-level logic for handling CLI arguments and their functions.
pub async fn handle_cli() {
    let cmd = cli_cmds();
    let matches = cmd.get_matches();

    match matches.try_get_one::<bool>("verbose") {
        Ok(a) => if let Some(b) = a { 
            if *b { log::set_max_level(log::LevelFilter::Trace) }
        },
        Err(e) => {
            error!("Error occured when handling \"verbose\" argument, skipping: {}", anyhow!(e))
        }
    }

    match matches.subcommand_name() {
        Some("run") => {
            if let Err(err) = run_cmd(&matches).await { error!("Error occured when using \"run\" command: {}", anyhow!(err)) };
        },
        Some("generate-docs") => {
            if let Err(err) = generate_docs_cmd(&matches).await { error!("Error occured when using \"generate-docs\" command: {}", anyhow!(err)) };
        }
        Some(&_) => {
            unimplemented!("Unknown command provided.")
        }
        None => {
            if let Err(err) = run_cmd(&matches).await { error!("Error occured when using default command: {}", anyhow!(err)) };
        }
    }
}

/// Discrete logic for choosing Task YAML in the "run" command.
async fn handle_file_dialog() -> Result<PathBuf> {
    let future = async {
        let file = AsyncFileDialog::new()
            .add_filter("YAML", &["yaml", "yml"])
            .set_directory("/")
            .pick_file()
            .await;
    
        match file {
            Some(d) => {
                return Ok(d.path().to_path_buf())
            },
            None => {
                Err(anyhow!("No task YAML specified. Closing Taskmaster."))
            }
        }
    };

    let output: Result<PathBuf> = future.await;
    output
}

/// Discrete logic for choosing output directory in the "generate-docs" command.
async fn handle_directory_dialog() -> Result<PathBuf> {
    let future = async {
        let dir = AsyncFileDialog::new()
            .set_directory("/")
            .pick_folder()
            .await;
    
        match dir {
            Some(d) => {
                return Ok(d.path().to_path_buf())
            },
            None => {
                Err(anyhow!("No output folder specified. Closing Taskmaster."))
            }
        }
    };

    let output: Result<PathBuf> = future.await;
    output
}

/// Discrete logic for the "run" command.
pub async fn run_cmd(matches: &ArgMatches) -> Result<()> {
    let user_input_path = match matches.try_get_one::<PathBuf>("file_input").to_owned() {
        Ok(b) => match b {
            Some(c) => c.to_owned(),
            None => {
                handle_file_dialog().await?
            }
        },
        Err(_) => {
            let b = match matches.subcommand_matches("run") {
                Some(c) => match c.clone().try_get_one::<PathBuf>("file") {
                    Ok(d) => match d {
                        Some(e) => e.to_owned(),
                        None => {
                            handle_file_dialog().await?
                        }
                    },
                    Err(err) => {
                        error!("Error occured when retreiving \"file\" argument of \"run\" subcommand: {}",  anyhow!(err));
                        handle_file_dialog().await?
                    }
                },
                None => { 
                    handle_file_dialog().await?
                }
            };
            b

        }
    };

    let user_input = file_contents(&user_input_path).await?;
    let task_input = task_defs_contents().await?;

    let user_yaml: Value = serde_yml::from_str(&user_input)?;
    let task_yaml: Value = serde_yml::from_str(&task_input)?;

    execute_tasks(&user_yaml, &task_yaml).await?;

    Ok(())
}

/// Handles when no arguments are sipplied to the "generate-docs" command.
async fn handle_no_docs_input() -> Result<(PathBuf, Vec<String>)> {
    info!("No tasks specified. Defaulting to generating documentation for all tasks.");

    let output_dir = handle_logged_result(
        handle_directory_dialog().await,
        "Error occured when picking output directory: "
    )?;

    let tasks = get_task_str_sequence().await?;

    Ok((output_dir.clone(), tasks))
}

/// Handles arguments supplied to the "generate-docs" command.
async fn handle_docs_input(sub_matches: &ArgMatches) -> Result<(PathBuf, Vec<String>)> {
    let output_dir: PathBuf = match sub_matches.try_get_one::<PathBuf>("output") {
        Ok(a) => match a {
            Some(b) => b.to_owned(),
            None => {
                handle_directory_dialog().await?
            }
        },
        Err(e) => {
            let err = anyhow!("Error occured when processing \"output\" argument: {}", e);
            error!("{}", err);
            return Err(err)
        }
    };

    if !output_dir.exists() {
        match create_dir("").await {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("Error occured when creating output directory: {}", e))
        }
    }

    let tasks: Vec<String> = match sub_matches.try_get_many::<String>("tasks") {
        Ok(a) => match a {
            Some(b) => {
                b.map(|c| c.to_owned()).collect()
            },
            None => {
                info!("No tasks specified. Defaulting to generating documentation for all tasks.");
                get_task_str_sequence().await?
            }
        },
        Err(e) => {
            return Err(anyhow!("Error occured when processing \"tasks\" argument: {}", e))
        }
    };

    Ok((output_dir.to_owned(), tasks))
}

/// Discrete logic for the "generate-docs" command.
pub async fn generate_docs_cmd(matches: &ArgMatches) -> Result<()> {
    let (output_dir, tasks): (PathBuf, Vec<String>) = match matches.subcommand_matches("generate-docs") {
        Some(a) => {
            handle_docs_input(a).await?
        },
        None => {
            handle_no_docs_input().await?
        }
    };

    let task_file = handle_logged_result(
        task_defs_contents().await,
        "Error occured when reading task definition file: "
    )?;

    let task_defs = handle_logged_result(
        get_task_str_sequence().await, 
        "Error occured when processing task definitions: "
    )?;

    let task_defs_val: Value = handle_logged_result(
        serde_yml::from_str(&task_file).map_err(|e| anyhow!(e.to_string())),
        "Error occured when parsing task definitions"
    )?;

    for task in tasks {
        if !task_defs.contains(&task) { continue }
        let contents_val = match get_by_path(&task_defs_val, &task, None) {
            Some((a, _)) => a,
            None => {
                error!("Error occured when retreiving task data for task \"{}\", skipping.", task);
                continue
            }
        };

        let obsidian_tags = indoc! {"
            ---
            tags:
            - Taskmaster
            ---
        "}.to_owned();

        let task_md = obsidian_tags + &format_element(&contents_val, "+", 0);

        let task_file = format!("{}.md", task);
        match create_file(&task_md, output_dir.join(&task_file)).await {
            Ok(_) => info!("\"{task}\" documentation successfully written to \"{task_file}\"", ),
            Err(e) => {
                error!("Error occured when writing markdown data for task \"{}\", skipping: {}", task, e);
            }
        }
    };

    Ok(())
}