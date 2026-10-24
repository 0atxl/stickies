use std::rc::Rc;

use gtk::prelude::*;

use crate::{
    Ui,
    app::{Note, NoteColor, NoteId},
    config::McpAccess,
    mcp_ipc::{
        FindNotesResult, GetNoteResult, IpcRequest, IpcResponse, MAX_PAYLOAD_BYTES,
        MCP_IPC_VERSION, MutationResult, NoteSummary,
    },
    storage::{color_name, parse_color},
    text_editing::Span,
};

const MAX_TITLE_BYTES: usize = 500;
// Serialized title, body and formatting together, including JSON escaping.
const MAX_NOTE_BYTES: usize = 1_000_000;

// Use the same tag serialization as open sessions: sorted marks, merged ranges,
// and no zero-length spans. Checklist prefix offsets remain plain-text offsets.
fn canonical_formatting(body: &str, formatting: &[Span]) -> Vec<Span> {
    let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    buffer.set_text(body);
    crate::text_editing::apply(&buffer, formatting);
    crate::text_editing::spans(&buffer)
}

fn structured_hash(value: &impl serde::Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("MCP strings and spans serialize to JSON");
    gtk::glib::compute_checksum_for_data(gtk::glib::ChecksumType::Sha256, &bytes)
        .expect("SHA-256 is supported by GLib")
        .to_string()
}

pub fn compute_revision(title: &str, body: &str, formatting: &[Span]) -> String {
    structured_hash(&(title, body, canonical_formatting(body, formatting)))
}

fn storage_error(error: impl std::fmt::Display) -> (String, String) {
    ("storage_error".into(), format!("Database error: {error}"))
}

fn invalid_argument(message: &str) -> (String, String) {
    ("invalid_argument".into(), message.into())
}

fn validate_content(title: &str, body: &str, formatting: &[Span]) -> Result<(), (String, String)> {
    if title.contains('\0') || body.contains('\0') {
        return Err(invalid_argument("Note text cannot contain NUL characters."));
    }
    if title.len() > MAX_TITLE_BYTES {
        return Err(invalid_argument("Note title exceeds the 500-byte limit."));
    }
    let bytes = serde_json::to_vec(&(title, body, formatting))
        .expect("MCP strings and spans serialize to JSON");
    if bytes.len() > MAX_NOTE_BYTES {
        return Err((
            "payload_limit_exceeded".into(),
            "Serialized title, body and formatting exceed the 1,000,000-byte note limit.".into(),
        ));
    }
    validate_formatting_spans(formatting, body)
}

fn current_note(ui: &Ui, id: NoteId) -> Result<(Note, Vec<Span>), (String, String)> {
    // Check eligibility even when the note still has an open session.
    let note = ui
        .storage
        .get_active_note(id)
        .map_err(storage_error)?
        .ok_or_else(|| {
            (
                "not_found".into(),
                "Note was not found or is excluded.".into(),
            )
        })?;
    let formatting = ui.storage.load_formatting(id).map_err(storage_error)?;
    if let Some(session) = ui
        .sessions
        .borrow()
        .get(&id)
        .and_then(std::rc::Weak::upgrade)
    {
        return Ok((
            session.live_note(),
            session.live_formatting().map_err(storage_error)?,
        ));
    }
    validate_content(&note.title, &note.body, &formatting)?;
    let formatting = canonical_formatting(&note.body, &formatting);
    Ok((note, formatting))
}

fn make_unicode_safe_excerpt(body: &str, max_chars: usize) -> String {
    let mut chars = body.chars();
    let excerpt: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{excerpt}…")
    } else {
        excerpt
    }
}

fn validate_formatting_spans(spans: &[Span], body: &str) -> Result<(), (String, String)> {
    let char_count = body.chars().count() as i32;
    for span in spans {
        if span.start < 0 || span.end < span.start || span.end > char_count {
            return Err((
                "invalid_argument".into(),
                format!(
                    "Formatting span [{}, {}] is outside body character bounds [0, {}].",
                    span.start, span.end, char_count
                ),
            ));
        }
    }
    Ok(())
}

