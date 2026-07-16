use actix_web::{HttpRequest, HttpResponse, Responder};
use tube_web::sse_channel;

pub async fn task_progress_stream(req: HttpRequest) -> HttpResponse {
    let (mut sender, receiver) = sse_channel(64);

    let system_status = {
        let st = dsa_service::system::DATA_SYNC_STATUS.lock().unwrap();
        st.to_value()
    };

    let screening_status = {
        let st = dsa_service::screening::SYNC_STATUS.lock().unwrap();
        st.to_value()
    };

    if system_status.get("running").and_then(|v| v.as_bool()).unwrap_or(false) {
        let _ = sender
            .send_data(&serde_json::to_string(&system_status).unwrap_or_default())
            .await;
    }

    if screening_status.get("running").and_then(|v| v.as_bool()).unwrap_or(false) {
        let _ = sender
            .send_data(&serde_json::to_string(&screening_status).unwrap_or_default())
            .await;
    }

    let mut rx = dsa_service::system::TASK_BROADCAST.subscribe();

    actix_rt::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(val) => {
                    if sender.is_closed() {
                        break;
                    }
                    let json_str = serde_json::to_string(&val).unwrap_or_default();
                    if sender.send_data(&json_str).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(_) => {
                    break;
                }
            }
        }
        let _ = sender.done("{}").await;
    });

    receiver.respond_to(&req)
}
