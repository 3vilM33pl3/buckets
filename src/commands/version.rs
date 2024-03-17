use std::io;
use std::io::Write;
use log::info;
use sysinfo::System;

pub fn execute<W: Write>(writer: &mut W) -> io::Result<()> {
    info!("System Information:\n{}\n", gather_system_info());
    writeln!(writer, "bucket version 0.1.0")
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_version() -> io::Result<()> {
        let mut buffer = Vec::new();
        execute(&mut buffer)?;

        let output = String::from_utf8(buffer).expect("Not UTF-8");
        assert_eq!(output, "bucket version 0.1.0\n");
        Ok(())
    }
}
