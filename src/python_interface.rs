use serde::{Deserialize, Serialize};



#[derive(Clone, Serialize)]
pub struct PythonTask {
    pub identifier: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub role: String, // "user" or "assistant"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>, // Tools for FC mode (function calling)
}

#[derive(Clone, Deserialize)]
pub struct PythonResponse {
    pub identifier: String,
    pub response: String,
}
