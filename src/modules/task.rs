use anyhow::{anyhow, Result};

use crate::modules::utils::{
    create_file, 
    sanitize_yaml, 
    sanitize_value, 
    sanitize_string, 
    file_contents,
    handle_logged_result
};

use serde_json;
use serde_yml::{Sequence, Value, Mapping};

use tokio::sync::mpsc::Sender;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::fs::read_to_string;

use warp::Filter;
use warp::http::StatusCode;

use sha256::digest;

use std::{
    env, 
    process::Stdio, 
    str,
    sync::Arc,
    path::{Path, PathBuf}
};

use log::{error, info};

use super::utils::delete_file;


/// Spawns a powershell process with a given script block in string slice form.
pub async fn spawn_powershell(script: &str, elevated: bool) -> Result<String> {
    let mut cmd = Command::new("powershell.exe");

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let script_dir = env::current_dir()?.join(".tm_temp\\");

    let script_path_raw = script_dir.join("temp.ps1");
    let script_path = script_path_raw.to_str().ok_or(anyhow!("Could not convert temp script path to string. Script Content: \n\"{}\"", script))?;

    create_file(script, PathBuf::from(script_path)).await?;

    let mut inner_proc_cmd = format!("'-NoProfile -ExecutionPolicy Bypass -File \"{}\"'", script_path);
    
    if elevated { 
        inner_proc_cmd += " -Verb RunAs";
    };

    let start_process_cmd = format!("Start-Process powershell.exe -ArgumentList {} -Wait", inner_proc_cmd);

    cmd.arg("-Command");
    cmd.arg(&start_process_cmd);
    let output = cmd.spawn()?.wait_with_output().await?;

    delete_file(PathBuf::from(script_path)).await?;

    Ok(String::from_utf8(output.stdout)?)

}

/// Gets the location of the tasks folder relative to the current exe. Returns ps1 path from there.
pub fn get_task_script(name: &str, ext: Option<&str>) -> Result<PathBuf> {
    let ext_str = ext.unwrap_or("ps1");
    let rel_script_path = PathBuf::from(&format!("tasks\\{}.{}", name, ext_str));

    let exe_path = env::current_dir()?;
    let script_path = exe_path.join(rel_script_path);

    Ok(script_path)
}

/// Retreives user data from a serde data structure.
pub fn user_input_by_path(user_data: &Value, task_data: &Value, path: &str) -> Option<(Value, String)> {
    if path.is_empty() { return Some((user_data.clone(), path.to_owned())) }
    let mut split_path: Vec<&str> = path.split('/').collect();

    if split_path.is_empty() { return Some((user_data.clone(), String::new())) }

    // let raw_next_part: &str = split_path.remove(0);
    let next_part: String = sanitize_string(split_path.remove(0)); //Retreive relevant portion of path
    let path_remains = split_path.join("/"); 

    //Actual recursive part, searching through array indexes is not a planned feature so this will suffice
    if user_data.is_mapping() && task_data.is_mapping() {
        let user_map = user_data.as_mapping()?;
        let task_map = task_data.as_mapping()?;

        // If task somehow cannot find the path part, we screwed up the path or gave a task bad data
        let task_inner = match task_map.get(&next_part) { Some(a) => a, None => return None };

        // Retrive aliases from task data
        let mut aliases = Sequence::new();
        aliases.push(Value::String(next_part));
        // Sanitize all aliases
        if let Some(a) = task_inner.get(Value::String("aliases".to_string())) { for b in a.as_sequence()? { aliases.push(sanitize_value(b)?) } }

        // See if user input matches any aliases, rerun with inner data if so
        for a in aliases {
            let user_inner = match user_map.get(a) { Some(b) => b.clone(), None => continue };
            return user_input_by_path(&user_inner, task_inner, &path_remains);
        }
    }

    Some((user_data.clone(), path_remains))
}

