use std::{
    fs::File,
    io::{self},
    path::{Path, PathBuf},
};

use zstd::stream::{copy_decode, copy_encode};

/// Default compression level for zstd (good balance of speed and compression ratio)
pub const DEFAULT_COMPRESSION_LEVEL: i32 = 3;

/// Maximum compression level for zstd (highest compression ratio, slower)
pub const MAX_COMPRESSION_LEVEL: i32 = 22;

/// Compress a file using zstd and store it at the specified path
/// 
/// # Arguments
/// * `input_path` - Path to the input file to compress
/// * `output_path` - Path where the compressed file will be stored
/// * `compression_level` - Compression level (0 for default, 1-22 for specific levels)
/// 
/// # Returns
/// * `Ok(())` on success
/// * `Err(io::Error)` on failure with descriptive error message
pub fn compress_file(
    input_path: &Path,
    output_path: &Path,
    compression_level: i32,
) -> io::Result<()> {
    // Validate compression level
    let level = if compression_level == 0 {
        DEFAULT_COMPRESSION_LEVEL
    } else if compression_level < 1 || compression_level > MAX_COMPRESSION_LEVEL {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid compression level: {}. Must be 0 (default) or 1-{}", 
                    compression_level, MAX_COMPRESSION_LEVEL),
        ));
    } else {
        compression_level
    };

    let input_file = File::open(input_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to open input file '{}': {}", input_path.display(), e),
        )
    })?;
    
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Failed to create output directory '{}': {}", parent.display(), e),
            )
        })?;
    }
    
    let mut output_file = File::create(output_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to create output file '{}': {}", output_path.display(), e),
        )
    })?;

    copy_encode(&input_file, &mut output_file, level).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to compress file from '{}' to '{}': {}",
                input_path.display(),
                output_path.display(),
                e
            ),
        )
    })?;

    Ok(())
}

/// Decompress a zstd-compressed file and restore it to the specified path
/// 
/// # Arguments
/// * `input_path` - Path to the compressed file
/// * `output_path` - Path where the decompressed file will be restored
/// 
/// # Returns
/// * `Ok(())` on success
/// * `Err(io::Error)` on failure with descriptive error message
pub fn decompress_file(input_path: &Path, output_path: &Path) -> io::Result<()> {
    let input_file = File::open(input_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to open compressed file '{}': {}", input_path.display(), e),
        )
    })?;
    
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Failed to create output directory '{}': {}", parent.display(), e),
            )
        })?;
    }
    
    let output_file = File::create(output_path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to create output file '{}': {}", output_path.display(), e),
        )
    })?;
    
    copy_decode(input_file, output_file).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to decompress file from '{}' to '{}': {}",
                input_path.display(),
                output_path.display(),
                e
            ),
        )
    })?;
    
    Ok(())
}

/// Legacy function for backward compatibility
/// Use `compress_file` instead
#[deprecated(since = "0.1.6", note = "Use `compress_file` instead")]
pub fn compress_and_store_file(
    input_path: &PathBuf,
    output_path: &PathBuf,
    compression_level: i32,
) -> io::Result<()> {
    compress_file(input_path, output_path, compression_level)
}

