#!/usr/bin/env bash
# Fetch the N most recent Zulip messages using credentials from .env
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
cd "$REPO_ROOT"
set -a
source .env
set +a

NUM="${1:-10}"

curl -s \
  -u "${ZULIP_EMAIL}:${ZULIP_API_KEY}" \
  --get "${ZULIP_SITE}/api/v1/messages" \
  --data-urlencode "anchor=newest" \
  --data-urlencode "num_before=${NUM}" \
  --data-urlencode "num_after=0" \
  --data-urlencode 'narrow=[]' \
  | jq -r '.messages[] | . as $m
      | (if ($m.display_recipient | type) == "array"
           then ($m.display_recipient | map(.full_name) | join(", "))
           else $m.display_recipient
         end) as $recipient
      | "[\($m.timestamp | gmtime | strftime("%Y-%m-%d %H:%M"))] \($m.sender_full_name) (\($recipient)\(if $m.subject != "" then " / " + $m.subject else "" end)):\n\($m.content | gsub("<[^>]*>";""))\n"'