/// Retreive data from a task via a string path.
///
/// # Example
/// "MapDrive/Elevated" would evaluate to True/False
pub fn get_by_path(data: &Value, path: &str, parent: Option<&Value>) -> Option<(Value, Value)> {
    if path.is_empty() || !data.is_mapping() { return Some((data.clone(), parent?.clone())) };
    let mut split_path: Vec<&str> = path.split('/').collect();

    if split_path.is_empty() { return Some((data.clone(), parent?.clone())) } //Return early if path is empty, means we already have the data.
    let raw_next_part: &str = split_path.remove(0);
    let next_part = sanitize_string(raw_next_part); //Retreive relevant portion of path
    let path_remains = split_path.join("/");

    if !data.is_mapping() { return Some((data.clone(), parent?.clone())) }

    //If it already exists in the data, don't bother checking aliases
    if let Some(a) = data.get(&next_part) {
        let b = Value::String(next_part.to_string());
        return get_by_path(a, &path_remains, Some(&b))
    }

    let mut aliases: Vec<String> = Vec::new();

    for key in data.as_mapping()?.keys() {
        let entry = data.get(key)?;
        if let Some(a) = entry.get("+Aliases") {
            let mut alias_seq = Vec::new();
            for b in a.as_sequence()? {
                alias_seq.push(
                    match b.as_str() { Some(c) => c.to_owned(), None => continue }   
                )
            }

            if !alias_seq.contains(&raw_next_part.to_owned()) {
                continue
            }

            aliases.extend(alias_seq);
                break
            }
        }


    for a in &aliases {
        match data.get(a) {
            Some(b) => {
                let c = Value::String(a.to_owned());
                return get_by_path(b, &path_remains, Some(&c))
            },
            None => continue
        }
    }

    None
}

/// Returns a tuple of the task source code with injected data, and a hash which is the task ID.
pub fn prep_passthru_args(user_data: &Value, task_data: &Value) -> Option<(String, String)> {
    // let task_name = get_task_name(task_data)?;
    let passthru_path = "Passthru/";

    let (passthru_data, _) = user_input_by_path(user_data, task_data, passthru_path)?;

    let output_ps: Option<String>;
    let task_id: String;
    match serde_json::to_string(&passthru_data) {
        Ok(a) => { 
            output_ps = Some(a.clone());
            task_id = digest(a).to_ascii_uppercase();
        },
        Err(e) => {
            error!("Error occured: {}", anyhow!(e));
            return None;
        }
    }

    output_ps.map(|a| (format!("$taskData = '{}'\r\n$taskId = '{}'\r\n", a, &task_id), task_id))
}

/// Creates an http server with a path corresponsing to a hash of the task's source code.
/// It was done like this in preparation for concurrent tasks, as a unique task ID was already needed for that.
/// 
/// Data is received by the http server via a JSON POST, and is assumed by Taskmaster to be report data.
/// 
/// Note that the http server listens forever, this coroutine should be joined with the process spawn.
/// 
/// TODO: Make this multithreaded and have the server kill itself once it has received a POST.
pub async fn listen_for_report(task_id: &str, tx: Sender<Value>) -> Result<(), Box<dyn std::error::Error>> {
    let tx_shared = Arc::new(Mutex::new(tx)); //hack so we can copy this to every instantiation of the closure
    let route = warp::post()
        .and(warp::path(task_id.to_owned()))
        .and(warp::body::json())
        .and_then(move |value: Value| {
            let tx = tx_shared.clone();
            async move { //Hack so move keyword doesn't consume the entire galaxy when we try to send
                match tx.lock().await.send(value).await {
                    Ok(_) => Ok::<_, warp::Rejection>(warp::reply::with_status("Report received.", StatusCode::OK)),
                    Err(_) => Ok::<_, warp::Rejection>(warp::reply::with_status("Error occured while processing report.", StatusCode::INTERNAL_SERVER_ERROR))
                }
            }
        }).boxed();

    let _ = warp::serve(route).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}

/// Retreives a relatively standardized task input from the user input serde structure.
fn unpack_tasks(user_input: &Value) -> Option<Value> {
    let task_keywords: Vec<&str> = vec!["tasks", "actions"];
    for keyword in task_keywords {
        match user_input.get(sanitize_string(keyword)) {
            Some(a) => {
                return Some(a.to_owned())
            },
            None => continue
        }
    }

    None
}

