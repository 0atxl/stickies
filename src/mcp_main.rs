use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    tool, tool_router,
};
use serde::{Deserialize, Serialize};

mod mcp_limits;
#[cfg(target_os = "windows")]
mod mcp_windows;
#[cfg(target_os = "windows")]
use mcp_limits::IPC_TIMEOUT;
use mcp_limits::{MAX_PAYLOAD_BYTES, MCP_IPC_VERSION};
#[cfg(target_os = "linux")]
const IPC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum IpcRequest {
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
        formatting: Option<Vec<SpanParam>>,
        color: Option<String>,
    },
    UpdateNote {
        version: u32,
        operation_id: String,
        id: String,
        expected_revision: String,
        title: Option<String>,
        body: Option<String>,
        formatting: Option<Vec<SpanParam>>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum IpcResponse {
    Ok { data: serde_json::Value },
    Error { code: String, message: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct SpanParam {
    #[schemars(description = "Start character offset in Unicode scalar values")]
    pub start: i32,
    #[schemars(description = "End character offset in Unicode scalar values")]
    pub end: i32,
    #[schemars(description = "Formatting mark: 'bold', 'italic', 'underline', 'strike'")]
    pub mark: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct FindNotesParams {
    #[schemars(
        description = "Optional search query text to filter notes by title and body content"
    )]
    query: Option<String>,
    #[schemars(description = "Pagination offset (zero-based, default 0)")]
    offset: Option<usize>,
    #[schemars(description = "Maximum number of notes to return (default 20, max 50)")]
    limit: Option<usize>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct GetNoteParams {
    #[schemars(description = "The decimal note ID (e.g. '1', '42')")]
    id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct CreateNoteParams {
    #[schemars(
        description = "Client-generated unique operation ID, e.g. UUID. Retry identical arguments with the same ID after a lost response; use a new ID for changed arguments."
    )]
    operation_id: String,
    #[schemars(description = "Optional note title, at most 500 UTF-8 bytes")]
    title: Option<String>,
    #[schemars(
        description = "Optional note body text. Serialized title, body and formatting together must fit 1,000,000 bytes."
    )]
    body: Option<String>,
    #[schemars(description = "Optional inline formatting spans (bold, italic, underline, strike)")]
    formatting: Option<Vec<SpanParam>>,
    #[schemars(
        description = "Optional note color: yellow, orange, red, pink, green, lime, teal, cyan, blue, indigo, purple, magenta, brown, gray, black, white. Defaults to yellow."
    )]
    color: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct UpdateNoteParams {
    #[schemars(
        description = "Client-generated unique operation ID, e.g. UUID. Retry identical arguments with the same ID after a lost response; use a new ID for changed arguments."
    )]
    operation_id: String,
    #[schemars(description = "The decimal note ID to update")]
    id: String,
    #[schemars(
        description = "The expected current revision of the note. Update will fail if concurrent modifications occurred."
    )]
    expected_revision: String,
    #[schemars(description = "New title. If omitted, existing title is preserved.")]
    title: Option<String>,
    #[schemars(description = "New body text. If omitted, existing body is preserved.")]
    body: Option<String>,
    #[schemars(
        description = "Formatting spans, supplied together with body. Required when body is updated; [] clears formatting. Omit both to preserve body and formatting."
    )]
    formatting: Option<Vec<SpanParam>>,
}

async fn call_stickies_gui(request: &IpcRequest) -> Result<serde_json::Value, (String, String)> {
    tokio::time::timeout(IPC_TIMEOUT, exchange_with_gui(request)).await
        .map_err(|_| ("timeout".into(), "Stickies IPC timed out. For a mutation, retry identical arguments with the same operation_id.".into()))?
}