fn parse_note_id(id_str: &str) -> Result<NoteId, (String, String)> {
    let value = id_str.trim().parse::<u64>().map_err(|_| {
        (
            "invalid_argument".into(),
            format!("Invalid note ID format: '{id_str}'. Expected decimal string."),
        )
    })?;
    Ok(NoteId::new(value))
}

fn handle_find_notes(
    ui: &Ui,
    query: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<FindNotesResult, (String, String)> {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(20).clamp(1, 50);

    let mut active_notes = ui
        .storage
        .get_all_active_notes()
        .map_err(|e| ("storage_error".into(), format!("Database error: {e}")))?;

    let sessions = ui.sessions.borrow();
    for note in &mut active_notes {
        if let Some(session) = sessions.get(&note.id).and_then(std::rc::Weak::upgrade) {
            *note = session.live_note();
        }
    }
    drop(sessions);
    if query.as_deref().is_some_and(|q| q.contains('\0')) {
        return Err(invalid_argument(
            "Search query cannot contain NUL characters.",
        ));
    }
    let query = query
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(gtk::glib::casefold);
    let filtered: Vec<Note> = active_notes
        .into_iter()
        .filter(|note| {
            query.as_ref().is_none_or(|query| {
                // Legacy text containing NUL is rejected before GLib conversion.
                !note.title.contains('\0')
                    && !note.body.contains('\0')
                    && (gtk::glib::casefold(&note.title).contains(&**query)
                        || gtk::glib::casefold(&note.body).contains(&**query))
            })
        })
        .collect();
    let total_count = filtered.len();
    let page: Vec<NoteSummary> = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|note| {
            let (note, formatting) = current_note(ui, note.id)?;
            validate_content(&note.title, &note.body, &formatting)?;
            Ok(NoteSummary {
                id: note.id.value().to_string(),
                revision: compute_revision(&note.title, &note.body, &formatting),
                excerpt: make_unicode_safe_excerpt(&note.body, 160),
                title: note.title,
                color: color_name(note.color).to_string(),
                pinned: note.pinned,
            })
        })
        .collect::<Result<_, (String, String)>>()?;

    let has_more = offset.saturating_add(page.len()) < total_count;

    Ok(FindNotesResult {
        notes: page,
        total_count,
        offset,
        limit,
        has_more,
    })
}

fn handle_get_note(ui: &Ui, id_str: &str) -> Result<GetNoteResult, (String, String)> {
    let id = parse_note_id(id_str)?;

    let (note, formatting) = current_note(ui, id)?;
    validate_content(&note.title, &note.body, &formatting)?;

    let revision = compute_revision(&note.title, &note.body, &formatting);

    Ok(GetNoteResult {
        id: note.id.value().to_string(),
        title: note.title,
        body: note.body,
        formatting,
        revision,
        color: color_name(note.color).to_string(),
        pinned: note.pinned,
    })
}

fn handle_create_note(
    ui: &Rc<Ui>,
    operation_id: &str,
    title: Option<String>,
    body: Option<String>,
    formatting: Option<Vec<Span>>,
    color: Option<String>,
) -> Result<MutationResult, (String, String)> {
    if operation_id.trim().is_empty() {
        return Err((
            "invalid_argument".into(),
            "operation_id cannot be empty.".into(),
        ));
    }

    let fingerprint = structured_hash(&("create", &title, &body, &formatting, &color));

    // Check existing receipt
    if let Some((existing_fp, existing_id, existing_rev)) = ui
        .storage
        .get_mutation_receipt(operation_id)
        .map_err(|e| ("storage_error".into(), format!("Database error: {e}")))?
    {
        if existing_fp == fingerprint {
            return Ok(MutationResult {
                id: existing_id.value().to_string(),
                revision: existing_rev,
            });
        } else {
            return Err((
                "operation_conflict".into(),
                format!("operation_id '{operation_id}' was already used with different arguments."),
            ));
        }
    }

    let note_color = match color {
        Some(name) => parse_color(&name).map_err(|_| {
            (
                "invalid_argument".into(),
                format!("Unknown color '{name}'."),
            )
        })?,
        None => {
            // Pick a color from palette
            NoteColor::ALL[0]
        }
    };

    let title = title.unwrap_or_default();
    let body = body.unwrap_or_default();
    let formatting = formatting.unwrap_or_default();
    validate_content(&title, &body, &formatting)?;
    let formatting = canonical_formatting(&body, &formatting);

    let revision = compute_revision(&title, &body, &formatting);

    let created = ui
        .storage
        .mcp_create_note(crate::storage::McpNewNote {
            operation_id,
            fingerprint: &fingerprint,
            title: &title,
            body: &body,
            formatting: &formatting,
            color: note_color,
            revision: &revision,
        })
        .map_err(|e| ("storage_error".into(), format!("Database error: {e}")))?;

    ui.refresh();

    Ok(MutationResult {
        id: created.id.value().to_string(),
        revision,
    })
}

