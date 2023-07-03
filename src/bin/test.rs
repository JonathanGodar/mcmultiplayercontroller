use std::net::UdpSocket;

#[tokio::main]
async fn main() {
    let socket = UdpSocket::bind("192.168.1.83:2233").unwrap();
    loop {
        let mut buff = [0; 2046];
        let read_result = socket.recv(&mut buff);
        if let Ok(read) = read_result {
            println!("read {read} bytes. {:?}", &buff[..read]);
        }
    }
}