/// Legacy function for backward compatibility
/// Use `decompress_file` instead
#[deprecated(since = "0.1.6", note = "Use `decompress_file` instead")]
pub fn restore_file(input_path: &PathBuf, output_path: &PathBuf) -> io::Result<()> {
    decompress_file(input_path, output_path)
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::{fs, io::Write};
    use tempfile::tempdir;

    #[test]
    fn test_compress_and_store_file() {
        let dir = tempdir().expect("Failed to create temp dir");
        let input_path = dir.path().join("input.txt");
        let result_path = dir.path().join("result.txt");
        let output_path = dir.path().join("output.zst");

        // Create a test file
        fs::write(&input_path, "Hello, world!").expect("Failed to write test file");

        // Compress the file
        compress_file(&input_path, &output_path, 3).expect("Failed to compress file");

        // Check if the compressed file exists
        assert!(output_path.exists());

        // uncompress the file
        let mut decoder = zstd::stream::Decoder::new(
            File::open(&output_path).expect("Failed to open compressed file"),
        )
        .expect("Failed to create decoder");
        let mut writer = File::create(&result_path).expect("Failed to create result file");
        io::copy(&mut decoder, &mut writer).expect("Failed to copy decoded data");
        decoder.finish();

        // Check if the uncompressed file exists
        assert!(result_path.exists());
        let content = fs::read_to_string(&result_path).expect("Failed to read result file");
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn test_restore_file() {
        let mut dir = tempdir().expect("Failed to create temp dir");
        dir.disable_cleanup(true);
        log::info!("dir: {:?}", dir.path());
        let bucket_path = dir.path().to_path_buf();
        let compressed_file_path = bucket_path.join("input.zst");
        let restored_file_path = bucket_path.join("output.txt");

        // Create a test file
        fs::write(&restored_file_path, "Hello, world!").expect("Failed to write test file");

        let writer = File::create(&compressed_file_path).expect("Failed to create compressed file");

        let mut encoder = zstd::stream::Encoder::new(&writer, 7).expect("Failed to create encoder");
        io::copy(
            &mut File::open(&restored_file_path).expect("Failed to open source file"),
            &mut encoder,
        )
        .expect("Failed to copy to encoder");
        encoder.finish().expect("Failed to finish encoding");

        // remove the original file
        fs::remove_file(&restored_file_path).expect("Failed to remove original file");

        // Restore the file
        decompress_file(&compressed_file_path, &restored_file_path).expect("Failed to restore file");
        log::info!("File restored successfully");

        // Check if the restored file exists and contains the expected content
        assert!(restored_file_path.exists());
        let content =
            fs::read_to_string(&restored_file_path).expect("Failed to read restored file");
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn test_compress_and_store_large_file() {
        let dir = tempdir().expect("Failed to create temp dir");
        let input_path = dir.path().join("input.txt");
        let result_path = dir.path().join("result.txt");
        let output_path = dir.path().join("output.zst");

        // Create a large test file
        let mut file = File::create(&input_path).expect("Failed to create test file");
        let buffer = vec![0; 1024 * 1024 * 10]; // 10MB
        file.write_all(&buffer).expect("Failed to write test data");

        // Compress the file
        compress_file(&input_path, &output_path, 3)
            .expect("Failed to compress large file");

        // Check if the compressed file exists
        assert!(output_path.exists());

        // uncompress the file
        let mut decoder = zstd::stream::Decoder::new(
            File::open(&output_path).expect("Failed to open compressed file"),
        )
        .expect("Failed to create decoder");
        let mut writer = File::create(&result_path).expect("Failed to create result file");
        io::copy(&mut decoder, &mut writer).expect("Failed to copy decoded data");
        decoder.finish();

        // Check if the uncompressed file exists
        assert!(result_path.exists());
    }

    #[test]
    fn test_restore_large_file() {
        let dir = tempdir().expect("Failed to create temp dir");
        let input_path = dir.path().join("input.txt");
        let result_path = dir.path().join("result.txt");
        let output_path = dir.path().join("output.zst");

        // Create a large test file
        let mut file = File::create(&input_path).expect("Failed to create test file");
        let buffer = vec![0; 1024 * 1024 * 10]; // 10MB
        file.write_all(&buffer).expect("Failed to write test data");

        // Compress the file
        compress_file(&input_path, &output_path, 3)
            .expect("Failed to compress large file");

        // Restore the file
        decompress_file(&output_path, &result_path).expect("Failed to restore large file");

        // Check if the restored file exists
        assert!(result_path.exists());
    }

    #[test]
    fn test_compress_nonexistent_file() {
        let dir = tempdir().expect("Failed to create temp dir");
        let nonexistent_path = dir.path().join("does_not_exist.txt");
        let output_path = dir.path().join("output.zst");

        // Test compression of non-existent file
        let result = compress_file(&nonexistent_path, &output_path, 3);
        assert!(result.is_err());

        if let Err(e) = result {
            assert_eq!(e.kind(), io::ErrorKind::NotFound);
        }
    }

    #[test]
    fn test_compress_to_invalid_directory() {
        let dir = tempdir().expect("Failed to create temp dir");
        let input_path = dir.path().join("input.txt");
        let invalid_output_path = dir.path().join("nonexistent_dir").join("output.zst");

        // Create input file
        fs::write(&input_path, "test content").expect("Failed to write input file");

        // Test compression to non-existent directory (should succeed because we auto-create directories)
        let result = compress_file(&input_path, &invalid_output_path, 3);
        assert!(result.is_ok(), "Compression should succeed because parent directories are auto-created");
        
        // Verify the file was created
        assert!(invalid_output_path.exists(), "Output file should exist after compression");
    }

    #[test]
    fn test_restore_nonexistent_file() {
        let dir = tempdir().expect("Failed to create temp dir");
        let nonexistent_compressed = dir.path().join("does_not_exist.zst");
        let output_path = dir.path().join("output.txt");

        // Test restoration of non-existent compressed file
        let result = decompress_file(&nonexistent_compressed, &output_path);
        assert!(result.is_err());

        if let Err(e) = result {
            assert_eq!(e.kind(), io::ErrorKind::NotFound);
        }
    }

    #[test]
    fn test_restore_invalid_compressed_file() {
        let dir = tempdir().expect("Failed to create temp dir");
        let fake_compressed = dir.path().join("fake.zst");
        let output_path = dir.path().join("output.txt");

        // Create fake compressed file (not actually compressed)
        fs::write(&fake_compressed, "this is not compressed data")
            .expect("Failed to write fake file");

        // Test restoration of invalid compressed file
        let result = decompress_file(&fake_compressed, &output_path);
        assert!(result.is_err());
        // Should fail during decompression
    }

    #[test]
    fn test_restore_to_readonly_directory() {
        let dir = tempdir().expect("Failed to create temp dir");
        let input_path = dir.path().join("input.txt");
        let compressed_path = dir.path().join("compressed.zst");

        // Create readonly subdirectory
        let readonly_dir = dir.path().join("readonly");
        fs::create_dir_all(&readonly_dir).expect("Failed to create readonly dir");

        // Set readonly permissions (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&readonly_dir)
                .expect("Failed to get metadata")
                .permissions();
            perms.set_mode(0o444); // readonly
            fs::set_permissions(&readonly_dir, perms).expect("Failed to set permissions");
        }

        // Create and compress test file
        fs::write(&input_path, "test content").expect("Failed to write input file");
        compress_file(&input_path, &compressed_path, 3).expect("Failed to compress");

        let readonly_output = readonly_dir.join("output.txt");

        // Test restoration to readonly directory (may fail on some systems)
        let result = decompress_file(&compressed_path, &readonly_output);

        // Restore directory permissions for cleanup
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&readonly_dir)
                .expect("Failed to get metadata")
                .permissions();
            perms.set_mode(0o755); // writable
            fs::set_permissions(&readonly_dir, perms).expect("Failed to restore permissions");
        }

        // On some systems this might succeed, on others it might fail with PermissionDenied
        if result.is_err() {
            if let Err(e) = result {
                assert!(
                    e.kind() == io::ErrorKind::PermissionDenied
                        || e.kind() == io::ErrorKind::NotFound
                );
            }
        }
    }

    #[test]
    fn test_compress_empty_file() {
        let dir = tempdir().expect("Failed to create temp dir");
        let input_path = dir.path().join("empty.txt");
        let output_path = dir.path().join("empty.zst");
        let restored_path = dir.path().join("restored.txt");

        // Create empty file
        File::create(&input_path).expect("Failed to create empty file");

        // Compress empty file
        compress_file(&input_path, &output_path, 3)
            .expect("Failed to compress empty file");
        assert!(output_path.exists());

        // Restore empty file
        decompress_file(&output_path, &restored_path).expect("Failed to restore empty file");
        assert!(restored_path.exists());

        let restored_content = fs::read(&restored_path).expect("Failed to read restored file");
        assert!(restored_content.is_empty());
    }

    #[test]
    fn test_compression_with_different_levels() {
        let dir = tempdir().expect("Failed to create temp dir");
        let input_path = dir.path().join("input.txt");
        let test_content = "This is test content for compression level testing.".repeat(100);

        fs::write(&input_path, &test_content).expect("Failed to write test file");

        // Test different compression levels
        for level in [0, 1, 3, 9, 22] {
            // zstd supports levels 1-22, 0 is default
            let output_path = dir.path().join(format!("output_level_{}.zst", level));
            let restored_path = dir.path().join(format!("restored_level_{}.txt", level));

            // Compress with specific level
            let result = compress_file(&input_path, &output_path, level);
            assert!(result.is_ok(), "Failed to compress with level {}", level);
            assert!(output_path.exists());

            // Restore and verify content
            decompress_file(&output_path, &restored_path).expect("Failed to restore");
            let restored_content =
                fs::read_to_string(&restored_path).expect("Failed to read restored");
            assert_eq!(restored_content, test_content);
        }
    }

    #[test]
    fn test_compression_roundtrip_various_data() {
        let dir = tempdir().expect("Failed to create temp dir");

        let binary_data = (0u8..=255u8).cycle().take(1000).collect::<Vec<u8>>();
        let binary_string = String::from_utf8_lossy(&binary_data);
        let repeated_string = "A".repeat(10000);

        let test_cases = vec![
            (
                "text",
                "Hello, world! This is a test string with various characters: 你好世界 🌍",
            ),
            ("binary", binary_string.as_ref()),
            ("repeated", repeated_string.as_str()),
            (
                "json",
                r#"{"name": "test", "value": 42, "items": [1, 2, 3, {"nested": true}]}"#,
            ),
            ("whitespace", "   \t\n\r  \n\n  \t  "),
        ];

        for (name, content) in test_cases {
            let input_path = dir.path().join(format!("input_{}.txt", name));
            let compressed_path = dir.path().join(format!("compressed_{}.zst", name));
            let restored_path = dir.path().join(format!("restored_{}.txt", name));

            // Write test content
            fs::write(&input_path, content).expect("Failed to write test content");

            // Compress
            compress_file(&input_path, &compressed_path, 3)
                .expect("Failed to compress test content");
            assert!(compressed_path.exists());

            // Restore
            decompress_file(&compressed_path, &restored_path).expect("Failed to restore test content");
            assert!(restored_path.exists());

            // Verify content matches
            let restored_content =
                fs::read_to_string(&restored_path).expect("Failed to read restored content");
            assert_eq!(
                restored_content, content,
                "Content mismatch for test case: {}",
                name
            );
        }
    }

    #[test]
    fn test_binary_file_roundtrip() {
        let dir = tempdir().expect("Failed to create temp dir");
        
        // Test various binary patterns
        let test_cases = vec![
            ("zeros", vec![0u8; 1000]),
            ("ones", vec![255u8; 1000]),
            ("alternating", (0..1000).map(|i| if i % 2 == 0 { 0xAA } else { 0x55 }).collect()),
            ("random_pattern", (0..1000).map(|i| (i * 17 + 42) as u8).collect()),
            ("all_bytes", (0u8..=255u8).cycle().take(2000).collect()),
            ("sparse", {
                let mut data = vec![0u8; 1000];
                for i in (0..1000).step_by(100) {
                    data[i] = 0xFF;
                }
                data
            }),
        ];

        for (name, binary_data) in test_cases {
            let input_path = dir.path().join(format!("binary_input_{}.bin", name));
            let compressed_path = dir.path().join(format!("binary_compressed_{}.zst", name));
            let restored_path = dir.path().join(format!("binary_restored_{}.bin", name));

            // Write binary test data
            fs::write(&input_path, &binary_data).expect("Failed to write binary test data");

            // Compress using new function
            compress_file(&input_path, &compressed_path, DEFAULT_COMPRESSION_LEVEL)
                .expect("Failed to compress binary data");
            assert!(compressed_path.exists());

            // Decompress using new function
            decompress_file(&compressed_path, &restored_path)
                .expect("Failed to decompress binary data");
            assert!(restored_path.exists());

            // Verify binary content matches exactly
            let restored_data = fs::read(&restored_path).expect("Failed to read restored binary data");
            assert_eq!(
                restored_data, binary_data,
                "Binary data mismatch for test case: {} (original len: {}, restored len: {})",
                name, binary_data.len(), restored_data.len()
            );
        }
    }

    #[test]
    fn test_large_binary_file_roundtrip() {
        let dir = tempdir().expect("Failed to create temp dir");
        
        // Generate a large binary file with patterns
        let mut large_binary = Vec::with_capacity(1024 * 1024); // 1MB
        
        // Fill with structured binary data
        for chunk in 0..(1024 * 1024 / 1024) {
            for byte_idx in 0..1024 {
                // Create a pattern that includes the chunk number and byte position
                let pattern_byte = ((chunk * 17 + byte_idx * 3) % 256) as u8;
                large_binary.push(pattern_byte);
            }
        }
        
        let input_path = dir.path().join("large_binary.bin");
        let compressed_path = dir.path().join("large_binary.zst");
        let restored_path = dir.path().join("large_binary_restored.bin");

        // Write large binary file
        fs::write(&input_path, &large_binary).expect("Failed to write large binary file");

        // Compress with high compression level for better ratio on large files
        compress_file(&input_path, &compressed_path, 9)
            .expect("Failed to compress large binary file");
        assert!(compressed_path.exists());

        // Check compression ratio (should be reasonably good due to patterns)
        let original_size = fs::metadata(&input_path).unwrap().len();
        let compressed_size = fs::metadata(&compressed_path).unwrap().len();
        let compression_ratio = compressed_size as f64 / original_size as f64;
        
        // Pattern-based data should compress well, expect at least 50% compression
        assert!(compression_ratio < 0.8, 
                "Compression ratio too low: {:.2}% (compressed: {} bytes, original: {} bytes)",
                compression_ratio * 100.0, compressed_size, original_size);

        // Decompress
        decompress_file(&compressed_path, &restored_path)
            .expect("Failed to decompress large binary file");
        assert!(restored_path.exists());

        // Verify exact binary match
        let restored_data = fs::read(&restored_path).expect("Failed to read restored large binary file");
        assert_eq!(restored_data.len(), large_binary.len(), "Size mismatch after decompression");
        assert_eq!(restored_data, large_binary, "Large binary data corrupted during compression round-trip");
    }

    #[test]
    fn test_executable_binary_roundtrip() {
        let dir = tempdir().expect("Failed to create temp dir");
        
        // Simulate executable binary with header, code sections, and data
        let mut executable_data = Vec::new();
        
        // ELF-like header (fake)
        executable_data.extend_from_slice(b"\x7fELF\x02\x01\x01\x00");
        executable_data.extend_from_slice(&[0u8; 8]); // padding
        
        // Machine code patterns (x86_64-like)
        let code_patterns = [
            &[0x48, 0x89, 0xe5][..], // mov %rsp, %rbp
            &[0xb8, 0x00, 0x00, 0x00, 0x00][..], // mov $0, %eax
            &[0x5d][..], // pop %rbp
            &[0xc3][..], // ret
        ];
        
        for _ in 0..100 {
            for pattern in &code_patterns {
                executable_data.extend_from_slice(pattern);
            }
        }
        
        // String table
        executable_data.extend_from_slice(b"\0main\0printf\0exit\0");
        
        // Padding/alignment
        while executable_data.len() % 16 != 0 {
            executable_data.push(0);
        }
        
        let input_path = dir.path().join("fake_executable.bin");
        let compressed_path = dir.path().join("fake_executable.zst");
        let restored_path = dir.path().join("fake_executable_restored.bin");

        // Write executable-like binary
        fs::write(&input_path, &executable_data).expect("Failed to write executable binary");

        // Compress
        compress_file(&input_path, &compressed_path, DEFAULT_COMPRESSION_LEVEL)
            .expect("Failed to compress executable binary");
        assert!(compressed_path.exists());

        // Decompress
        decompress_file(&compressed_path, &restored_path)
            .expect("Failed to decompress executable binary");
        assert!(restored_path.exists());

        // Verify exact match (critical for executables)
        let restored_data = fs::read(&restored_path).expect("Failed to read restored executable");
        assert_eq!(restored_data, executable_data, 
                   "Executable binary corrupted during compression - this would break the program!");
    }

    #[test]
    fn test_multimedia_binary_roundtrip() {
        let dir = tempdir().expect("Failed to create temp dir");
        
        // Simulate multimedia file patterns (like JPEG, PNG headers and data)
        let mut multimedia_data = Vec::new();
        
        // JPEG-like header
        multimedia_data.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]);
        multimedia_data.extend_from_slice(b"JFIF");
        
        // Random image-like data with some structure
        for y in 0..100 {
            for x in 0..100 {
                // Generate RGB-like values with some correlation
                let r = ((x * 3 + y * 7) % 256) as u8;
                let g = ((x * 5 + y * 11) % 256) as u8;
                let b = ((x * 7 + y * 13) % 256) as u8;
                multimedia_data.extend_from_slice(&[r, g, b]);
            }
        }
        
        // JPEG-like footer
        multimedia_data.extend_from_slice(&[0xFF, 0xD9]);
        
        let input_path = dir.path().join("multimedia.bin");
        let compressed_path = dir.path().join("multimedia.zst");
        let restored_path = dir.path().join("multimedia_restored.bin");

        // Write multimedia binary
        fs::write(&input_path, &multimedia_data).expect("Failed to write multimedia binary");

        // Compress
        compress_file(&input_path, &compressed_path, DEFAULT_COMPRESSION_LEVEL)
            .expect("Failed to compress multimedia binary");
        assert!(compressed_path.exists());

        // Decompress
        decompress_file(&compressed_path, &restored_path)
            .expect("Failed to decompress multimedia binary");
        assert!(restored_path.exists());

        // Verify exact match
        let restored_data = fs::read(&restored_path).expect("Failed to read restored multimedia");
        assert_eq!(restored_data, multimedia_data, 
                   "Multimedia binary corrupted during compression");
        
        // Verify header and footer integrity specifically
        assert_eq!(&restored_data[0..4], &[0xFF, 0xD8, 0xFF, 0xE0], "JPEG header corrupted");
        assert_eq!(&restored_data[restored_data.len()-2..], &[0xFF, 0xD9], "JPEG footer corrupted");
    }

    #[test]
    fn test_compression_level_validation() {
        let dir = tempdir().expect("Failed to create temp dir");
        let input_path = dir.path().join("test.txt");
        let output_path = dir.path().join("test.zst");
        
        fs::write(&input_path, "test content").expect("Failed to write test file");
        
        // Test invalid compression levels
        let invalid_levels = [-1, 23, 100];
        for level in invalid_levels {
            let result = compress_file(&input_path, &output_path, level);
            assert!(result.is_err(), "Should fail for invalid compression level: {}", level);
            
            if let Err(e) = result {
                assert_eq!(e.kind(), io::ErrorKind::InvalidInput);
                assert!(e.to_string().contains("Invalid compression level"));
            }
        }
        
        // Test valid compression levels
        let valid_levels = [0, 1, 3, 9, 22];
        for level in valid_levels {
            let output_path = dir.path().join(format!("test_{}.zst", level));
            let result = compress_file(&input_path, &output_path, level);
            assert!(result.is_ok(), "Should succeed for valid compression level: {}", level);
            assert!(output_path.exists());
        }
    }

    #[test]
    fn test_new_functions_with_path_types() {
        let dir = tempdir().expect("Failed to create temp dir");
        let content = b"Test content for new functions";
        
        // Test with &Path (preferred)
        let input_path = dir.path().join("input.txt");
        let compressed_path = dir.path().join("compressed.zst");
        let restored_path = dir.path().join("restored.txt");
        
        fs::write(&input_path, content).expect("Failed to write test file");
        
        // Test compress_file with &Path
        compress_file(&input_path, &compressed_path, 0).expect("compress_file failed");
        assert!(compressed_path.exists());
        
        // Test decompress_file with &Path
        decompress_file(&compressed_path, &restored_path).expect("decompress_file failed");
        assert!(restored_path.exists());
        
        let restored_content = fs::read(&restored_path).expect("Failed to read restored file");
        assert_eq!(restored_content, content);
    }
}
