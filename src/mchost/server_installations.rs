use super::{
    constants::SERVER_INSTALLATIONS_PATH,
    server_installation::{ServerInstallation, ServerInstallationId},
};
use tokio_stream::{wrappers::ReadDirStream, StreamExt};

#[derive(Debug)]
pub struct ServerInstallations {
    installations: Vec<ServerInstallation>,
}

impl ServerInstallations {
    pub async fn load() -> Self {
        println!(
            "Createing a thing in {:?}",
            SERVER_INSTALLATIONS_PATH.as_path()
        );
        tokio::fs::create_dir_all(SERVER_INSTALLATIONS_PATH.as_path())
            .await
            .unwrap();

        let installations = ReadDirStream::new(
            tokio::fs::read_dir(SERVER_INSTALLATIONS_PATH.as_path())
                .await
                .unwrap(),
        );

        let installations: Vec<_> = installations
            .filter_map(|dir_entry| dir_entry.ok().map(|entry| ServerInstallation::new(entry)))
            .then(|f| f)
            .filter_map(|val| val)
            .collect()
            .await;

        Self { installations }
    }

    pub fn get(&self, id: ServerInstallationId) -> Option<&ServerInstallation> {
        self.installations
            .iter()
            .find(|installation| installation.id == id)
    }

    pub fn get_latest_installation(&self) -> Option<&ServerInstallation> {
        self.installations.get(0)
    }
}
