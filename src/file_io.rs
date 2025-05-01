use std::fs::File;
use std::io::Read;


pub fn read_file(filepath: &str) -> std::io::Result<String> {
    let mut file = File::open(filepath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn write_file(filepath: &str, contents: &str) -> std::io::Result<()> {
    std::fs::write(filepath, contents)
        .map_err(|e| std::io::Error::new(e.kind(), format!("Failed to write to file: {}", e)))
}