fn handle_update_note(
    ui: &Rc<Ui>,
    operation_id: &str,
    id_str: &str,
    expected_revision: &str,
    title_opt: Option<String>,
    body_opt: Option<String>,
    formatting_opt: Option<Vec<Span>>,
) -> Result<MutationResult, (String, String)> {
    if operation_id.trim().is_empty() {
        return Err((
            "invalid_argument".into(),
            "operation_id cannot be empty.".into(),
        ));
    }
    if expected_revision.trim().is_empty() {
        return Err((
            "invalid_argument".into(),
            "expected_revision cannot be empty.".into(),
        ));
    }

    let id = parse_note_id(id_str)?;

    let fingerprint = structured_hash(&(
        "update",
        id_str,
        expected_revision,
        &title_opt,
        &body_opt,
        &formatting_opt,
    ));

    // Check existing receipt
    if let Some((existing_fp, existing_id, existing_rev)) = ui
        .storage
        .get_mutation_receipt(operation_id)
        .map_err(|e| ("storage_error".into(), format!("Database error: {e}")))?
    {
        if existing_fp == fingerprint {
            return Ok(MutationResult {
                id: existing_id.value().to_string(),
                revision: existing_rev,
            });
        } else {
            return Err((
                "operation_conflict".into(),
                format!("operation_id '{operation_id}' was already used with different arguments."),
            ));
        }
    }

    if body_opt.is_some() != formatting_opt.is_some() {
        return Err(invalid_argument(
            "Body and formatting must be supplied together; use [] to clear formatting.",
        ));
    }
    let (current_note, current_formatting) = current_note(ui, id)?;
    let open_session = ui
        .sessions
        .borrow()
        .get(&id)
        .and_then(std::rc::Weak::upgrade);

    let current_revision =
        compute_revision(&current_note.title, &current_note.body, &current_formatting);
    if current_revision != expected_revision {
        return Err((
            "conflict".into(),
            format!(
                "Note was modified concurrently. Expected revision '{expected_revision}', current revision is '{current_revision}'. Please re-fetch and retry."
            ),
        ));
    }

    let new_title = title_opt.as_deref().unwrap_or(&current_note.title);
    let new_body = body_opt.as_deref().unwrap_or(&current_note.body);
    let new_formatting = if body_opt.is_some() {
        // Body replacement requires explicit formatting; empty clears it.
        formatting_opt.unwrap_or_default()
    } else {
        // Title-only preserves body formatting.
        current_formatting
    };

    validate_content(new_title, new_body, &new_formatting)?;
    let new_formatting = canonical_formatting(new_body, &new_formatting);

    let new_revision = compute_revision(new_title, new_body, &new_formatting);

    let updated_note = Note {
        id,
        title: new_title.to_string(),
        body: new_body.to_string(),
        color: current_note.color,
        pinned: current_note.pinned,
    };

    ui.storage
        .mcp_update_note(
            operation_id,
            &fingerprint,
            &updated_note,
            &new_formatting,
            &new_revision,
        )
        .map_err(|e| ("storage_error".into(), format!("Database error: {e}")))?;

    // Publish to open session as an isolated undo step
    if let Some(session) = open_session {
        session.apply_mcp_update(
            title_opt.as_deref(),
            body_opt.as_deref(),
            Some(&new_formatting),
        );
    }

    ui.refresh();

    Ok(MutationResult {
        id: id.value().to_string(),
        revision: new_revision,
    })
}

