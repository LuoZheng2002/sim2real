use std::{fs::File, io::{BufRead, BufReader}, path::Path};

pub fn load_json_lines(file_path: impl AsRef<Path>) -> Result<Vec<serde_json::Value>, String> {
    let file = File::open(&file_path).map_err(|e| {
        format!(
            "Unable to open file {}: {}",
            file_path.as_ref().display(),
            e
        )
    })?;
    let reader = BufReader::new(file);

    let mut results = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Unable to read line: {}", e))?;
        let line_json: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| format!("Unable to parse JSON: {}", e))?;
        results.push(line_json);
    }
    Ok(results)
}

pub fn write_json_lines_to_file(
    file_path: impl AsRef<Path>,
    results: &Vec<serde_json::Value>,
) -> Result<(), String> {
    use std::fs::{File, create_dir_all};
    use std::io::Write;
    if let Some(parent) = file_path.as_ref().parent() {
        create_dir_all(parent).map_err(|e| format!("Unable to create parent directory: {}", e))?;
    }

    let mut file = File::create(file_path).map_err(|e| format!("Unable to create file: {}", e))?;
    for result in results {
        let line = serde_json::to_string(result)
            .map_err(|e| format!("Unable to serialize JSON: {}", e))?;
        writeln!(file, "{}", line).map_err(|e| format!("Unable to write to file: {}", e))?;
    }
    file.flush()
        .map_err(|e| format!("Unable to flush file: {}", e))?;

    Ok(())
}