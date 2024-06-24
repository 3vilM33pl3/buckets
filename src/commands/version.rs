use std::{env, fs, io};
use std::io::Write;
use std::path::{Path, PathBuf};
use log::info;
use sysinfo::{System};
use crate::cli;
use crate::utils::checks::find_bucket_repo;

pub fn execute<W: Write>(writer: &mut W) -> io::Result<()> {
    info!("Bucket version {}\n", cli().get_version().unwrap_or_default());

    // Get the current directory or a default path
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

    // Find the bucket repository path or return None if not found
    let bucket_repo_path = find_bucket_repo(current_dir.as_path());

    match bucket_repo_path {
        Some(path) => match dir_size(path.as_path()) {
            Ok(size) => {
                writeln!(writer, "Total size of bucket repository: {} bytes", size)?;
                info!("Total size of bucket repository: {} bytes\n", size);
                Ok(())
            },
            Err(e) => {
                writeln!(writer, "Error calculating directory size: {}", e)?;
                Err(e)
            },
        },
        None => {
            writeln!(writer, "Bucket repository not found.")?;
            Ok(())
        },
    }?;

    info!("System Information:\n{}\n", gather_system_info());
    writeln!(writer, "System Information:\n{}\n", gather_system_info())?;
    Ok(())
}

/// Gathers and returns system information as a formatted string.
fn gather_system_info() -> String {
    let mut system = System::new_all();
    system.refresh_all(); // Refresh all system information

    let system_name = System::name().unwrap_or_default();
    let kernel_version = System::kernel_version().unwrap_or_default();
    let os_version = System::os_version().unwrap_or_default();
    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let total_swap = system.total_swap();
    let used_swap = system.used_swap();
    let cpu_usage: Vec<String> = system.cpus().iter()
        .map(|cpu| format!("{:.2}%", cpu.cpu_usage()))
        .collect();

    format!(
        "System Name: {}\nKernel Version: {}\nOS Version: {}\nTotal Memory: {} KB\nUsed Memory: {} KB\nTotal Swap: {} KB\nUsed Swap: {} KB\nCPU Usage: {}",
        system_name,
        kernel_version,
        os_version,
        total_memory,
        used_memory,
        total_swap,
        used_swap,
        cpu_usage.join(", ")
    )
}

/// Recursively calculates the total size of all files in the given directory.
///
/// # Arguments
///
/// * `dir` - A reference to a `Path` representing the directory to calculate the size of.
///
/// # Returns
///
/// * `io::Result<u64>` - The total size of all files in the directory, in bytes.
fn dir_size(dir: &Path) -> io::Result<u64> {
    let mut size = 0;

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                size += dir_size(&path)?;
            } else {
                size += fs::metadata(path)?.len();
            }
        }
    }

    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() -> io::Result<()> {
        let mut buffer = Vec::new();
        execute(&mut buffer)?;

        let output = String::from_utf8(buffer).expect("Not UTF-8");
        assert!(output.contains("System Information:"));
        Ok(())
    }
}
