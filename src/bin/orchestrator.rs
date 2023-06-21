use mcmultiplayercontroller::orchestrator::{
    discord_bot::start_discord_bot, grpc_server::grpc_host_manager::GrpcHostManager,
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    start_discord_bot(GrpcHostManager::new()).await;
}
