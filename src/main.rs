#![forbid(unsafe_code)]

mod modules;

// use crate::modules::windows::is_process_elevated;
use crate::modules::cli::handle_cli;
use crate::modules::logging::init_logging;


#[tokio::main]
async fn main() {
    // match is_process_elevated() { // Kill self if ran as admin
    //     Ok(a) => if a { panic!("Taskmaster should NEVER be run with administrator permissions.\n\nPlease relaunch from an unelevated shell/process.") },
    //     Err(e) => {panic!("Could not check the privileges of the current process, failed with the following error: {}", anyhow!(e)) }
    // }

    init_logging();
    handle_cli().await;
}
