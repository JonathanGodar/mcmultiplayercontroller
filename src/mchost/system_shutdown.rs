use std::time::Duration;

use tokio::{
    process, select,
    sync::{broadcast, mpsc},
    task::{self, spawn_blocking},
    time::sleep,
};
use tokio_util::sync::CancellationToken;

use super::servers_manager::ServersManagerEvent;

// TODO Find better name
pub async fn user_wants_shutdown(
    should_shutdown: mpsc::Sender<bool>,
    cancellation_token: CancellationToken,
) {
    let mut command = process::Command::new("zenity");
    command.args(&[
        "--warning",
        "--text",
        "\"mchostd will shut down the computer if you do not press OK",
    ]);
    command.kill_on_drop(true);

    let mut process = command.spawn().unwrap();
    select! {
        _ = process.wait() => {
            println!("User interaction recieved");
            should_shutdown.send(false).await.unwrap();
        },
        _ = sleep(Duration::from_secs(3 * 60)) => {
            should_shutdown.send(true).await.unwrap();
        }
        _ = cancellation_token.cancelled() => {
            process.kill().await.unwrap();
        }
    }
}

pub async fn system_shutdown_handler(
    mut evt_reciever: broadcast::Receiver<ServersManagerEvent>,
    system_shutdown_signal: CancellationToken,
    program_shutdown_signal: CancellationToken,
) {
    let mut system_shutdown_check_cancel = CancellationToken::new();
    let (system_shutdown_tx, mut system_shutdown_rx) = mpsc::channel(1);
    let mut system_shutdown_join_handle = None;

    // TODO Refactor
    loop {
        select! {
            _ = program_shutdown_signal.cancelled() => {
                break;
            }
            recv = evt_reciever.recv() => {
                match recv {
                    Ok(evt) => match evt {
                        ServersManagerEvent::AllServersStopped => {
                            if matches!(system_shutdown_join_handle, None) {
                                system_shutdown_join_handle = Some(tokio::spawn(user_wants_shutdown(system_shutdown_tx.clone(), system_shutdown_check_cancel.clone())));
                            }
                        },
                        ServersManagerEvent::ServerStarted(_) => {
                            system_shutdown_tx.send(false).await.unwrap();
                        },
                    },
                    Err(err) => {
                        panic!("Err reding evt {:?}", err);
                    }
                }
            },
            maybe_should_shutdown = system_shutdown_rx.recv() => {
                match maybe_should_shutdown {
                    Some(should_shutdown) => {
                        if should_shutdown {
                            program_shutdown_signal.cancel();
                            system_shutdown_signal.cancel();
                            break;
                        } else {
                            system_shutdown_check_cancel.cancel();
                            if let Some(join_handle) = system_shutdown_join_handle {
                                join_handle.await.unwrap();
                            }
                            system_shutdown_join_handle = None;
                            system_shutdown_check_cancel = CancellationToken::new();
                        }

                    },
                    None => {
                        panic!("Shit all pipes to system_shutdown_rx closed");
                    },
                }
            }
        }
    }
}
