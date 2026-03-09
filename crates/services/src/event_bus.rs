use composer_api_types::WsEvent;
use tokio::sync::{broadcast, mpsc};

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<WsEvent>,
    persist_tx: mpsc::UnboundedSender<WsEvent>,
}

impl EventBus {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<WsEvent>) {
        let (tx, _) = broadcast::channel(16384);
        let (persist_tx, persist_rx) = mpsc::unbounded_channel();
        (Self { tx, persist_tx }, persist_rx)
    }

    pub fn broadcast(&self, event: WsEvent) {
        let _ = self.tx.send(event.clone());
        let _ = self.persist_tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.tx.subscribe()
    }

    pub fn sender(&self) -> broadcast::Sender<WsEvent> {
        self.tx.clone()
    }
}
