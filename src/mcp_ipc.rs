use serde::{Deserialize, Serialize};

use crate::text_editing::Span;

pub use crate::mcp_limits::{MAX_PAYLOAD_BYTES, MCP_IPC_VERSION};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum IpcRequest {
    FindNotes {
        version: u32,
        query: Option<String>,
        offset: Option<usize>,
        limit: Option<usize>,
    },
    GetNote {
        version: u32,
        id: String,
    },
    CreateNote {
        version: u32,
        operation_id: String,
        title: Option<String>,
        body: Option<String>,
        formatting: Option<Vec<Span>>,
        color: Option<String>,
    },
    UpdateNote {
        version: u32,
        operation_id: String,
        id: String,
        expected_revision: String,
        title: Option<String>,
        body: Option<String>,
        formatting: Option<Vec<Span>>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IpcResponse {
    Ok { data: serde_json::Value },
    Error { code: String, message: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub excerpt: String,
    pub revision: String,
    pub color: String,
    pub pinned: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct FindNotesResult {
    pub notes: Vec<NoteSummary>,
    pub total_count: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct GetNoteResult {
    pub id: String,
    pub title: String,
    pub body: String,
    pub formatting: Vec<Span>,
    pub revision: String,
    pub color: String,
    pub pinned: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct MutationResult {
    pub id: String,
    pub revision: String,
}
