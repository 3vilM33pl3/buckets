use std::io;
use std::io::Write;

pub fn execute<W: Write>(writer: &mut W) -> io::Result<()> {
    writeln!(writer, "bucket version 0.1.0")
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
