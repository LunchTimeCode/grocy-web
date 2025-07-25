use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::database::{self, DBClient, items::Item};

#[derive(Debug)]
pub enum LlmError {
    Request(String),
    Auth(String),
    Parse(String),
}

#[derive(Debug, Serialize)]
pub struct Prompt {
    prompt: String,
}

#[derive(Debug, Deserialize)]
pub struct TaskList {
    list: Vec<String>,
}

pub async fn simple_item_response(
    nest_api: &str,
    nest_api_key: &str,
    user_message: &str,
    user_id: String,
    db_client: &DBClient,
) -> Result<String, LlmError> {
    let client = Client::new();

    let with_sys = format!(
        "{}{}",
        "Create only grocery items out of this, ignore everything else: ", user_message
    );

    let prompt = Prompt {
        prompt: with_sys.to_string(),
    };

    let full_url = format!("{}{}", nest_api, "/api/task");

    let masked = nest_api_key.to_string().split_off(10);
    info!("calling: {full_url} with key: {masked} ");

    let response = client
        .post(full_url)
        .header("api-key", nest_api_key)
        .json(&prompt)
        .send()
        .await
        .map_err(|e| LlmError::Request(format!("Failed to send request: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(LlmError::Auth(format!(
            "API returned status {status}: {error_text}"
        )));
    }

    let task_list: TaskList = response
        .json()
        .await
        .map_err(|e| LlmError::Parse(format!("Failed to parse response: {e}")))?;

    let items: Vec<Item> = task_list
        .list
        .iter()
        .map(|t| Item {
            owner_id: user_id.clone(),
            id: None,
            task: t.clone(),
            completed: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .collect();

    database::items::create_items(db_client, items).await;

    let tasks_string = task_list.list.join("\n");

    let answer = format!("Created {tasks_string}");

    Ok(answer)
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub content: String,
}

pub async fn simple_chat_response(
    nest_api: &str,
    nest_api_key: &str,
    user_message: &str,
) -> Result<String, LlmError> {
    let client = Client::new();

    let with_insctructions = format!(
        "
        Only answer in commonmark markdown format.
        You are Rezi a helpful assistant for recipes, cooking, ingredients and groceries.


        this is the message from the user: {user_message}

        "
    );

    let prompt = Prompt {
        prompt: with_insctructions,
    };

    let full_url = format!("{}{}", nest_api, "/api/chat");

    let masked = nest_api_key.to_string().split_off(10);
    info!("calling: {full_url} with key: {masked} ");

    let response = client
        .post(full_url)
        .header("api-key", nest_api_key)
        .json(&prompt)
        .send()
        .await
        .map_err(|e| LlmError::Request(format!("Failed to send request: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(LlmError::Auth(format!(
            "API returned status {status}: {error_text}"
        )));
    }

    let chat_response: ChatResponse = response
        .json()
        .await
        .map_err(|e| LlmError::Parse(format!("Failed to parse response: {e}")))?;

    Ok(chat_response.content)
}
