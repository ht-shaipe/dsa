use tube::{Result, Value};
use tube_web::RequestParameter;

pub async fn distribute(param: &RequestParameter) -> Result<Value> {
    let service = dsa_service::BotDispatcher::new();
    let raw_message = param.value.get("message").and_then(|v| v.as_str()).unwrap_or_default();
    let platform = param.value.get("platform").and_then(|v| v.as_str()).unwrap_or_default();
    let user_id = param.value.get("user_id").and_then(|v| v.as_str()).unwrap_or_default();
    let chat_id = param.value.get("chat_id").and_then(|v| v.as_str()).unwrap_or_default();
    let context = dsa_service::bot::dispatcher::BotContext {
        platform: platform.to_string(),
        user_id: user_id.to_string(),
        chat_id: chat_id.to_string(),
        is_admin: false,
    };
    service
        .dispatch(&raw_message, &context)
        .await
        .map(|text| value!({"response": text}))
        .map_err(|e| { let msg = format!("{}", e); error!("{}", &msg); tube::Error::from(msg) })
}
