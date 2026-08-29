---
name: zulip-fetch
description: Fetch the most recent messages from the ki-editor Zulip organization. Use when the user asks to check, read, or fetch Zulip messages/notifications.
---

# Zulip Fetch

Fetches the N most recent messages (default 10) from the ki-editor Zulip
organization via the Zulip REST API.

## Prerequisites

A `.env` file at the repo root (gitignored) with:

```
ZULIP_SITE=https://ki-editor.zulipchat.com
ZULIP_EMAIL=<account email>
ZULIP_API_KEY=<api key>
```

Get the API key from `https://ki-editor.zulipchat.com/#settings/account-and-privacy`
(personal key) or `https://ki-editor.zulipchat.com/#settings/your-bots` (bot key).

Requires `curl` and `jq` on PATH.

## Usage

```bash
bash .claude/skills/zulip-fetch/scripts/zulip_fetch.sh [N]
```

- `N` — number of most recent messages to fetch (default: 10).

Prints each message as:

```
[YYYY-MM-DD HH:MM] Sender Name (stream / topic or DM recipients):
message content (HTML tags stripped)
```

## Notes

- Never print or log the contents of `.env` (it holds a live API key).
- If the request fails, check that `ZULIP_API_KEY` is actually populated in
  `.env` and that `ZULIP_SITE` matches the org's real URL.
