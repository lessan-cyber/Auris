use crate::AppState;
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message::Text, WebSocket},
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct WsMessage {
    pub track_id: Uuid,
    pub status: String,
    pub progress: Option<u8>,
    pub message: Option<String>,
}

pub type WsClient = Arc<DashMap<Uuid, mpsc::UnboundedSender<WsMessage>>>;

/// WebSocket handler for job status
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // Split socket into sender and receiver
    let (mut socket_sender, mut socket_receiver) = socket.split();

    // First message from client must be the track_id they want to monitor
    let track_id = match socket_receiver.next().await {
        Some(Ok(Text(text))) => match Uuid::parse_str(text.trim()) {
            Ok(id) => id,
            Err(_) => {
                let _ = socket_sender
                    .send(Text(
                        json!({"error": "Invalid track_id UUID"}).to_string().into(),
                    ))
                    .await;
                return;
            }
        },
        _ => {
            let _ = socket_sender
                .send(Text(
                    json!({"error": "First message must be track_id UUID"})
                        .to_string()
                        .into(),
                ))
                .await;
            return;
        }
    };

    // Create channel for this client
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Register client
    state.ws_clients.insert(track_id, tx);

    // Send confirmation
    let _ = socket_sender
        .send(Text(
            json!({
                "track_id": track_id,
                "status": "subscribed",
                "message": "Monitoring job status"
            })
            .to_string()
            .into(),
        ))
        .await;

    // Forward messages from channel to WebSocket
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let payload = json!({
                "track_id": msg.track_id,
                "status": msg.status,
                "progress": msg.progress,
                "message": msg.message,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
            .to_string();

            if socket_sender.send(Text(payload.into())).await.is_err() {
                break; // Client disconnected
            }
        }
    });

    // Keep connection alive until client disconnects
    while let Some(Ok(msg)) = socket_receiver.next().await {
        if matches!(msg, axum::extract::ws::Message::Close(_)) {
            break;
        }
    }

    // Cleanup
    state.ws_clients.remove(&track_id);
    forward_task.abort();
}
