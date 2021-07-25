use tokio::net::TcpStream;
use tokio::sync::mpsc;

use tokio_tungstenite::tungstenite;

use futures::sink::SinkExt;
use futures::stream::StreamExt;

type WebSocketStream = tokio_tungstenite::WebSocketStream<TcpStream>;

pub enum Message {
    WsMessage(tungstenite::Message),
    Terminate
}

struct WebSocketActor {
    ws_stream: WebSocketStream,
    rx: mpsc::Receiver<Message>,
    tx: mpsc::Sender<tungstenite::Message>
}

impl WebSocketActor {
    async fn run(mut self) {
        loop {
            tokio::select! {
                opt = self.ws_stream.next() => {
                    let ws_msg = match opt {
                        Some(Ok(v)) => v,
                        _ => return
                    };

                    let _ = self.tx.send(ws_msg).await;
                },
                opt = self.rx.recv() => match opt {
                    Some(Message::WsMessage(msg)) => {
                        if let Err(_) = self.ws_stream.send(msg).await {
                            return;
                        }
                    },
                    Some(Message::Terminate) => {
                        return;
                    },
                    None => return
                }
            }
        }
    }
}

pub fn start(
    ws_stream: WebSocketStream
) -> (mpsc::Sender<Message>, mpsc::Receiver<tungstenite::Message>) {
    let (tx_user, rx_actor) = mpsc::channel(8);
    let (tx_actor, rx_user) = mpsc::channel(8);

    let ws_actor = WebSocketActor {
        ws_stream,
        rx: rx_actor,
        tx: tx_actor
    };

    tokio::spawn(ws_actor.run());

    (tx_user, rx_user)
}
