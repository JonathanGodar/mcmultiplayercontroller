use tokio::net::UdpSocket;
pub async fn wake_on_lan(mac_addr: MacAddr) -> Result<(), Box<dyn std::error::Error>> {

    let header = [255_u8; 6];
    // let mut mac_repeats = [0_u8; 6 * 16];
    let mac_repeats: [u8; 6 * 16]  = mac_addr.0.into_iter().cycle().take(6 * 16).collect::<Vec<_>>().try_into().unwrap();
    // let payload = [header, mac_repeats].concat();

    let payload: [u8; 102] = {
        let mut payload = [0; 102];
        let (one, two) = payload.split_at_mut(header.len());
        one.copy_from_slice(&header);
        two.copy_from_slice(&mac_repeats);
        payload
    };

    println!("{:x?}", payload);
    println!("{}", payload.len());

    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.send_to(&payload, "0.0.0.0:9").await?;

    Ok(())
}


pub struct MacAddr(pub [u8; 6]);


impl From<&str> for MacAddr {
    fn from(value: &str) -> MacAddr {
        let mut mac_addr = MacAddr([0; 6]);
        value.split(":").into_iter().enumerate().for_each(
            |(idx, part)| {
                if part.len() != 2 {
                    println!("aaaa: {}", part);
                    panic!("shit");
                } else {
                    mac_addr.0[idx] = u8::from_str_radix(&part[0..2], 16).unwrap();
                }
            }
        );
        mac_addr
    }
}
