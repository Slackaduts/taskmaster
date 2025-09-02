use anyhow::{Result, anyhow};
use serde_yml::{Value, Sequence, Mapping};

use std::path::PathBuf;
use std::iter::zip;

use tokio::io::{BufWriter, AsyncWriteExt, AsyncReadExt};
use tokio::fs::{create_dir, remove_file, File};

/// Strips numbers, symbols, and converts to lowercase on a string slice.
pub fn sanitize_string(string: &str) -> String {
    let mut output = String::new();

    for ch in string.to_ascii_lowercase().chars()  {
        if !ch.is_ascii_alphabetic() { continue; }

        output.push(ch);
    }

    output
}

/// Converts value to str, runs sanitize_str(), and then converts back to Value.
pub fn sanitize_value(value: &Value) -> Option<Value> {
    Some(Value::String(sanitize_string(value.as_str()?)))
}

/// Returns a tuple of the sanitized Mapping keys and original keys, 1:1 indexing. 
pub fn sanitize_map_keys(data: &Mapping) -> Option<(Vec<String>, Vec<String>)> {
    let mut output = Vec::new();
    let mut originals = Vec::new();

    for val in data.keys() {
        let val_str = val.as_str()?;
        output.push(sanitize_string(val_str));
        originals.push(val_str.to_owned());
    }

    Some((output, originals))
}

/// Strips symbols, numbers, and makes all keys of all levels of a Value lowercase.
pub fn sanitize_yaml(input: &Value) -> Option<Value> {
    if input.is_mapping() {
        let mut clean_map = Mapping::new();
        let input_map = input.as_mapping()?;

        let (norm_keys, orig_keys) = sanitize_map_keys(input_map)?;

        for (n_k, o_k) in zip(norm_keys, orig_keys) {
            match sanitize_yaml(&input_map[Value::String(o_k.clone())]) {
                Some(a) => clean_map.insert(Value::String(n_k), a),
                None => continue
            };
        }

        return Some(Value::from(clean_map));
    }

    if input.is_sequence() {
        let mut clean_seq = Sequence::new();
        let input_seq = input.as_sequence()?;

        for elem in input_seq {
            if let Some(a) = sanitize_yaml(elem) { clean_seq.push(a) }
        }

        return Some(Value::from(clean_seq));
    }

    Some(input.clone())
}

/// Creates a file with data at a given path.
pub async fn create_file(data: &str, path: PathBuf) -> Result<()> {
    let file = File::create(path).await?;
    let mut writer = BufWriter::new(file);

    writer.write_all(data.as_bytes()).await?;
    writer.flush().await?;

    drop(writer);

    Ok(())
}

/// Deletes a file at a given path.
pub async fn delete_file(path: PathBuf) -> Result<()> {
    remove_file(path).await?;
    Ok(())
}

/// Retreives the contents of a file by its path.
pub async fn file_contents(path: &PathBuf) -> Result<String> {
    let dir_path = match path.parent() { Some(a) => PathBuf::from(a), None => PathBuf::new() };
    if let Err(e) = ensure_dir(dir_path).await {
        return Err(anyhow!("Error occured when creating directory for filepath \"{}\": {}", path.display(), anyhow!(e))) 
    };

    let mut f: File = File::open(path).await?;

    let mut f_content = String::new();
    f.read_to_string(&mut f_content).await?;

    Ok(f_content)
}

/// Creates a given directory if it does not already exist.
pub async fn ensure_dir(path: PathBuf) -> Result<()> {
    if path.exists() { return Ok(()) }
    create_dir(path).await?;

    Ok(())
}

/// Handles error logging with anyhow!() and an error content message.
/// 
/// # Purpose
/// Propogating error values throws off the line reference of log!, so this allows for additional content
/// to be prepended to the error value.
pub fn handle_logged_result<T>(result: Result<T>, error_msg: &str) -> Result<T> {
    if let Err(e) = result {
        let err = anyhow!(format!("{}{}", error_msg, e));
        return Err(err)
    }

    result
}