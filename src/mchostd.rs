use std::{env, io::{Stdout, BufWriter, Write}, process::Stdio, time::Duration, alloc::System, sync::Arc};

use prost::encoding::bool;
use regex::Regex;
use tokio_util::sync::CancellationToken;
use crate::controllerp::{basics_client::BasicsClient, HelloRequest};
use controllerp::{Command, ControllerCommands};
use tokio::{process, io::{BufReader, AsyncBufReadExt, AsyncWrite, AsyncWriteExt}, select, net::UnixListener};
use lazy_static::lazy_static;
use std::sync::Mutex;
use tokio::sync::watch;
use tokio_stream::StreamExt;

mod mchost_unix_stream;

pub mod controllerp {
    tonic::include_proto!("controllerp");
}

lazy_static! {
    static ref SERVER_RUNNING: Mutex<bool> = Mutex::new(false);
}

async fn auto_power_off_watcher(shutdown: CancellationToken, tx: Arc<Mutex<watch::Sender<bool>>>){
    // TODO Move to end of function and make the application gracefully exit.
    if let Err(err) = tokio::fs::remove_file(mchost_unix_stream::PATH).await {
        println!("Could not remove {} because {}", mchost_unix_stream::PATH, err);
    }

    let listener = UnixListener::bind(mchost_unix_stream::PATH).unwrap();
    loop {
        select! {
            accepted = listener.accept() =>  {
                match accepted {
                    Ok((stream, _)) => {
                        // TODO Make it able to recieve shutdown signal here as well
                        stream.readable().await.unwrap();
                        let mut buf = vec![];
                        stream.try_read_buf(&mut buf).unwrap();
                        match std::str::from_utf8(&buf).unwrap() {
                            "auto_power_off=true" =>  {
                                tx.lock().unwrap().send(true).unwrap();
                            }
                            "auto_power_off=false" => {
                                tx.lock().unwrap().send(false).unwrap();
                            },
                            _ => {
                                println!("received unknown command");
                            }
                        }
                    },
                    Err(why)  => {
                        println!("Could not accept connection, {why}");
                        break;
                    }
                }
            },
            _ = shutdown.cancelled() => {
                break;
            }
        }
    }
    _ = tokio::fs::remove_file(mchost_unix_stream::PATH).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    _ = dotenvy::dotenv();
    let shutdown = CancellationToken::new();

    let (auto_power_off_tx, auto_power_off_rx) = watch::channel(false);
    let auto_power_off_tx = Arc::new(Mutex::new(auto_power_off_tx));
    let handle = tokio::spawn(auto_power_off_watcher(shutdown.clone(), auto_power_off_tx.clone()));

    // TODO tokio::spawn(handle_command);
    let mut client = BasicsClient::connect(std::env::var("controller_address").expect("controller_address env var must be set")).await?;
    let response  = client.on_host_startup(()).await;
    match response {
        Ok(response) => {
            let mut stream = response.into_inner();
            loop {
                select! {
                    _ = tokio::signal::ctrl_c() => {
                        break;
                    },
                    strm_result = stream.next() => {
                        if let Some(itm) = strm_result {
                            match itm {
                                Ok(command) => {
                                    if command.id < 3 && command.command() == Command::StartServer {
                                        auto_power_off_tx.lock().unwrap().send(true).unwrap();
                                    }
                                    handle_command(command, auto_power_off_rx.clone()).await;
                                }, 
                                Err(_) => {
                                    println!("Error reading response");
                                }
                            }
                        }
                    }

                }
            }
        },
        Err(err) => {
            println!("Could not connect to stream {}", err);
        }
    }

    shutdown.cancel();
    handle.await.unwrap();

    Ok(())
}


async fn handle_start_server(auto_shutdown: watch::Receiver<bool>) {

    match SERVER_RUNNING.lock() {
        Ok(mut lock) => {
            if *lock {
                println!("Tried to start the server while it was running");
                return;
            }

            *lock = true;
        }
        Err(_) => {
        }
    }

    let mut child = process::Command::new("java").current_dir("/home/jonathan/minecraft/1.19.4/").arg("-jar").arg("server.jar")
        .stdout(Stdio::piped()).stdin(Stdio::piped()).kill_on_drop(true).spawn().unwrap();

    let stdout = child.stdout.take().expect("Mc process did not have a stdout handle"); 
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stdin = child.stdin.take().expect("Mc process did not have a stdin handle");

    let mut player_count = 0;

    let player_timeout_duration = Duration::from_secs(30);
    let mut player_timeout = Some(Box::pin(tokio::time::sleep(player_timeout_duration)));
    loop {
        if let Some(timeout) = player_timeout.as_mut() {
            println!("Timeout active");
            select! {
                line_res = stdout_reader.next_line() => {
                    if let Ok(Some(line)) = line_res {
                        update_player_count(&line, &mut player_count);
                        if player_count != 0 {
                            player_timeout = None;
                        }
                    } else {
                        println!("Exiting because childprocess closed. (In timeoutbranch)");
                        break;
                    }
                }
                _ = timeout => {
                    println!("Exiting because of timeout");
                    break;
                }
            }
        } else {
            println!("Timeout not active");
            if let Ok(Some(line)) = stdout_reader.next_line().await {
                if update_player_count(&line, &mut player_count) && player_count == 0 {
                    player_timeout = Some(Box::pin(tokio::time::sleep(player_timeout_duration)));
                }
            } else {
                println!("Exiting because childprocess closed.");
                break;
            }
        }
    }


    println!("Stopping server");
    _ = stdin.write("stop\r\n".as_bytes()).await;
    // _ = stdin.flush().await;

    println!("Awainting child");
    child.wait().await.unwrap();


    println!("shut down?");
    *SERVER_RUNNING.lock().unwrap() = false;
    if *auto_shutdown.borrow() {
        println!("Would have shutdown!");
        // let output = process::Command::new("systemctl shutdown").output().await.unwrap();
        // std::io::stdout().write(&output.stdout).unwrap();
    }
    println!("Done :D");
}

fn update_player_count(mc_output_line: &String, player_count: &mut u32) -> bool {
    // TODO move to lazy static
    let join_re = Regex::new(r"^\[\d{2}:\d{2}:\d{2} INFO\]:\s+(\w+) joined the game$").unwrap();
    let leave_re = Regex::new(r"^\[\d{2}:\d{2}:\d{2} INFO\]:\s+(\w+) left the game$").unwrap();

    if join_re.is_match(&mc_output_line) {
        *player_count += 1;
        return true;
    } 

    if leave_re.is_match(&mc_output_line) {
        *player_count -= 1;
        return true;
    }

    return false;
}


async fn handle_command(controller_command: ControllerCommands, auto_shutdown: watch::Receiver<bool>) {
    match Command::from_i32(controller_command.command).unwrap() {
        Command::StartServer => {
            handle_start_server(auto_shutdown).await;
        }
        Command::HeartBeat => {}
    }
}
