use std::{future::Future, time::Duration};
use tokio::{
    net::{ToSocketAddrs, UdpSocket},
    select,
};
use tokio_util::sync::CancellationToken;

const MAC_ADDR_SIZE: usize = 6;
const MAGIC_PACKET_HEADER_SIZE: usize = 6;

#[derive(Clone, Copy)]
pub struct MacAddr(pub [u8; MAC_ADDR_SIZE]);

pub async fn send_wol_with_timeout<T: 'static + ToSocketAddrs + Send + Sync + Clone>(
    mac_addr: MacAddr,
    from: T,
    to: T,
    host_activated: impl Future<Output = bool>,
    timeout: std::time::Duration,
) -> Result<(), &'static str> {
    let cancellation_token = CancellationToken::new();
    let cancellation_token_clone = cancellation_token.clone();

    let wol_sender_handle = tokio::spawn(async move {
        let wol_sender = async move {
            loop {
                _ = tokio::time::sleep(Duration::from_millis(100)).await;

                if let Err(err) = send_wol_request(mac_addr, from.clone(), to.clone()).await {
                    return err;
                };
            }
        };

        select! {
            _ = cancellation_token_clone.cancelled() => {
                Ok(())
            },
            err = wol_sender => {
                Err(err)
            },
        }
    });

    let result = select! {
        result = wol_sender_handle => {
            result.unwrap()
        }
        did_host_activate = host_activated => {
            if did_host_activate {
                Ok(())
            } else {
                Err("The host did not activate")
            }
        }
        _ = tokio::time::sleep(timeout) => {
            Err("Timeout exceeded")
        }
    };

    cancellation_token.cancel();
    result
}

pub async fn send_wol_request<T: ToSocketAddrs>(
    mac_addr: MacAddr,
    from: T,
    to: T,
) -> Result<(), &'static str> {
    let mut magic_packet_content = [0; 102];

    // Magic Header
    for i in 0..MAGIC_PACKET_HEADER_SIZE {
        magic_packet_content[i] = 0xFF;
    }

    // Copy mac addr 16 times
    for i in 0..16 {
        for (offset, byte) in mac_addr.0.iter().enumerate() {
            magic_packet_content[MAGIC_PACKET_HEADER_SIZE + i * MAC_ADDR_SIZE + offset] = *byte;
        }
    }

    let socket = UdpSocket::bind(from).await.unwrap();
    let send_result = socket.send_to(&magic_packet_content, to).await;
    assert!(matches!(send_result, Ok(102)));

    Ok(())
}

// pub async fn try_until_with_timeout(
//     mac_addr: MacAddr,
//     condition: impl Fn() -> bool,
//     timeout: Duration,
// ) -> Result<(), ()> {
//     send(mac_addr).await.unwrap();
//     let mut timeout = sleep(timeout).boxed();
//     while !condition() {
//         let retry_in = sleep(Duration::from_secs(1));
//         select! {
//             _ = retry_in => {
//                 send(mac_addr).await.unwrap();
//             }
//             _ = &mut timeout => {
//                 return Err(());
//             }
//         }
//     }

//     Ok(())
// }

// )?;
// pub async fn send(mac_addr: MacAddr) -> Result<(), Box<dyn std::error::Error>> {
// println!("sending packet :D");
// // TODO make async
// wake_on_lan::MagicPacket::new(&mac_addr.0).send_to(
//     env::var("broadcast_address").unwrap(),
//     env::var("local_wol_send_address").unwrap(),
// Ok(())
// }

impl From<&str> for MacAddr {
    fn from(value: &str) -> MacAddr {
        let mut mac_addr = MacAddr([0; 6]);
        value
            .split(":")
            .into_iter()
            .enumerate()
            .for_each(|(idx, part)| {
                if part.len() != 2 {
                    panic!("shit");
                } else {
                    mac_addr.0[idx] = u8::from_str_radix(&part[0..2], 16).unwrap();
                }
            });
        mac_addr
    }
}
