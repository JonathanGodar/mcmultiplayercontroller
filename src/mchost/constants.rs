use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;

pub const ORCHESTRATOR_ENDPOINT_ENV_NAME: &'static str = "orchestrator_endpoint";

pub static SERVER_INSTALLATIONS_PATH: Lazy<PathBuf> = Lazy::new(|| {
    dirs::home_dir()
        .unwrap()
        .join(".mchostd/server_installations/")
});

pub static SERVERS_PATH: Lazy<PathBuf> =
    Lazy::new(|| dirs::home_dir().unwrap().join(".mchostd/servers/"));
