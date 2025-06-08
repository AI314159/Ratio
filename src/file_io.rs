use std::fs::File;
use std::io::Read;

pub fn read_file(filepath: &std::path::PathBuf) -> std::io::Result<String> {
    std::fs::read_to_string(filepath)
}

pub fn write_file(filepath: &str, contents: &str) -> std::io::Result<()> {
    std::fs::write(filepath, contents)
        .map_err(|e| std::io::Error::new(e.kind(), format!("Failed to write to file: {}", e)))
}