/// Returns a task input (serde Value) as a vector of Values.
fn tasks_from_seq(input: &Value) -> Option<Vec<Value>> {
    input.as_sequence().map(|a| a.to_owned())
}

/// Returns a task input (serde Value) that is a Mapping and converts to a vec of Values.
pub fn tasks_from_map(input: &Value) -> Option<Vec<Value>> {
    let user_map = match input.as_mapping() {
        Some(a) => a,
        None => return None
    };

    let mut tasks: Vec<Value> = Vec::new();
    
    for (key, val) in user_map {
        let mut task = Mapping::new();
        task.insert(key.to_owned(), val.to_owned());
        // {} Not sure why this was here, 99.9% sure it was not needed
        tasks.push(Value::from(task));
    }

    Some(tasks)
}

/// Unpack user task data and return a vector of the inner values.
fn get_task_sequence(user_input: &Value) -> Option<Vec<Value>> {
    let inner_input = match unpack_tasks(user_input) { // Get inner values of task.
        Some(a) => a,
        None => user_input.clone()
    };

    if user_input.is_mapping() { // Hashtable handling
        return tasks_from_map(&inner_input);
    }

    if user_input.is_sequence() { // Array handling
        return tasks_from_seq(&inner_input);
    }

    None // No other data structure is supported.
}

pub async fn execute_tasks(user_input: &Value, task_input: &Value) -> Result<()> {
    let clean_user_input = sanitize_yaml(user_input).ok_or(anyhow!("Could not sanitize user input. This typically means malformed user input."))?;
    let clean_task_input = sanitize_yaml(task_input).ok_or(anyhow!("Could not sanitize task input. This typically means malformed task input."))?;

    // Get the tasks to run as defined by the user
    let tasks = match get_task_sequence(&clean_user_input) {
        Some(a) => a,
        None => {
            let b = match get_by_path(&clean_user_input, "tasks", None) {
                Some((c, _)) => {
                    match get_task_sequence(&c) {
                        Some(d) => d,
                        None => return Err(anyhow!("Could not get task sequence from user input."))
                    }
                },
                None => return Err(anyhow!("Could not get task sequence from user input."))
            };
            b
        }
    };

    for user_task in tasks {
        if !user_task.is_mapping() { // Tasks should only ever be hashtables
            error!("Task with the following data is not a Mapping/Hashtable and was skipped: {:#?}", user_task);
            continue
        }

        // Tasks should only have 1 entry point
        let user_map = user_task.as_mapping().ok_or(anyhow!(format!("Task with the following data is not valid: {:#?}", user_task)))?;
        if user_map.keys().len() != 1 {
            error!("Task with the following data had more than 1 entrypoint and was skipped: {:#?}", user_task);
            continue
        }

        // Get entrypoint of task as string
        let key_str = match user_map.keys().next() {
            Some(a) => {
                match a.as_str() {
                    Some(b) => b,
                    None => {
                        error!("Could not convert {:#?} to string, skipping task.", a);
                        continue
                    }
                }
            },
            None => continue
        };

        // Get real name of task based on user input task name.
        let canonical_key = match get_by_path(&clean_task_input, &format!("{}/", key_str), None) {
            Some((_, b)) => match b.as_str() {
                Some(c) => {
                    c.to_owned()
                },
                None => {
                    error!("Found task name/alias \"{}\" but could not convert it to string.", key_str);
                    continue
                } 
            },
            None => {
                error!("Could not find task with name/alias \"{}\", skipping task.", key_str);
                continue
            }
        };

        // Get the first level of task data from the user input task.
        // This should never fail as we got key_str from the user input task.
        let user_data = match user_task.get(Value::from(key_str)) {
            Some(a) => a,
            None => {
                error!("Could not derive task data from task \"{key_str}\". This should not be possible, skipping task.");
                continue
            }
        };

        // Creates a temporary script to run based on the current task specified by the user..
        let ps_file_pathbuf: PathBuf = get_task_script(&canonical_key, None)?;
        let ps_file_path = match ps_file_pathbuf.to_str() {
            Some(a) => a,
            None => {
                error!("Could not convert task code of \"{}\" to string. Skipping task.", key_str);
                continue
            }
        };
        
        // Check if the task actually has a reference {TASK NAME}.ps1 file\
        // This should be in the ".\tasks" directory
        match Path::new(ps_file_path).exists() {
            true => {},
            false => {
                error!(
                    "PowerShell code for task \"{}\" could not be found at \"{}\". Please create this file and add code you would like to run for the task, and try again. Skipping task.",
                    key_str, 
                    ps_file_path
                );
                continue
            }
        }

        // Read the code for the user's task to str.
        let ps_code = read_to_string(ps_file_path).await?;

        // Get inner task data for the referenced task as defined by tasks.yaml
        let (task_data, _) = match get_by_path(&clean_task_input, format!("{}/", canonical_key).as_ref(), None) {
            Some(a) => a,
            None => {
                error!("Could not find task \"{canonical_key}\" in tasks.yaml. Please create a task with that name and try again.");
                continue
            }
        };

        // Get edited source code + hash of source code for the task ps1 to run.
        // This ensures code can't be modified JIT by some nefarious process or silly evaluation on a task.
        // This was also done in preparation for parallel tasks which is no longer planned.
        let (t_source, t_hash) = match prep_passthru_args(user_data, &task_data) {
            Some((a, b)) => (a, b.to_ascii_uppercase()),
            None => return Err(anyhow!("Error occured when initializing task data."))
        };

        let task_code = format!("{}{}", &t_source, ps_code);

        // Set up a channel for async communication, this is purely within Taskmaster.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Value>(10);

        // Set up a report listener for powershell-side (HTTP on localhost), and the powershell process spawner.
        tokio::select! {
            output = spawn_powershell(&task_code, false) => {
                info!("Output: {}", output?)
            }
    
            _ = listen_for_report(&t_hash, tx) => {
                error!("Report server failed.")
            }
        }

        // Return report back so it can be logged.
        let report = match rx.recv().await {
            Some(a) => a,
            None => return Err(anyhow!("Error occured when retreiving report thread data."))
        };

        // Log report.
        info!("Report for task \"{canonical_key}\" of hash \"{t_hash}\": {:?}", report);
    }

    Ok(())
}