pub fn handle_raw_request(ui: &Rc<Ui>, raw_json: &str) -> String {
    if raw_json.len() > MAX_PAYLOAD_BYTES {
        let err = IpcResponse::Error {
            code: "payload_limit_exceeded".into(),
            message: format!("Request payload exceeds {} byte limit.", MAX_PAYLOAD_BYTES),
        };
        return encode_response(err);
    }

    let request: IpcRequest = match serde_json::from_str(raw_json) {
        Ok(req) => req,
        Err(e) => {
            let err = IpcResponse::Error {
                code: "invalid_argument".into(),
                message: format!("Malformed request JSON: {e}"),
            };
            return encode_response(err);
        }
    };

    let access = ui.config.borrow().mcp_access;
    if access == McpAccess::Off {
        let err = IpcResponse::Error {
            code: "access_denied".into(),
            message: "MCP access is disabled in Stickies Settings.".into(),
        };
        return encode_response(err);
    }

    let response = match request {
        IpcRequest::FindNotes {
            version,
            query,
            offset,
            limit,
        } => {
            if version != MCP_IPC_VERSION {
                IpcResponse::Error {
                    code: "incompatible_version".into(),
                    message: format!(
                        "IPC version mismatch: server {MCP_IPC_VERSION}, client {version}"
                    ),
                }
            } else {
                match handle_find_notes(ui, query, offset, limit) {
                    Ok(result) => IpcResponse::Ok {
                        data: serde_json::to_value(result).unwrap_or_default(),
                    },
                    Err((code, message)) => IpcResponse::Error { code, message },
                }
            }
        }
        IpcRequest::GetNote { version, id } => {
            if version != MCP_IPC_VERSION {
                IpcResponse::Error {
                    code: "incompatible_version".into(),
                    message: format!(
                        "IPC version mismatch: server {MCP_IPC_VERSION}, client {version}"
                    ),
                }
            } else {
                match handle_get_note(ui, &id) {
                    Ok(result) => IpcResponse::Ok {
                        data: serde_json::to_value(result).unwrap_or_default(),
                    },
                    Err((code, message)) => IpcResponse::Error { code, message },
                }
            }
        }
        IpcRequest::CreateNote {
            version,
            operation_id,
            title,
            body,
            formatting,
            color,
        } => {
            if access != McpAccess::ReadWrite {
                IpcResponse::Error {
                    code: "access_denied".into(),
                    message:
                        "Write access is not enabled in Stickies Settings (currently Read-only)."
                            .into(),
                }
            } else if version != MCP_IPC_VERSION {
                IpcResponse::Error {
                    code: "incompatible_version".into(),
                    message: format!(
                        "IPC version mismatch: server {MCP_IPC_VERSION}, client {version}"
                    ),
                }
            } else {
                match handle_create_note(ui, &operation_id, title, body, formatting, color) {
                    Ok(result) => IpcResponse::Ok {
                        data: serde_json::to_value(result).unwrap_or_default(),
                    },
                    Err((code, message)) => IpcResponse::Error { code, message },
                }
            }
        }
        IpcRequest::UpdateNote {
            version,
            operation_id,
            id,
            expected_revision,
            title,
            body,
            formatting,
        } => {
            if access != McpAccess::ReadWrite {
                IpcResponse::Error {
                    code: "access_denied".into(),
                    message:
                        "Write access is not enabled in Stickies Settings (currently Read-only)."
                            .into(),
                }
            } else if version != MCP_IPC_VERSION {
                IpcResponse::Error {
                    code: "incompatible_version".into(),
                    message: format!(
                        "IPC version mismatch: server {MCP_IPC_VERSION}, client {version}"
                    ),
                }
            } else {
                match handle_update_note(
                    ui,
                    &operation_id,
                    &id,
                    &expected_revision,
                    title,
                    body,
                    formatting,
                ) {
                    Ok(result) => IpcResponse::Ok {
                        data: serde_json::to_value(result).unwrap_or_default(),
                    },
                    Err((code, message)) => IpcResponse::Error { code, message },
                }
            }
        }
    };

    encode_response(response)
}

