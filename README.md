# clipygo-plugin-slack

Slack target provider for [clipygo](https://github.com/it-atelier-gn/clipygo).

Sends clipboard content (text and images) to Slack channels via the Web API.

## Setup

1. Go to [api.slack.com/apps](https://api.slack.com/apps) → **Create New App** → **From scratch**
2. Under **OAuth & Permissions**, add these Bot Token Scopes:
   - `channels:read` — list public channels
   - `groups:read` — list private channels
   - `chat:write` — post messages
   - `files:write` — upload images
3. Install the app to your workspace and copy the **Bot User OAuth Token** (starts with `xoxb-`)
4. Invite the bot to each channel you want to post to: `/invite @your-bot`
5. In clipygo Settings → Plugins, add the plugin and paste the bot token

The plugin auto-discovers all non-archived channels the bot has been invited to.

## Supported formats

- **text** — sent as a message via `chat.postMessage`
- **image** — uploaded as a file attachment (base64-encoded PNG) via the external upload API

## Build

```sh
cargo build --release
```

## License

MIT