/// Retreives the task definitions file and the contents of said file.
/// 
/// # Panic
/// This function will panic if it cannot get the task definitions file.
/// 
/// This is intended as the entire tool relies on this functionality, and therefore there is no recovery if this happens.
pub async fn task_defs_contents() -> Result<String> {
    let task_input_path = match get_task_script("tasks", Some("yaml")) {
        Ok(a) => a,
        Err(e) => {
            panic!("Error occured when retreiving \"tasks\\tasks.yaml\": {}", anyhow!(e))
        }
    };
    file_contents(&task_input_path).await
}

/// Extracts the name of the nth key of a serde Mapping as a String.
fn extract_map_nth_key(value: Value, index: usize) -> Option<String> {
    let keys: Vec<&Value> = value.as_mapping()?.keys().into_iter().collect();
    let nth_key = keys.get(index)?;
    return Some(nth_key.as_str()?.to_owned());
}

/// Returns a vector of defined task names.
pub async fn get_task_str_sequence() -> Result<Vec<String>> {
    let tasks_path = handle_logged_result(
        get_task_script("tasks", Some("yaml")),
        "Error occured when retreiving location of tasks folder: "
    )?;

    let tasks_file = handle_logged_result(
        file_contents(&tasks_path).await, 
        &format!("Error occured when reading file \"{}\": ", &tasks_path.display())
    )?;

    let tasks_raw = handle_logged_result(
        serde_yml::from_str(&tasks_file).map_err(|e| anyhow!(e.to_string())), 
        "Error occured when parsing tasks file: "
    )?;

    return match tasks_from_map(&tasks_raw) {
        Some(tasks_vec) => {
            let mut tasks: Vec<String> = Vec::new();
            for task in tasks_vec {
                if let Some(a) = extract_map_nth_key(task, 0) { tasks.push(a) };
            }
            Ok(tasks)
        },
        None => {
            let e = anyhow!("Could not retreive task definition list from task input Mapping.");
            error!("{}", e);
            return Err(e)
        }
    };
}