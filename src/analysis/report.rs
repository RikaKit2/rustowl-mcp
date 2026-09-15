use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::owl::client::RustOwlClient;
use crate::owl::protocol::{DecorationType, RustOwlDecoration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableLifetimeReport {
    pub file: String,
    pub target: CursorTarget,
    pub live_span: Option<LiveSpan>,
    pub active_borrows: Vec<BorrowSpan>,
    pub moves: Vec<MoveEvent>,
    pub conflicts: Vec<ConflictIssue>,
    pub summary: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actionable_fixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorTarget {
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSpan {
    pub start_line: u32,
    pub end_line: u32,
    pub certainty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowSpan {
    pub kind: String, // "immutable" or "mutable"
    pub lines: (u32, u32),
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveEvent {
    pub line: u32,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictIssue {
    pub kind: String,
    pub lines: (u32, u32),
    pub description: String,
    pub recommendation: String,
}

pub fn normalize_decorations(
    file: &Path,
    target_line: u32,
    target_col: u32,
    decorations: &[RustOwlDecoration],
) -> VariableLifetimeReport {
    let mut live_span = None;
    let mut active_borrows = Vec::new();
    let mut moves = Vec::new();
    let mut conflicts = Vec::new();
    let mut actionable_fixes = Vec::new();

    for d in decorations {
        let start_l = d.range.start.line + 1; // Convert 0-indexed to 1-indexed for humans/agents
        let end_l = d.range.end.line + 1;

        match d.kind {
            DecorationType::DefinitelyLive | DecorationType::Lifetime => {
                live_span = Some(LiveSpan {
                    start_line: start_l,
                    end_line: end_l,
                    certainty: "definitely_live".to_string(),
                });
            }
            DecorationType::MaybeInitialized => {
                live_span = Some(LiveSpan {
                    start_line: start_l,
                    end_line: end_l,
                    certainty: "maybe_initialized".to_string(),
                });
            }
            DecorationType::ImmBorrow => {
                active_borrows.push(BorrowSpan {
                    kind: "immutable".to_string(),
                    lines: (start_l, end_l),
                    description: d.hover_text.clone(),
                });
            }
            DecorationType::MutBorrow => {
                active_borrows.push(BorrowSpan {
                    kind: "mutable".to_string(),
                    lines: (start_l, end_l),
                    description: d.hover_text.clone(),
                });
            }
            DecorationType::Move => {
                moves.push(MoveEvent {
                    line: start_l,
                    kind: "move".to_string(),
                });
            }
            DecorationType::Call => {
                moves.push(MoveEvent {
                    line: start_l,
                    kind: "call_consumption".to_string(),
                });
            }
            DecorationType::Outlive => {
                let rec = format!(
                    "Lifetime conflict on lines {}-{}: reference outlives the value. Scope references or consider taking ownership.",
                    start_l, end_l
                );
                actionable_fixes.push(rec.clone());
                conflicts.push(ConflictIssue {
                    kind: "outlive_conflict".to_string(),
                    lines: (start_l, end_l),
                    description: "Reference outlives borrowed data".to_string(),
                    recommendation: rec,
                });
            }
            DecorationType::SharedMut => {
                let rec = format!(
                    "Aliasing violation on lines {}-{}: mutable borrow overlaps with another borrow. Narrow borrow scope with `{{ ... }}`.",
                    start_l, end_l
                );
                actionable_fixes.push(rec.clone());
                conflicts.push(ConflictIssue {
                    kind: "aliasing_conflict".to_string(),
                    lines: (start_l, end_l),
                    description: "Cannot borrow as mutable while also borrowed".to_string(),
                    recommendation: rec,
                });
            }
            DecorationType::Unknown => {}
        }
    }

    let summary = if conflicts.is_empty() {
        if active_borrows.is_empty() {
            "No active borrows or conflicts detected for this variable.".to_string()
        } else {
            format!(
                "Clean lifetime with {} active borrow(s). No borrow-checker conflicts.",
                active_borrows.len()
            )
        }
    } else {
        format!(
            "ALERT: {} borrow-checker conflict(s) detected! Immediate resolution recommended.",
            conflicts.len()
        )
    };

    VariableLifetimeReport {
        file: file.display().to_string(),
        target: CursorTarget {
            line: target_line,
            col: target_col,
        },
        live_span,
        active_borrows,
        moves,
        conflicts,
        summary,
        actionable_fixes,
    }
}

pub async fn inspect_cursor(file: &Path, line: u32, col: u32) -> Result<VariableLifetimeReport> {
    let workspace = find_workspace_root(file)
        .unwrap_or_else(|| file.parent().unwrap_or(Path::new(".")).to_path_buf());

    let mut client = RustOwlClient::new_or_fallback(&workspace).await;
    let decorations = client.query_cursor(file, line, col).await?;
    Ok(normalize_decorations(file, line, col, &decorations))
}

pub async fn inspect_line(file: &Path, line: u32) -> Result<Vec<VariableLifetimeReport>> {
    if !file.exists() {
        anyhow::bail!("File does not exist: {}", file.display());
    }

    let content = tokio::fs::read_to_string(file).await?;
    let line_text = match content.lines().nth(line.saturating_sub(1) as usize) {
        Some(l) => l,
        None => return Ok(vec![]),
    };

    // Extract identifiers and their column offsets on this line
    let mut reports = Vec::new();
    let mut current_ident = String::new();
    let mut start_col = 0;

    for (idx, ch) in line_text.char_indices() {
        if ch.is_alphanumeric() || ch == '_' {
            if current_ident.is_empty() {
                start_col = idx as u32 + 1;
            }
            current_ident.push(ch);
        } else {
            if !current_ident.is_empty() {
                // Filter out common language keywords that are never variables
                if !is_keyword(&current_ident) {
                    if let Ok(report) = inspect_cursor(file, line, start_col).await {
                        reports.push(report);
                    }
                }
                current_ident.clear();
            }
        }
    }

    if !current_ident.is_empty() && !is_keyword(&current_ident) {
        if let Ok(report) = inspect_cursor(file, line, start_col).await {
            reports.push(report);
        }
    }

    Ok(reports)
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "fn" | "let"
            | "mut"
            | "pub"
            | "struct"
            | "enum"
            | "impl"
            | "for"
            | "while"
            | "loop"
            | "if"
            | "else"
            | "match"
            | "return"
            | "use"
            | "mod"
            | "crate"
            | "self"
            | "Self"
            | "super"
            | "where"
            | "as"
            | "true"
            | "false"
    )
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        if current.join("Cargo.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}