fn encode_response(response: IpcResponse) -> String {
    let encoded = serde_json::to_string(&response).expect("IPC response serializes to JSON");
    if encoded.len() > MAX_PAYLOAD_BYTES {
        return serde_json::to_string(&IpcResponse::Error {
            code: "payload_limit_exceeded".into(),
            message: "Response exceeds the IPC payload limit.".into(),
        })
        .expect("IPC error serializes to JSON");
    }
    encoded
}

pub struct McpServer {
    #[cfg(target_os = "linux")]
    registration_id: Option<gtk::gio::RegistrationId>,
    #[cfg(target_os = "windows")]
    stop_signal: Option<tokio::sync::oneshot::Sender<()>>,
    #[cfg(target_os = "windows")]
    worker: Option<std::thread::JoinHandle<()>>,
    #[cfg(target_os = "windows")]
    dispatch: gtk::glib::JoinHandle<()>,
}

impl McpServer {
    #[cfg(target_os = "linux")]
    pub fn start(ui: &Rc<Ui>) -> Option<Self> {
        use gtk::gio;

        let connection = ui.application.dbus_connection()?;

        const MCP_INTERFACE_XML: &str = r#"
        <node>
          <interface name="dev.stickies.Mcp1">
            <method name="Call">
              <arg type="s" name="request" direction="in"/>
              <arg type="s" name="response" direction="out"/>
            </method>
          </interface>
        </node>
        "#;

        let node_info = gio::DBusNodeInfo::for_xml(MCP_INTERFACE_XML).ok()?;
        let iface = node_info.lookup_interface("dev.stickies.Mcp1")?;

        let weak_ui = Rc::downgrade(ui);
        let registration_id = connection
            .register_object("/dev/stickies/Mcp", &iface)
            .method_call(
                move |conn, sender, _path, _iface, method, params, invocation| {
                    if method != "Call" {
                        invocation.return_dbus_error(
                            "org.freedesktop.DBus.Error.UnknownMethod",
                            "Unknown method",
                        );
                        return;
                    }

                    let Some(sender) = sender else {
                        invocation.return_dbus_error(
                            "org.freedesktop.DBus.Error.AccessDenied",
                            "No sender specified",
                        );
                        return;
                    };

                    // Authenticate caller: Verify caller UID matches current process UID via bus credentials
                    let sender_variant = (sender,).to_variant();
                    let uid_call = conn.call_sync(
                        Some("org.freedesktop.DBus"),
                        "/org/freedesktop/DBus",
                        "org.freedesktop.DBus",
                        "GetConnectionUnixUser",
                        Some(&sender_variant),
                        gtk::glib::VariantType::new("(u)").ok().as_deref(),
                        gio::DBusCallFlags::NONE,
                        1_000,
                        None::<&gio::Cancellable>,
                    );

                    let caller_uid = match uid_call {
                        Ok(val) => val.child_value(0).get::<u32>().unwrap_or(u32::MAX),
                        Err(_) => {
                            invocation.return_dbus_error(
                                "org.freedesktop.DBus.Error.AccessDenied",
                                "Could not verify caller bus credentials",
                            );
                            return;
                        }
                    };

                    let my_uid = unsafe { libc::getuid() };
                    if caller_uid != my_uid {
                        invocation.return_dbus_error(
                            "org.freedesktop.DBus.Error.AccessDenied",
                            "Access denied: caller UID mismatch",
                        );
                        return;
                    }

                    let Some(ui) = weak_ui.upgrade() else {
                        invocation.return_dbus_error(
                            "org.freedesktop.DBus.Error.Failed",
                            "Stickies is shutting down",
                        );
                        return;
                    };

                    let request_str = params.child_value(0).get::<String>().unwrap_or_default();
                    let response_str = handle_raw_request(&ui, &request_str);
                    invocation.return_value(Some(&(response_str,).to_variant()));
                },
            )
            .build()
            .ok()?;

        Some(Self {
            registration_id: Some(registration_id),
        })
    }

