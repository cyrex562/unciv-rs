use std::sync::Arc;
use std::collections::HashMap;

/// Server state
pub struct ServerState {
    pub(crate) auth_map: Arc<HashMap<String, String>>,
    pub(crate) auth_enabled: bool,
    pub(crate) identify_operators: bool,
    pub(crate) folder: String,
}