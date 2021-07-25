use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::IpAddr;

use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite;

type AnyError = Box<dyn std::error::Error + Send + Sync>;
type ActorHandle = mpsc::Sender<actor::Message>;

mod actor;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let map: HashMap<IpAddr, ActorHandle> = HashMap::new();
    let map = Arc::new(Mutex::new(map));

    let listener = TcpListener::bind("127.0.0.1:9001").await?;

    loop {
        let (stream, addr) = listener.accept().await?;

        let map = map.clone();
        tokio::spawn(async move {
            let ws_stream = match accept_async(stream).await {
                Ok(v) => v,
                Err(_) => return
            };

            let (handle, mut rx) = actor::start(ws_stream);

            let old_handle = map.lock().unwrap().insert(addr.ip(), handle.clone());

            match old_handle {
                Some(old_handle) => {
                    let _ = old_handle.send(actor::Message::Terminate).await;
                },
                None => ()
            }

            tokio::spawn(async move {
                loop {
                    let msg = tungstenite::Message::Text("I am alive.".to_owned());
                    let msg = actor::Message::WsMessage(msg);

                    if let Err(_) = handle.send(msg).await {
                        break;
                    }

                    sleep(Duration::from_secs(1)).await;
                }
            });

            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    println!("Message received: {:?}", msg);
                }
            });
        });
    }
}
