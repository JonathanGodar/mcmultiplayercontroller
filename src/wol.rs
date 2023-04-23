use std::{time::Duration, env};

use futures::FutureExt;
use tokio::{net::UdpSocket, time::sleep, select};

pub async fn try_until_with_timeout(mac_addr: MacAddr, condition: impl Fn() -> bool, timeout: Duration) -> Result<(), ()>{
    send(mac_addr).await.unwrap();
    let mut timeout = sleep(timeout).boxed();
    while!condition() {
        let retry_in = sleep(Duration::from_secs(1));
        select! {
            _ = retry_in => {
                send(mac_addr).await.unwrap();
            }
            _ = &mut timeout => {
                return Err(());
            }
        }

    }

    Ok(())
}

pub async fn send(mac_addr: MacAddr) -> Result<(), Box<dyn std::error::Error>> {
    println!("sending packet :D");
    wake_on_lan::MagicPacket::new(&mac_addr.0).send_to(env::var("broadcast_address").unwrap(), env::var("local_wol_send_addr").unwrap())?;
    Ok(())
    // let header = [255_u8; 6];
    // let mac_repeats: [u8; 6 * 16]  = mac_addr.0.into_iter().cycle().take(6 * 16).collect::<Vec<_>>().try_into().unwrap();

    // let payload: [u8; 102] = {
    //     let mut payload = [0; 102];
    //     let (one, two) = payload.split_at_mut(header.len());
    //     one.copy_from_slice(&header);
    //     two.copy_from_slice(&mac_repeats);
    //     payload
    // };

    // println!("{:x?}", payload);
    // println!("{}", payload.len());

    // let socket = UdpSocket::bind("192.168.1.223:0").await?;
    // println!("Sending wol req");
    // socket.send_to(&payload, "225.225.225.225:9").await?;

    // Ok(())
}


#[derive(Clone, Copy)]
pub struct MacAddr(pub [u8; 6]);

impl From<&str> for MacAddr {
    fn from(value: &str) -> MacAddr {
        let mut mac_addr = MacAddr([0; 6]);
        value.split(":").into_iter().enumerate().for_each(
            |(idx, part)| {
                if part.len() != 2 {
                    panic!("shit");
                } else {
                    mac_addr.0[idx] = u8::from_str_radix(&part[0..2], 16).unwrap();
                }
            }
        );
        mac_addr
    }
}
