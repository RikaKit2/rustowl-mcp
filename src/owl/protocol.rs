use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustOwlCursorParams {
    pub document: TextDocumentIdentifier,
    #[serde(rename = "textDocument", skip_serializing_if = "Option::is_none")]
    pub text_document: Option<TextDocumentIdentifier>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecorationType {
    Lifetime,
    DefinitelyLive,
    MaybeInitialized,
    ImmBorrow,
    MutBorrow,
    Move,
    Call,
    Outlive,
    SharedMut,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustOwlDecoration {
    pub range: Range,
    #[serde(rename = "type")]
    pub kind: DecorationType,
    #[serde(default)]
    pub hover_text: Option<String>,
    #[serde(default)]
    pub overlapped: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RustOwlCursorResult {
    #[serde(default)]
    pub is_analyzed: bool,
    #[serde(default)]
    pub decorations: Vec<RustOwlDecoration>,
}

impl RustOwlDecoration {
    pub fn new(range: Range, kind: DecorationType) -> Self {
        Self {
            range,
            kind,
            hover_text: None,
            overlapped: false,
        }
    }
}
