pub const MCP_IPC_VERSION: u32 = 1;
pub const MAX_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
// Sent only after the companion has read the complete length-prefixed response.
#[cfg(target_os = "windows")]
pub const RESPONSE_CONSUMED: u8 = 0x06;
#[cfg(target_os = "windows")]
pub const IPC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
