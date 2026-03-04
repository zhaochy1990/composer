use composer_api_types::WsEvent;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<WsEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(16384);
        Self { tx }
    }

    pub fn broadcast(&self, event: WsEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.tx.subscribe()
    }

    pub fn sender(&self) -> broadcast::Sender<WsEvent> {
        self.tx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use composer_api_types::{TaskStatus, Task};
    use uuid::Uuid;

    fn make_task_created_event() -> WsEvent {
        WsEvent::TaskCreated(Task {
            id: Uuid::new_v4(),
            title: "Test".to_string(),
            description: None,
            status: TaskStatus::Backlog,
            priority: 0,
            assigned_agent_id: None,
            position: 1.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    #[tokio::test]
    async fn broadcast_and_subscribe() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.broadcast(make_task_created_event());
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, WsEvent::TaskCreated(_)));
    }

    #[tokio::test]
    async fn multiple_subscribers() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        bus.broadcast(WsEvent::TaskDeleted { task_id: Uuid::nil() });
        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }

    #[test]
    fn no_subscribers_no_panic() {
        let bus = EventBus::new();
        // Should not panic even with no subscribers
        bus.broadcast(make_task_created_event());
    }
}
