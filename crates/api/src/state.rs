//! Shared application state.

use std::sync::Arc;

use crate::config::Config;
use crate::store::Store;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub config: Arc<Config>,
}
