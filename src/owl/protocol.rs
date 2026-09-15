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
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecorationType {
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RustOwlCursorResult {
    #[serde(default)]
    pub decorations: Vec<RustOwlDecoration>,
}
