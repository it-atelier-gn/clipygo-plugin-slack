use crate::api;
use crate::config::{load_config, save_config};
use crate::protocol::{InfoResponse, Request, SendResponse, Target, TargetsResponse};

const ICON: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGPwEvUGAAFXAKtZG+t6AAAAAElFTkSuQmCC";

pub fn handle(request: Request) -> serde_json::Value {
    match request {
        Request::GetInfo => serde_json::to_value(InfoResponse {
            name: "Slack",
            version: env!("CARGO_PKG_VERSION"),
            description: "Send clipboard content to Slack channels",
            author: "clipygo",
            link: Some("https://github.com/it-atelier-gn/clipygo-plugin-slack"),
        })
        .unwrap(),

        Request::GetTargets => {
            let config = load_config();

            if config.bot_token.is_empty() {
                eprintln!("[slack] Bot token not configured");
                return serde_json::to_value(TargetsResponse { targets: vec![] }).unwrap();
            }

            let channels = match api::fetch_channels(&config.bot_token) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[slack] Failed to fetch channels: {e}");
                    return serde_json::to_value(TargetsResponse { targets: vec![] }).unwrap();
                }
            };

            let mut targets = Vec::new();
            for channel in channels {
                if channel.is_archived || !channel.is_member {
                    continue;
                }
                let name = channel.name.as_deref().unwrap_or("unknown");
                let description = if channel.is_private {
                    "Private channel"
                } else {
                    "Public channel"
                };
                targets.push(Target {
                    id: format!("channel:{}", channel.id),
                    provider: "Slack".to_string(),
                    formats: vec!["text".to_string(), "image".to_string()],
                    title: format!("#{name}"),
                    description: description.to_string(),
                    image: ICON.to_string(),
                });
            }

            if targets.is_empty() {
                eprintln!(
                    "[slack] No channels found. Invite the bot to a channel with /invite @your-bot"
                );
            }

            serde_json::to_value(TargetsResponse { targets }).unwrap()
        }

        Request::GetConfigSchema => {
            let config = load_config();
            serde_json::json!({
                "instructions": "1. Go to https://api.slack.com/apps → Create New App → From scratch\n\
                    2. Under OAuth & Permissions, add Bot Token Scopes:\n\
                       channels:read, groups:read, chat:write, files:write\n\
                    3. Install the app to your workspace and copy the Bot User OAuth Token (starts with xoxb-)\n\
                    4. Invite the bot to each channel you want to post to with /invite @your-bot",
                "schema": {
                    "type": "object",
                    "title": "Slack",
                    "properties": {
                        "bot_token": {
                            "type": "string",
                            "title": "Bot Token",
                            "description": "Bot User OAuth Token from api.slack.com (xoxb-…)",
                            "format": "password"
                        }
                    },
                    "required": ["bot_token"]
                },
                "values": {
                    "bot_token": config.bot_token
                }
            })
        }

        Request::SetConfig { values } => {
            let mut config = load_config();

            if let Some(v) = values.get("bot_token").and_then(|v| v.as_str()) {
                config.bot_token = v.to_string();
            }

            save_config(&config);

            serde_json::to_value(SendResponse {
                success: true,
                error: None,
            })
            .unwrap()
        }

        Request::Send {
            target_id,
            content,
            format,
        } => {
            let config = load_config();

            if config.bot_token.is_empty() {
                return serde_json::to_value(SendResponse {
                    success: false,
                    error: Some("Bot token not configured".to_string()),
                })
                .unwrap();
            }

            let channel_id = target_id.strip_prefix("channel:").unwrap_or(&target_id);

            let result = match format.as_str() {
                "text" => api::send_text(&config.bot_token, channel_id, &content),
                "image" => api::send_image(&config.bot_token, channel_id, &content),
                _ => Err(format!("Unsupported format: {format}")),
            };

            match result {
                Ok(()) => serde_json::to_value(SendResponse {
                    success: true,
                    error: None,
                })
                .unwrap(),
                Err(e) => serde_json::to_value(SendResponse {
                    success: false,
                    error: Some(e),
                })
                .unwrap(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_info_fields() {
        let resp = handle(Request::GetInfo);
        assert_eq!(resp["name"], "Slack");
        assert!(resp["version"].is_string());
        assert!(resp["description"].is_string());
        assert_eq!(resp["author"], "clipygo");
    }

    #[test]
    fn get_info_includes_link() {
        let resp = handle(Request::GetInfo);
        assert!(resp["link"].as_str().unwrap().starts_with("https://"));
    }

    #[test]
    fn get_targets_empty_when_no_token() {
        let resp = handle(Request::GetTargets);
        let targets = resp["targets"].as_array().unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn get_config_schema_has_required_fields() {
        let resp = handle(Request::GetConfigSchema);
        assert!(resp.get("instructions").is_some());
        assert!(resp.get("schema").is_some());
        assert!(resp.get("values").is_some());
        let props = &resp["schema"]["properties"];
        assert!(props.get("bot_token").is_some());
    }

    #[test]
    fn get_config_schema_bot_token_is_password() {
        let resp = handle(Request::GetConfigSchema);
        let format = resp["schema"]["properties"]["bot_token"]["format"]
            .as_str()
            .unwrap();
        assert_eq!(format, "password");
    }

    #[test]
    fn get_config_schema_mentions_scopes() {
        let resp = handle(Request::GetConfigSchema);
        let instructions = resp["instructions"].as_str().unwrap();
        assert!(instructions.contains("chat:write"));
        assert!(instructions.contains("xoxb-"));
    }

    #[test]
    fn set_config_returns_success() {
        let resp = handle(Request::SetConfig {
            values: serde_json::json!({ "bot_token": "test-token" }),
        });
        assert_eq!(resp["success"], true);
    }

    #[test]
    fn send_fails_without_token() {
        save_config(&crate::config::Config::default());
        let resp = handle(Request::Send {
            target_id: "channel:C123".to_string(),
            content: "hello".to_string(),
            format: "text".to_string(),
        });
        assert_eq!(resp["success"], false);
        assert!(resp["error"].as_str().unwrap().contains("token"));
    }

    #[test]
    fn send_rejects_unsupported_format() {
        save_config(&crate::config::Config {
            bot_token: "xoxb-fake".to_string(),
        });
        let resp = handle(Request::Send {
            target_id: "channel:C123".to_string(),
            content: "data".to_string(),
            format: "video".to_string(),
        });
        assert_eq!(resp["success"], false);
        assert!(resp["error"].as_str().unwrap().contains("video"));
    }

    #[test]
    fn invalid_json_rejected() {
        assert!(serde_json::from_str::<Request>("not json").is_err());
    }

    #[test]
    fn unknown_command_rejected() {
        assert!(serde_json::from_str::<Request>(r#"{"command":"unknown"}"#).is_err());
    }

    #[test]
    fn config_roundtrip() {
        let config = crate::config::Config {
            bot_token: "xoxb-test".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: crate::config::Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bot_token, "xoxb-test");
    }
}
