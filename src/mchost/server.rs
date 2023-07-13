use std::{
    io::Error,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{read_to_string, File},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{self, ChildStdout},
    select,
    sync::mpsc,
    task::JoinHandle,
    time::sleep,
};
use tokio_util::sync::CancellationToken;

use super::{
    constants::SERVERS_PATH,
    server_installation::{ServerInstallation, ServerInstallationId},
    server_installations::{self, ServerInstallations},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Server {
    pub id: u64,
    pub installation_id: ServerInstallationId,
    name: String,
}

impl Server {
    fn get_path_for(id: u64) -> PathBuf {
        SERVERS_PATH.join(format!("{}/", id))
    }

    fn get_path(&self) -> PathBuf {
        Server::get_path_for(self.id)
    }

    fn get_metadata_path_for(id: u64) -> PathBuf {
        Self::get_path_for(id).join("mchostd_metadata.json")
    }

    fn get_metadata_path(&self) -> PathBuf {
        Server::get_metadata_path_for(self.id)
    }

    fn get_eula_path(&self) -> PathBuf {
        self.get_path().join("eula.txt")
    }

    pub async fn create(
        id: u64,
        name: String,
        installation: ServerInstallationId,
    ) -> Result<Self, &'static str> {
        let server = Self {
            id,
            installation_id: installation,
            name,
        };

        if let Ok(true) = tokio::fs::try_exists(Self::get_metadata_path_for(id)).await {
            return Err("Could not create server <- Server already exists");
        }

        tokio::fs::create_dir_all(server.get_path()).await.unwrap();
        let mut eula_file = File::create(server.get_eula_path()).await.unwrap();
        eula_file.write_all(b"eula=true").await.unwrap();

        server.write_metadata().await;
        Ok(server)
    }

    pub async fn load(id: u64) -> Result<Self, &'static str> {
        let server_json = read_to_string(Server::get_metadata_path_for(id))
            .await
            .map_err(|_| "Could not read the file for the server.")?;

        let server = serde_json::from_str(&server_json).map_err(|_| "Could not parse json")?;

        Ok(server)
    }

    pub async fn write_metadata(&self) {
        let metadata = serde_json::to_string(self).unwrap();
        let mut metadata_file = File::create(self.get_metadata_path()).await.unwrap();
        metadata_file.write_all(metadata.as_bytes()).await.unwrap();
    }

    async fn handle_server_stdout(
        mut stdout: Lines<BufReader<ChildStdout>>,
        player_count_tx: mpsc::Sender<usize>,
    ) {
        let mut player_count = 0;
        loop {
            match stdout.next_line().await {
                Ok(Some(line)) => {
                    handle_server_stdout_line(line, &mut player_count, &player_count_tx).await;
                }
                Ok(None) => {
                    println!("Stdout of server closed");
                    break;
                }
                Err(err) => {
                    println!("Error reading stdout of server: {:?}", err);
                    break;
                }
            }
        }
    }

    async fn wait_on_empty_server(mut player_count_rx: mpsc::Receiver<usize>) {
        const MAX_SERVER_EMPTY_TIME: Duration = Duration::from_secs(30);
        let mut timeout_active = true;

        loop {
            select! {
                _ = async { sleep(MAX_SERVER_EMPTY_TIME).await }, if timeout_active => {
                    break;
                },
                maybe_player_count = player_count_rx.recv() =>  {
                    match maybe_player_count {
                        Some(player_count) => {
                            timeout_active = player_count == 0;
                        },
                        None => {
                            println!("Server.wait_on_empty_server exited because of player_count pipe closing");
                            break;
                        },
                    }
                }
            }
        }
    }

    pub async fn run(
        &mut self,
        server_installations: Arc<ServerInstallations>,
        cancellation_token: CancellationToken,
    ) -> Result<(), &str> {
        let installation = server_installations
            .get(self.installation_id.clone())
            .ok_or("The correct server installation is not available")?;

        println!("Server {}: Starting server", self.id);
        let mut server_process_handle = process::Command::new("java")
            .current_dir(self.get_path())
            .arg("-jar")
            .arg(installation.jar_path.to_str().unwrap())
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();

        let stdout = server_process_handle
            .stdout
            .take()
            .expect("Could not connect to minecraft server std out");

        let stdout_reader = BufReader::new(stdout).lines();
        let mut stdin = server_process_handle
            .stdin
            .take()
            .expect("Could not connect to minecraft server std in");

        let (player_count_tx, player_count_rx) = mpsc::channel(2);

        select! {
            _ = Self::handle_server_stdout(stdout_reader,player_count_tx) => {
                println!("Server {}: Shutting down because of stdout stop", self.id);
            },
            _ = Self::wait_on_empty_server(player_count_rx) => {
                println!("Server {}: Shutting down because of player count", self.id);
            },
            _ = cancellation_token.cancelled() => {
                println!("Server {}: Shutting down because of shutdown signal", self.id);
            }
        }

        println!("Server {}: Sending stop command", self.id);
        _ = stdin.write("stop\r\n".as_bytes()).await;
        _ = stdin.flush().await;

        println!("Server {}: Waiting for server process to exit", self.id);
        server_process_handle.wait().await.unwrap();
        println!("Server {}: Shutdown complete", self.id);
        Ok(())
    }
}

static JOIN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\[\d{2}:\d{2}:\d{2} INFO\]:\s+(\w+) joined the game$").unwrap());

static LEAVE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\[\d{2}:\d{2}:\d{2} INFO\]:\s+(\w+) left the game$").unwrap());

async fn handle_server_stdout_line(
    line: String,
    player_count: &mut usize,
    player_count_tx: &mpsc::Sender<usize>,
) {
    if JOIN_RE.is_match(&line) {
        *player_count += 1;
        player_count_tx.send(*player_count).await.unwrap();
    }

    if LEAVE_RE.is_match(&line) {
        *player_count -= 1;
        player_count_tx.send(*player_count).await.unwrap();
    }
}
