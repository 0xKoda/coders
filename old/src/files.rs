use anyhow::Result;
use std::path::Path;
use crate::app::{Change, ChangeType, FileDiff};

/// Smart merge algorithm for combining original and modified code
pub fn smart_merge(original: &str, new: &str) -> FileDiff {
    let original_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // If the number of lines is significantly different, treat it as a full file replacement
    if (new_lines.len() as f32 / original_lines.len() as f32).abs() > 0.5 && 
       (original_lines.len() as i32 - new_lines.len() as i32).abs() > 5 {
        return full_file_diff(&original_lines, &new_lines);
    }

    let mut updated_lines = original_lines.clone();
    let mut changes = Vec::new();

    // Process matching line ranges
    for (i, (old_line, new_line)) in original_lines.iter().zip(new_lines.iter()).enumerate() {
        if old_line != new_line {
            changes.push(Change {
                change_type: ChangeType::Modify,
                line_number: i + 1,
                content: new_line.to_string(),
            });
            if i < updated_lines.len() {
                updated_lines[i] = new_line;
            }
        }
    }

    // Process added lines
    for (i, new_line) in new_lines.iter().enumerate().skip(original_lines.len()) {
        changes.push(Change {
            change_type: ChangeType::Insert,
            line_number: i + 1,
            content: new_line.to_string(),
        });
        updated_lines.push(new_line);
    }

    // Process removed lines
    for i in new_lines.len()..original_lines.len() {
        changes.push(Change {
            change_type: ChangeType::Delete,
            line_number: i + 1,
            content: original_lines[i].to_string(),
        });
    }

    FileDiff {
        original: original.to_string(),
        modified: updated_lines.join("\n"),
        changes,
    }
}

/// Full file diff for complete replacements
fn full_file_diff(original_lines: &[&str], new_lines: &[&str]) -> FileDiff {
    let mut changes = Vec::new();

    // Compare each line and record the differences
    for (i, line) in new_lines.iter().enumerate() {
        if i < original_lines.len() {
            if line != &original_lines[i] {
                changes.push(Change {
                    change_type: ChangeType::Modify,
                    line_number: i + 1,
                    content: line.to_string(),
                });
            }
        } else {
            changes.push(Change {
                change_type: ChangeType::Insert,
                line_number: i + 1,
                content: line.to_string(),
            });
        }
    }

    // Mark lines that exist in original but not in new as deleted
    for i in new_lines.len()..original_lines.len() {
        changes.push(Change {
            change_type: ChangeType::Delete,
            line_number: i + 1,
            content: original_lines[i].to_string(),
        });
    }

    FileDiff {
        original: original_lines.join("\n"),
        modified: new_lines.join("\n"),
        changes,
    }
}

/// Read a file and return its contents
pub fn read_file(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

/// Write content to a file
pub fn write_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content)?;
    Ok(())
}

/// List files in a directory
pub fn list_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut entries = Vec::new();
    
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        entries.push(path);
    }
    
    // Sort directories first, then files
    entries.sort_by(|a, b| {
        let a_is_dir = a.is_dir();
        let b_is_dir = b.is_dir();
        
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });
    
    Ok(entries)
} 