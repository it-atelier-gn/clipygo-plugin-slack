use base64::Engine;
use reqwest::blocking::{Client, Response};
use serde::Deserialize;

const API_BASE: &str = "https://slack.com/api";

fn client(token: &str) -> Client {
    Client::builder()
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {token}").parse().unwrap(),
            );
            headers
        })
        .build()
        .expect("Failed to build HTTP client")
}

#[derive(Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
    #[serde(default)]
    pub is_member: bool,
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
    pub is_archived: bool,
}

#[derive(Deserialize, Default)]
struct ResponseMetadata {
    #[serde(default)]
    next_cursor: String,
}

#[derive(Deserialize)]
struct ConversationsList {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    channels: Vec<Channel>,
    #[serde(default)]
    response_metadata: ResponseMetadata,
}

#[derive(Deserialize)]
struct ApiResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct UploadUrlResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    upload_url: String,
    #[serde(default)]
    file_id: String,
}

fn api_error(error: Option<String>) -> String {
    format!(
        "Slack API error: {}",
        error.unwrap_or_else(|| "unknown".to_string())
    )
}

fn parse_ok(resp: Response) -> Result<(), String> {
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Slack API error ({status}): {body}"));
    }
    let data: ApiResponse = resp.json().map_err(|e| format!("Bad response: {e}"))?;
    if data.ok {
        Ok(())
    } else {
        Err(api_error(data.error))
    }
}

pub fn fetch_channels(token: &str) -> Result<Vec<Channel>, String> {
    let mut all = Vec::new();
    let mut cursor = String::new();

    loop {
        let mut req = client(token)
            .get(format!("{API_BASE}/conversations.list"))
            .query(&[
                ("types", "public_channel,private_channel"),
                ("exclude_archived", "true"),
                ("limit", "1000"),
            ]);
        if !cursor.is_empty() {
            req = req.query(&[("cursor", cursor.as_str())]);
        }

        let resp = req.send().map_err(|e| format!("Request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Slack API error ({status}): {body}"));
        }

        let data: ConversationsList = resp.json().map_err(|e| format!("Bad response: {e}"))?;
        if !data.ok {
            return Err(api_error(data.error));
        }

        all.extend(data.channels);
        cursor = data.response_metadata.next_cursor;
        if cursor.is_empty() {
            break;
        }
    }

    Ok(all)
}

pub fn send_text(token: &str, channel_id: &str, text: &str) -> Result<(), String> {
    let resp = client(token)
        .post(format!("{API_BASE}/chat.postMessage"))
        .json(&serde_json::json!({ "channel": channel_id, "text": text }))
        .send()
        .map_err(|e| format!("Request failed: {e}"))?;
    parse_ok(resp)
}

pub fn send_image(token: &str, channel_id: &str, base64_data: &str) -> Result<(), String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| format!("Invalid base64: {e}"))?;
    let length = bytes.len().to_string();

    let resp = client(token)
        .post(format!("{API_BASE}/files.getUploadURLExternal"))
        .form(&[("filename", "clipboard.png"), ("length", length.as_str())])
        .send()
        .map_err(|e| format!("Request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Slack API error ({status}): {body}"));
    }
    let upload: UploadUrlResponse = resp.json().map_err(|e| format!("Bad response: {e}"))?;
    if !upload.ok {
        return Err(api_error(upload.error));
    }

    let part = reqwest::blocking::multipart::Part::bytes(bytes)
        .file_name("clipboard.png")
        .mime_str("image/png")
        .map_err(|e| format!("MIME error: {e}"))?;
    let form = reqwest::blocking::multipart::Form::new().part("file", part);

    let up_resp = client(token)
        .post(&upload.upload_url)
        .multipart(form)
        .send()
        .map_err(|e| format!("Upload failed: {e}"))?;
    if !up_resp.status().is_success() {
        let status = up_resp.status();
        let body = up_resp.text().unwrap_or_default();
        return Err(format!("Slack upload error ({status}): {body}"));
    }

    let complete = client(token)
        .post(format!("{API_BASE}/files.completeUploadExternal"))
        .json(&serde_json::json!({
            "files": [{ "id": upload.file_id, "title": "clipboard.png" }],
            "channel_id": channel_id
        }))
        .send()
        .map_err(|e| format!("Request failed: {e}"))?;
    parse_ok(complete)
}
