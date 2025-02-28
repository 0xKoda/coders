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
        explanation_text: None,
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
        explanation_text: None,
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

/// Calculate diff between original and modified code
pub fn calculate_diff(original: &str, modified: &str) -> Vec<crate::app::Change> {
    let mut changes = Vec::new();
    
    // Split the original and modified code into lines
    let original_lines: Vec<&str> = original.lines().collect();
    let modified_lines: Vec<&str> = modified.lines().collect();
    
    // Use a simple line-by-line comparison for now
    // This is a basic implementation and could be improved with a proper diff algorithm
    let max_lines = std::cmp::max(original_lines.len(), modified_lines.len());
    
    for i in 0..max_lines {
        let original_line = original_lines.get(i).map(|s| *s).unwrap_or("");
        let modified_line = modified_lines.get(i).map(|s| *s).unwrap_or("");
        
        if i >= original_lines.len() {
            // Line was added
            changes.push(crate::app::Change {
                change_type: crate::app::ChangeType::Insert,
                line_number: i,
                content: modified_line.to_string(),
            });
        } else if i >= modified_lines.len() {
            // Line was deleted
            changes.push(crate::app::Change {
                change_type: crate::app::ChangeType::Delete,
                line_number: i,
                content: original_line.to_string(),
            });
        } else if original_line != modified_line {
            // Line was modified
            changes.push(crate::app::Change {
                change_type: crate::app::ChangeType::Modify,
                line_number: i,
                content: modified_line.to_string(),
            });
        }
    }
    
    changes
}

/// Apply a patch to a file
pub fn apply_patch(file_path: &std::path::Path, patch: &str) -> anyhow::Result<()> {
    // Read the original file content
    let original_content = std::fs::read_to_string(file_path)?;
    
    // Apply the patch to get the modified content
    let modified_content = apply_patch_to_string(&original_content, patch)?;
    
    // Write the modified content back to the file
    std::fs::write(file_path, modified_content)?;
    
    Ok(())
}

/// Apply a patch to a string
pub fn apply_patch_to_string(original: &str, patch: &str) -> anyhow::Result<String> {
    // Parse the patch
    let diff = parse_diff(patch)?;
    
    // Apply the diff to the original content
    let modified = apply_diff(original, &diff)?;
    
    Ok(modified)
}

/// Apply a diff to a string
pub fn apply_diff(original: &str, diff: &crate::app::FileDiff) -> anyhow::Result<String> {
    // If there are no changes, return the original content
    if diff.changes.is_empty() {
        return Ok(original.to_string());
    }
    
    // Split the original content into lines
    let original_lines: Vec<&str> = original.lines().collect();
    let mut result_lines: Vec<String> = original_lines.iter().map(|&s| s.to_string()).collect();
    
    // Sort changes by line number in reverse order to avoid index shifting
    let mut sorted_changes = diff.changes.clone();
    sorted_changes.sort_by(|a, b| b.line_number.cmp(&a.line_number));
    
    // Apply each change
    for change in sorted_changes {
        let line_idx = change.line_number;
        
        match change.change_type {
            crate::app::ChangeType::Insert => {
                // Insert a new line
                if line_idx >= result_lines.len() {
                    result_lines.push(change.content);
                } else {
                    result_lines.insert(line_idx, change.content);
                }
            },
            crate::app::ChangeType::Delete => {
                // Delete a line if it exists
                if line_idx < result_lines.len() {
                    result_lines.remove(line_idx);
                }
            },
            crate::app::ChangeType::Modify => {
                // Modify a line if it exists
                if line_idx < result_lines.len() {
                    result_lines[line_idx] = change.content;
                }
            },
        }
    }
    
    // Join the lines back into a string
    Ok(result_lines.join("\n"))
}

/// Parse a diff string into a FileDiff struct
pub fn parse_diff(diff_str: &str) -> anyhow::Result<crate::app::FileDiff> {
    // Extract explanation text if present (text before any code blocks)
    let explanation_text = extract_explanation_text(diff_str);
    
    // For now, we'll create a simple diff that contains the entire content
    // In a real implementation, this would parse a proper diff format
    Ok(FileDiff {
        original: String::new(),
        modified: diff_str.to_string(),
        changes: Vec::new(),
        explanation_text,
    })
}

/// Extract explanation text from a diff string
fn extract_explanation_text(diff_str: &str) -> Option<String> {
    // Simple heuristic: extract text before the first code block or file marker
    let lines: Vec<&str> = diff_str.lines().collect();
    let mut explanation = Vec::new();
    
    for line in lines {
        // Stop at code block markers or file markers
        if line.starts_with("```") || line.starts_with("File:") {
            break;
        }
        explanation.push(line);
    }
    
    if explanation.is_empty() {
        None
    } else {
        Some(explanation.join("\n"))
    }
}

/// Create a diff between two files
pub fn create_diff(original_path: &std::path::Path, modified_path: &std::path::Path) -> anyhow::Result<crate::app::FileDiff> {
    // Read the original and modified file content
    let original = std::fs::read_to_string(original_path)?;
    let modified = std::fs::read_to_string(modified_path)?;
    
    // Calculate the diff
    let changes = calculate_diff(&original, &modified);
    
    Ok(FileDiff {
        original,
        modified,
        changes,
        explanation_text: None,
    })
} 