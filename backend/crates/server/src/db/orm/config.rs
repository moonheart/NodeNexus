use once_cell::sync::OnceCell;
use std::time::Duration;
use tracing::warn;

static ORM_CONFIG: OnceCell<OrmConfig> = OnceCell::new();

#[derive(Debug, Clone, Copy)]
pub struct OrmConfig {
    pub slow_query_threshold: Duration,
}

impl OrmConfig {
    pub fn global() -> &'static Self {
        ORM_CONFIG.get().expect("ORM config is not initialized")
    }
}

pub fn init_orm_config(slow_query_threshold_ms: u64) {
    let config = OrmConfig {
        slow_query_threshold: Duration::from_millis(slow_query_threshold_ms),
    };
    if ORM_CONFIG.set(config).is_err() {
        warn!("ORM config has already been initialized");
    }
}