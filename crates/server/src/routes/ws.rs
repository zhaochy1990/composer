use crate::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use composer_api_types::{WsCommand, WsEvent};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Extract a session_id from a WsEvent, if applicable.
fn extract_session_id(event: &WsEvent) -> Option<Uuid> {
    match event {
        WsEvent::SessionStarted { session_id, .. }
        | WsEvent::SessionCompleted { session_id, .. }
        | WsEvent::SessionFailed { session_id, .. }
        | WsEvent::SessionPaused { session_id }
        | WsEvent::SessionOutput { session_id, .. }
        | WsEvent::SessionResumeIdCaptured { session_id, .. }
        | WsEvent::UserQuestionRequested { session_id, .. }
        | WsEvent::UserQuestionAnswered { session_id }
        | WsEvent::PlanCompleted { session_id, .. } => Some(*session_id),
        _ => None,
    }
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut event_rx = state.event_bus.subscribe();

    // Per-connection subscription set. Empty = receive all events.
    let subscriptions = Arc::new(tokio::sync::Mutex::new(HashSet::<Uuid>::new()));
    let sub_clone = subscriptions.clone();

    // Forward events to WebSocket client
    let mut send_task = tokio::spawn(async move {
        loop {
            let event = match event_rx.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WebSocket client lagged, dropped {} events", n);
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };

            // Skip internal-only events that clients don't need.
            // UserQuestionRequested with plan_content=None is the raw event from the
            // process manager; the session service re-emits an enriched version with
            // plan_content populated.
            if matches!(event, WsEvent::SessionResumeIdCaptured { .. }) {
                continue;
            }
            if let WsEvent::UserQuestionRequested {
                ref plan_content, ..
            } = event
            {
                if plan_content.is_none() {
                    continue;
                }
            }
            // PlanCompleted: executor now includes plan content directly,
            // no filtering needed (content is read eagerly at ExitPlanMode detection).

            // Filter session events by subscription set
            let subs = sub_clone.lock().await;
            if !subs.is_empty() {
                if let Some(sid) = extract_session_id(&event) {
                    if !subs.contains(&sid) {
                        continue;
                    }
                }
            }
            drop(subs);

            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Read from WebSocket client (for commands like subscribe/unsubscribe)
    let sub_clone2 = subscriptions.clone();
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<WsCommand>(&text) {
                        Ok(WsCommand::SubscribeSession { session_id }) => {
                            sub_clone2.lock().await.insert(session_id);
                        }
                        Ok(WsCommand::UnsubscribeSession { session_id }) => {
                            sub_clone2.lock().await.remove(&session_id);
                        }
                        Ok(WsCommand::SendInput {
                            session_id,
                            message,
                        }) => {
                            let id_str = session_id.to_string();
                            if let Err(e) = state_clone
                                .services
                                .sessions
                                .send_input(&id_str, message)
                                .await
                            {
                                tracing::warn!("Failed to send input to session {}: {}", id_str, e);
                            }
                        }
                        Ok(WsCommand::AnswerUserQuestion {
                            session_id,
                            request_id,
                            answers,
                        }) => {
                            let id_str = session_id.to_string();
                            if let Err(e) = state_clone
                                .services
                                .sessions
                                .answer_question(&id_str, request_id, answers)
                                .await
                            {
                                tracing::warn!(
                                    "Failed to answer question for session {}: {}",
                                    id_str,
                                    e
                                );
                            }
                        }
                        Ok(WsCommand::Ping) => {
                            // No-op — just keep the connection alive
                        }
                        Err(_) => {
                            tracing::debug!("Unknown WS command: {}", text);
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Fix #22: Use &mut references in select! and abort the other task on completion
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        },
        _ = &mut recv_task => {
            send_task.abort();
        },
    }
}
