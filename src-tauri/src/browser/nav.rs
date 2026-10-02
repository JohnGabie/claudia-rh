// Navigation history records. The tracker that produces them arrives in Task 3.4; until then
// `spawn_tracker` is a no-op so the browser tools compile and can already wire it in.
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::cdp::CdpClient;
use crate::mcp::tools::browser::ActionSink;

#[derive(Debug, Clone, PartialEq)]
pub struct NavRecord {
    pub url: String,
    pub status: Option<i64>,
    pub redirects: Vec<String>,
    /// "agent" while a browser_* tool is running; "outside-tool" otherwise.
    pub initiator: String,
}

pub fn spawn_tracker(_client: &Arc<CdpClient>, _sink: Arc<dyn ActionSink>, _agent_active: Arc<AtomicBool>) {}
