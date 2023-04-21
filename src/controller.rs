use std::{time::Duration, pin::Pin, cell::RefCell, sync::Arc};
use futures::{Stream, future::select};

use controllerp::basics_server::Basics;
use tokio::{sync::{mpsc::{self, Receiver}, watch, Mutex}, select};
use tokio_stream::{wrappers::{ReceiverStream, BroadcastStream}, StreamExt};
use tonic::{Request, Response, Status, transport::Server};

use crate::{controller::controllerp::Command, MCHostStatus};

use self::controllerp::{HelloRequest, HelloReply, basics_server::BasicsServer, ControllerCommands};

pub mod controllerp {
    tonic::include_proto!("controllerp");
}

#[derive(Debug)]
pub struct MyBasics {
    status_updater: Arc<tokio::sync::Mutex<watch::Sender<MCHostStatus>>>,
    command_rx: Arc<Mutex<Receiver<controllerp::Command>>>,
}


#[tonic::async_trait]
impl Basics for MyBasics {
    type OnHostStartupStream = Pin<Box<dyn Stream<Item = Result<ControllerCommands, Status>> + Send>>;

    async fn say_hello(
        &self,
        request: Request<HelloRequest>
    ) -> Result<Response<HelloReply>, Status>{
        Ok(Response::new(HelloReply {
            message: format!("Hello world :D, and hello {}", request.get_ref().name),
        }))
    }

    async fn on_host_startup(&self, _: Request<()>) -> Result<Response<Self::OnHostStartupStream>, Status> {

        println!("The host connected");
        self.status_updater.lock().await.send(MCHostStatus::Online).unwrap();
        let (tx, rx) = mpsc::channel::<Result<ControllerCommands, Status>>(128);
        {
            let commands = self.command_rx.clone();
            let status_updater = self.status_updater.clone();
            tokio::spawn(async move {
                let mut id_counter = 0;

                // TODO make the guard time out
                let mut guard = commands.lock().await;
                let mut heart_beat = Box::pin(tokio_stream::iter(std::iter::repeat(Command::HeartBeat)).throttle(Duration::from_secs(2)));

                loop {
                    let command = select! {
                        res = guard.recv() => {
                            res
                        },
                        res = heart_beat.next() => {
                            res
                        }
                    };

                    if command.is_none() {
                        break;
                    }
                    let command = command.unwrap();
                    match tx.send(Ok(ControllerCommands { id: id_counter, command: command.into() })).await {
                        Ok(_) => {
                            println!("Sent a command to host");
                        }, 
                        Err(_) => {
                            println!("Could not send to client");
                            break;
                        }

                    }
                    id_counter += 1;

                }

                status_updater.lock().await.send(MCHostStatus::Offline).unwrap();
                // let commands = BroadcastStream::new(self.command_rx).merge(
                //     tokio_stream::iter(std::iter::repeat(Command::HeartBeat)).throttle(Duration::from_secs(6))
                // );


                // let heart_beat = std::iter::repeat(HeartBea)
                //     t

                // loop {
                //     select! {
                //         result => {

                //         }


                //     }
                // }
                // while let Some(command) = command_reciever.lock().await.recv().await {
                //     match tx.send(Ok(ControllerCommands {
                //         id: id_counter,
                //         command: command.into(),
                //     })).await {
                //         Ok(()) => {
                //             println!("Sent a command to host");
                //         },
                //         Err(err) => {
                //             println!("Could not send command to host: {}", err);
                //             break;
                //         }
                //     };
                //     id_counter += 1;
                // }
                // println!("The host disconnected");
            });
        }

        let output_stream = ReceiverStream::new(rx.into());
        Ok(Response::new(Box::pin(output_stream)))
    }
}


pub async fn start_tonic(command_queue: mpsc::Receiver<controllerp::Command>, status_updater: watch::Sender<MCHostStatus>) -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;

    let service = MyBasics {
        command_rx: Arc::new(command_queue.into()),
        status_updater: Arc::new(status_updater.into()),
    };

    Server::builder().tcp_keepalive(Some(Duration::from_secs(59))).add_service(BasicsServer::new(service)).serve(addr).await?;
    Ok(())
}

// pub async fn start_tonic() -> Result<(), Box<dyn std::error::Error>> {
//     let addr = "[::1]:50051".parse()?;
// }
