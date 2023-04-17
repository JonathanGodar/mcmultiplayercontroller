use wol::MacAddr;

mod wol;

#[tokio::main]
async fn main() {
    wol::wake_on_lan(
            MacAddr::from(
            "aa:bb:cc:dd:ee:ee"
            
        )).await.unwrap();
}