    #[cfg(target_os = "windows")]
    pub fn start(ui: &Rc<Ui>) -> Option<Self> {
        use crate::mcp_limits::IPC_TIMEOUT;
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            sync::{mpsc, oneshot},
        };
        // Only request data and reply channels cross threads. The future that owns
        // Weak<Ui> is created and polled on the GTK main context.
        type Call = (String, std::time::Instant, oneshot::Sender<String>);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()?;
        let mut pipe = {
            let _entered = runtime.enter();
            match crate::mcp_windows::create_pipe() {
                Ok(pipe) => pipe,
                Err(error) => {
                    eprintln!("Could not start MCP pipe: {error}");
                    return None;
                }
            }
        };
        let (requests, mut receiver) = mpsc::channel::<Call>(1);
        let weak_ui = Rc::downgrade(ui);
        let dispatch = gtk::glib::MainContext::default().spawn_local(async move {
            while let Some((request, deadline, reply)) = receiver.recv().await {
                if reply.is_closed() || std::time::Instant::now() >= deadline {
                    continue;
                }
                let Some(ui) = weak_ui.upgrade() else { break };
                let _ = reply.send(handle_raw_request(&ui, &request));
            }
        });
        let (stop, mut stopping) = oneshot::channel();
        let worker = std::thread::spawn(move || {
            runtime.block_on(async move {
                let serve = async {
                    loop {
                        if pipe.connect().await.is_err() {
                            break;
                        }
                        let exchange = async {
                            let len = pipe.read_u32_le().await? as usize;
                            if len > MAX_PAYLOAD_BYTES {
                                return Err(std::io::Error::other(
                                    "MCP request exceeds payload limit",
                                ));
                            }
                            let mut bytes = vec![0; len];
                            pipe.read_exact(&mut bytes).await?;
                            let request =
                                String::from_utf8(bytes).map_err(std::io::Error::other)?;
                            let (reply, response) = oneshot::channel();
                            requests
                                .send((request, std::time::Instant::now() + IPC_TIMEOUT, reply))
                                .await
                                .map_err(std::io::Error::other)?;
                            let response = response.await.map_err(std::io::Error::other)?;
                            pipe.write_u32_le(response.len() as u32).await?;
                            pipe.write_all(response.as_bytes()).await?;
                            // Disconnect discards unread output. Wait for the companion
                            // to consume it, within the exchange timeout/shutdown select.
                            if pipe.read_u8().await? != crate::mcp_limits::RESPONSE_CONSUMED {
                                return Err(std::io::Error::other(
                                    "Invalid MCP response acknowledgement",
                                ));
                            }
                            Ok::<_, std::io::Error>(())
                        };
                        let _ = tokio::time::timeout(IPC_TIMEOUT, exchange).await;
                        if pipe.disconnect().is_err() {
                            break;
                        }
                    }
                };
                tokio::select! { biased; _ = &mut stopping => {}, _ = serve => {} }
            })
        });
        Some(Self {
            stop_signal: Some(stop),
            worker: Some(worker),
            dispatch,
        })
    }

    pub fn stop(&mut self, ui: &Ui) {
        #[cfg(target_os = "linux")]
        {
            if let Some(id) = self.registration_id.take()
                && let Some(conn) = ui.application.dbus_connection()
            {
                let _ = conn.unregister_object(id);
            }
        }

        #[cfg(target_os = "windows")]
        {
            let _ = ui;
            self.dispatch.abort();
            if let Some(stop) = self.stop_signal.take() {
                let _ = stop.send(());
            }
            // The async select cancels connect/read/write/reply waits on shutdown.
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
}
