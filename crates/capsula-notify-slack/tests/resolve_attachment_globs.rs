//! Tests for the `resolve_attachment_globs` function.
#![expect(clippy::unwrap_used, reason = "unwrap is acceptable in test code")]

use capsula_notify_slack::resolve_attachment_globs;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_resolve_empty_globs() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    let globs = vec![];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert
    assert_eq!(result.len(), 0);
}

#[test]
fn test_resolve_single_file_pattern() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
    let globs = vec!["test.txt".to_string()];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].file_name().unwrap(), "test.txt");
}

#[test]
fn test_resolve_wildcard_pattern() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("file3.log"), "content").unwrap();
    let globs = vec!["*.txt".to_string()];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert
    assert_eq!(result.len(), 2);
    let filenames: Vec<_> = result
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect();
    assert!(filenames.contains(&"file1.txt"));
    assert!(filenames.contains(&"file2.txt"));
    assert!(!filenames.contains(&"file3.log"));
}

#[test]
fn test_resolve_multiple_patterns() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("data.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("output.log"), "content").unwrap();
    fs::write(temp_dir.path().join("result.csv"), "content").unwrap();
    let globs = vec!["*.txt".to_string(), "*.log".to_string()];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert
    assert_eq!(result.len(), 2);
    let filenames: Vec<_> = result
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect();
    assert!(filenames.contains(&"data.txt"));
    assert!(filenames.contains(&"output.log"));
    assert!(!filenames.contains(&"result.csv"));
}

#[test]
fn test_resolve_subdirectory_pattern() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("outputs");
    fs::create_dir(&subdir).unwrap();
    fs::write(subdir.join("result.png"), "content").unwrap();
    fs::write(subdir.join("graph.png"), "content").unwrap();
    let globs = vec!["outputs/*.png".to_string()];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert
    assert_eq!(result.len(), 2);
}

#[test]
fn test_resolve_ignores_directories() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("file.txt"), "content").unwrap();
    fs::create_dir(temp_dir.path().join("dir.txt")).unwrap();
    let globs = vec!["*.txt".to_string()];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert - only the file should be included, not the directory
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].file_name().unwrap(), "file.txt");
}

#[test]
fn test_resolve_no_matches() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
    let globs = vec!["*.png".to_string()];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert
    assert_eq!(result.len(), 0);
}

#[test]
fn test_resolve_truncates_to_10_files() {
    // Arrange - create 15 files
    let temp_dir = TempDir::new().unwrap();
    for i in 0..15 {
        fs::write(temp_dir.path().join(format!("file{i}.txt")), "content").unwrap();
    }
    let globs = vec!["*.txt".to_string()];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert - should be truncated to 10 files
    assert_eq!(result.len(), 10);
}

#[test]
fn test_resolve_invalid_pattern_error() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    let globs = vec!["[".to_string()]; // Invalid glob pattern

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path());

    // Assert
    assert!(result.is_err());
}

#[test]
fn test_resolve_relative_path_patterns() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    let subdir1 = temp_dir.path().join("dir1");
    let subdir2 = temp_dir.path().join("dir2");
    fs::create_dir(&subdir1).unwrap();
    fs::create_dir(&subdir2).unwrap();
    fs::write(subdir1.join("a.txt"), "content").unwrap();
    fs::write(subdir2.join("b.txt"), "content").unwrap();
    let globs = vec!["dir1/*.txt".to_string(), "dir2/*.txt".to_string()];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert
    assert_eq!(result.len(), 2);
}

#[test]
fn test_resolve_overlapping_patterns() {
    // Arrange - patterns that might match the same file
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
    let globs = vec!["*.txt".to_string(), "test.*".to_string()];

    // Act
    let result = resolve_attachment_globs(&globs, temp_dir.path()).unwrap();

    // Assert - file might appear twice (this tests current behavior)
    // The function doesn't deduplicate, so the same file can appear multiple times
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].file_name().unwrap(), "test.txt");
    assert_eq!(result[1].file_name().unwrap(), "test.txt");
}