async fn exchange_with_gui(request: &IpcRequest) -> Result<serde_json::Value, (String, String)> {
    let req_json = serde_json::to_string(request).map_err(|e| {
        (
            "internal_error".into(),
            format!("JSON serialization error: {e}"),
        )
    })?;

    if req_json.len() > MAX_PAYLOAD_BYTES {
        return Err((
            "payload_limit_exceeded".into(),
            "Request exceeds the IPC payload limit.".into(),
        ));
    }

    #[cfg(target_os = "linux")]
    {
        let connection = zbus::Connection::session().await.map_err(|e| {
            (
                "unavailable".into(),
                format!("Stickies is not running or D-Bus session is unavailable: {e}"),
            )
        })?;

        let proxy = zbus::Proxy::new(
            &connection,
            "dev.stickies.Stickies",
            "/dev/stickies/Mcp",
            "dev.stickies.Mcp1",
        )
        .await
        .map_err(|e| {
            (
                "unavailable".into(),
                format!("Could not connect to Stickies: {e}"),
            )
        })?;
        let reply: Option<(String,)> = proxy
            .call_with_flags(
                "Call",
                zbus::proxy::MethodFlags::NoAutoStart.into(),
                &(req_json,),
            )
            .await
            .map_err(|e| {
                (
                    "unavailable".into(),
                    format!("Stickies is not running or rejected the request: {e}"),
                )
            })?;
        let (reply,) =
            reply.ok_or_else(|| ("unavailable".into(), "Stickies returned no reply.".into()))?;
        if reply.len() > MAX_PAYLOAD_BYTES {
            return Err((
                "payload_limit_exceeded".into(),
                "Response exceeds the IPC payload limit.".into(),
            ));
        }

        let response: IpcResponse = serde_json::from_str(&reply).map_err(|e| {
            (
                "internal_error".into(),
                format!("Invalid response JSON from Stickies: {e}"),
            )
        })?;

        match response {
            IpcResponse::Ok { data } => Ok(data),
            IpcResponse::Error { code, message } => Err((code, message)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let pipe_name = mcp_windows::pipe_name().map_err(|e| {
            (
                "unavailable".into(),
                format!("Could not identify the Windows user: {e}"),
            )
        })?;
        let mut client = loop {
            match tokio::net::windows::named_pipe::ClientOptions::new().open(&pipe_name) {
                Ok(client) => break client,
                Err(error)
                    if error.raw_os_error()
                        == Some(windows_sys::Win32::Foundation::ERROR_PIPE_BUSY as i32) =>
                {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Err(error) => {
                    return Err((
                        "unavailable".into(),
                        format!("Stickies is not running or access was denied: {error}"),
                    ));
                }
            }
        };

        let req_bytes = req_json.as_bytes();
        let req_len = (req_bytes.len() as u32).to_le_bytes();
        client.write_all(&req_len).await.map_err(|e| {
            (
                "io_error".into(),
                format!("Failed to write length to pipe: {e}"),
            )
        })?;
        client.write_all(req_bytes).await.map_err(|e| {
            (
                "io_error".into(),
                format!("Failed to write request to pipe: {e}"),
            )
        })?;
        let mut len_bytes = [0u8; 4];
        client.read_exact(&mut len_bytes).await.map_err(|e| {
            (
                "io_error".into(),
                format!("Failed to read response length from pipe: {e}"),
            )
        })?;
        let resp_len = u32::from_le_bytes(len_bytes) as usize;
        if resp_len > MAX_PAYLOAD_BYTES {
            return Err((
                "payload_limit_exceeded".into(),
                "Response exceeds the IPC payload limit.".into(),
            ));
        }
        let mut resp_buf = vec![0u8; resp_len];
        client.read_exact(&mut resp_buf).await.map_err(|e| {
            (
                "io_error".into(),
                format!("Failed to read response body from pipe: {e}"),
            )
        })?;

        // The full response is now in memory. This bounded acknowledgement lets
        // the server disconnect safely; an ACK error must not discard the reply.
        let _ = client.write_u8(mcp_limits::RESPONSE_CONSUMED).await;

        let resp_str = String::from_utf8(resp_buf).map_err(|e| {
            (
                "internal_error".into(),
                format!("Invalid UTF-8 from Stickies pipe: {e}"),
            )
        })?;

        let response: IpcResponse = serde_json::from_str(&resp_str).map_err(|e| {
            (
                "internal_error".into(),
                format!("Invalid JSON from Stickies pipe: {e}"),
            )
        })?;

        match response {
            IpcResponse::Ok { data } => Ok(data),
            IpcResponse::Error { code, message } => Err((code, message)),
        }
    }
}

fn error_to_tool_result(code: String, message: String) -> CallToolResult {
    let mut res = CallToolResult::structured_error(serde_json::json!({
        "error": code,
        "message": message
    }));
    res.content = vec![ContentBlock::text(format!("Error [{code}]: {message}"))];
    res
}

#[derive(Clone)]
struct StickiesMcpServer;

#[tool_router(server_handler)]
impl StickiesMcpServer {
    #[tool(
        name = "notes_find",
        description = "Find active notes matching an optional text query, or list active notes with pagination. Live unsaved edits in open notes are reflected automatically."
    )]
    async fn notes_find(
        &self,
        Parameters(params): Parameters<FindNotesParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let req = IpcRequest::FindNotes {
            version: MCP_IPC_VERSION,
            query: params.query,
            offset: params.offset,
            limit: params.limit,
        };

        match call_stickies_gui(&req).await {
            Ok(data) => {
                let summary = if let Some(notes) = data.get("notes").and_then(|n| n.as_array()) {
                    let total = data
                        .get("total_count")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);
                    let mut text = format!("Found {} note(s):\n", total);
                    for item in notes {
                        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let title = item
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("(Untitled)");
                        let excerpt = item.get("excerpt").and_then(|v| v.as_str()).unwrap_or("");
                        let rev = item.get("revision").and_then(|v| v.as_str()).unwrap_or("");
                        text.push_str(&format!("- [{id}] \"{title}\" (rev: {rev}): {excerpt}\n"));
                    }
                    text
                } else {
                    "No notes found.".into()
                };

                let mut res = CallToolResult::structured(data);
                res.content = vec![ContentBlock::text(summary)];
                Ok(res)
            }
            Err((code, message)) => Ok(error_to_tool_result(code, message)),
        }
    }

    #[tool(
        name = "notes_get",
        description = "Retrieve the full text, formatting spans, color, and current revision of an active note by its ID."
    )]
    async fn notes_get(
        &self,
        Parameters(params): Parameters<GetNoteParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let req = IpcRequest::GetNote {
            version: MCP_IPC_VERSION,
            id: params.id,
        };

        match call_stickies_gui(&req).await {
            Ok(data) => {
                let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let title = data.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let body = data.get("body").and_then(|v| v.as_str()).unwrap_or("");
                let rev = data.get("revision").and_then(|v| v.as_str()).unwrap_or("");
                let color = data.get("color").and_then(|v| v.as_str()).unwrap_or("");
                let pinned = data
                    .get("pinned")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let summary = format!(
                    "Note {id}: \"{title}\" (color: {color}, pinned: {pinned}, rev: {rev})\n\n{body}"
                );

                let mut res = CallToolResult::structured(data);
                res.content = vec![ContentBlock::text(summary)];
                Ok(res)
            }
            Err((code, message)) => Ok(error_to_tool_result(code, message)),
        }
    }

    #[tool(
        name = "notes_create",
        description = "Create a new note. Requires Read/write access in Stickies Settings. Returns the new note's ID and initial revision."
    )]
    async fn notes_create(
        &self,
        Parameters(params): Parameters<CreateNoteParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let req = IpcRequest::CreateNote {
            version: MCP_IPC_VERSION,
            operation_id: params.operation_id,
            title: params.title,
            body: params.body,
            formatting: params.formatting,
            color: params.color,
        };

        match call_stickies_gui(&req).await {
            Ok(data) => {
                let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let rev = data.get("revision").and_then(|v| v.as_str()).unwrap_or("");
                let summary = format!("Created note {id} with initial revision {rev}.");

                let mut res = CallToolResult::structured(data);
                res.content = vec![ContentBlock::text(summary)];
                Ok(res)
            }
            Err((code, message)) => Ok(error_to_tool_result(code, message)),
        }
    }

    #[tool(
        name = "notes_update",
        description = "Update an active note. Read its current revision first. A conflict requires rereading before retrying. Supply body and formatting together and retain current checklist markers such as - [x] for completed items. Title is limited to 500 UTF-8 bytes; serialized title, body and formatting together to 1,000,000 bytes."
    )]
    async fn notes_update(
        &self,
        Parameters(params): Parameters<UpdateNoteParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let req = IpcRequest::UpdateNote {
            version: MCP_IPC_VERSION,
            operation_id: params.operation_id,
            id: params.id,
            expected_revision: params.expected_revision,
            title: params.title,
            body: params.body,
            formatting: params.formatting,
        };

        match call_stickies_gui(&req).await {
            Ok(data) => {
                let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let rev = data.get("revision").and_then(|v| v.as_str()).unwrap_or("");
                let summary = format!("Updated note {id}. New revision is {rev}.");

                let mut res = CallToolResult::structured(data);
                res.content = vec![ContentBlock::text(summary)];
                Ok(res)
            }
            Err((code, message)) => Ok(error_to_tool_result(code, message)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = rmcp::transport::io::stdio();
    let server = StickiesMcpServer;
    let running = rmcp::serve_server(server, transport).await?;
    let _ = running.waiting().await;
    Ok(())
}
