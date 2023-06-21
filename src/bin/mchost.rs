fn main() {
    println!("Ran mchost");
}

// use std::env::{args, Args};

// use tokio::net::UnixStream;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {

//     let mut arg = args();
//     println!("Args: {:?}", arg);
//     if let None = arg.next() {
//         print_help();
//         return Ok(());

//     }

//     let command = match arg.next().as_deref() {
//         Some("help") => None,
//         Some("autopoweroff") | Some("apo") => parse_auto_power_off_cmd(arg).ok().map(|v| Commands::AutoPowerOff(v)),
//         _ => {
//             println!("Invalid usage.");
//             None
//         }
//     };

//     if command.is_none() {
//         print_help();
//         return Ok(());
//     }
//     let command = command.unwrap();

//     let conn = tokio::net::UnixStream::connect().await;
//     match conn {
//         Ok(stream) => {
//             match command {
//                 Commands::AutoPowerOff(args) => {
//                     match args {
//                         AutoPowerOffArgs::Status => auto_power_off_status(stream).await,
//                         AutoPowerOffArgs::SetStatus(status) => auto_power_off_set_status(stream, status).await,
//                     }

//                 }
//             }
//         },
//         Err(_) => {
//             println!("Could not connect to {}, make sure it exists, and that this programme has the permissions to read it", mchost_unix_stream::PATH);
//         }

//     }

//     Ok(())
// }

// async fn auto_power_off_status(stream: UnixStream) {
//     // this function should return an err, not do this, but it is just a proto for me so whatevs
//     let err_msg =  format!("Could not write to stream `{}`", mchost_unix_stream::PATH);

//     stream.writable().await.expect(&err_msg);
//     let to_write = b"auto_power_off.status";

//     // TODO Check that all bytes have been written
//     stream.try_write(to_write).expect(&err_msg);

// }

// async fn auto_power_off_set_status(stream: UnixStream, status: bool) {
//     let err_msg =  format!("Could not write to stream `{}`", mchost_unix_stream::PATH);

//     stream.writable().await.expect(&err_msg);
//     let to_write = format!("auto_power_off={}", {
//         if status { "true" } else {"false"}
//     });

//     // TODO Check that all bytes have been written
//     let written = stream.try_write(to_write.as_bytes()).expect(&err_msg);
//     assert_eq!(written, to_write.len());
// }

// fn parse_auto_power_off_cmd(mut args: Args) -> Result<AutoPowerOffArgs, ()> {
//     let args_next = args.next();

//     let r = match args_next.as_deref() {
//         Some("status") => {
//             Ok(AutoPowerOffArgs::Status)
//         },
//         Some("true") => {
//             Ok(AutoPowerOffArgs::SetStatus(true))
//         },
//         Some("false") => {
//             Ok(AutoPowerOffArgs::SetStatus(false))
//         }
//         _ => {
//             Err(())
//         }
//     };

//     return r;
// }

// fn print_help(){
//     println!("Usage: mchost <command> <args>");
//     println!("Commands:");
//     println!("\thelp - prints this help message");
//     println!("\tautopoweroff(apo) [true,false,status] - sets the auto power off status, or gets the currently set status");
// }

// enum Commands{
//     AutoPowerOff(AutoPowerOffArgs)
// }

// enum AutoPowerOffArgs {
//     Status,
//     SetStatus(bool)
// }
