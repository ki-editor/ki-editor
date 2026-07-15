#!/usr/bin/env bash

set -euo pipefail

repo="${GITHUB_REPOSITORY:?}"
fast="${FAST:-true}"
expected_version="${EXPECTED_VERSION:-}"
snapshot_keep="${SNAPSHOT_KEEP:-14}"
stage="$RUNNER_TEMP/release-channels"

rm -rf "$stage"
mkdir -p "$stage"

effective_fast="$fast"
if git -C source fetch origin channels --depth=1 >/dev/null 2>&1 \
  && git -C source show FETCH_HEAD:releases.jsonl \
    > "$stage/existing-releases.jsonl" 2>/dev/null; then
  :
else
  : > "$stage/existing-releases.jsonl"
  effective_fast=false
fi

normalize_releases() {
  jq -c '
    .[]
    | select(.draft == false)
    | if (.tag_name | test("^v[0-9]+\\.[0-9]+\\.[0-9]+$")) then
        {
          version: .tag_name,
          channel: "stable",
          created_at: .created_at,
          commit_id: .target_commitish
        }
      elif (.tag_name | test("^snapshot-[0-9]{8}-[0-9]{4}-[0-9a-f]{12}$")) then
        {
          version: .tag_name,
          channel: "snapshot",
          created_at: .created_at,
          commit_id: .target_commitish
        }
      else
        empty
      end
  '
}

query_releases() {
  if [ "$effective_fast" = "true" ]; then
    gh api "repos/$repo/releases?per_page=100" | normalize_releases
  else
    gh api --paginate --slurp "repos/$repo/releases?per_page=100" \
      | jq -c '.[][]' \
      | jq -cs '.' \
      | normalize_releases
  fi
}

attempt=1
while :; do
  query_releases > "$stage/api-releases.jsonl"

  if [ -z "$expected_version" ] \
    || jq -e --arg version "$expected_version" \
      'select(.version == $version)' "$stage/api-releases.jsonl" >/dev/null; then
    break
  fi

  if [ "$attempt" -ge 5 ]; then
    echo "Published release is not visible through the API: $expected_version" >&2
    exit 1
  fi

  sleep "$((attempt * 2))"
  attempt="$((attempt + 1))"
done

if [ "$effective_fast" = "true" ]; then
  jq -cs '
    reduce .[] as $release ({}; .[$release.version] = $release)
    | [.[]]
    | sort_by([.created_at, .version])
    | reverse
    | .[]
  ' "$stage/existing-releases.jsonl" "$stage/api-releases.jsonl" \
    > "$stage/merged-releases.jsonl"
else
  cp "$stage/api-releases.jsonl" "$stage/merged-releases.jsonl"
fi

jq -cs '
  sort_by([.created_at, .version])
  | reverse
  | .[]
' "$stage/merged-releases.jsonl" > "$stage/sorted-releases.jsonl"
mv "$stage/sorted-releases.jsonl" "$stage/merged-releases.jsonl"

jq -cs --argjson snapshot_keep "$snapshot_keep" '
  [ .[] | select(.channel == "stable") ],
  ([ .[] | select(.channel == "snapshot") ] | .[:$snapshot_keep])
  | .[]
' "$stage/merged-releases.jsonl" > "$stage/retained-releases.jsonl"

jq -rcs --argjson snapshot_keep "$snapshot_keep" '
  [ .[] | select(.channel == "snapshot") ][ $snapshot_keep: ]
  | .[].version
' "$stage/merged-releases.jsonl" > "$stage/releases-to-delete.txt"

while IFS= read -r version; do
  if lookup_error="$(gh release view "$version" --repo "$repo" 2>&1 >/dev/null)"; then
    gh release delete "$version" --repo "$repo" --cleanup-tag --yes
  elif printf '%s\n' "$lookup_error" | grep -Eq 'HTTP 404|release not found'; then
    echo "Release already absent: $version"
  else
    printf '%s\n' "$lookup_error" >&2
    exit 1
  fi
done < "$stage/releases-to-delete.txt"

if git -C source ls-remote --exit-code --heads origin channels >/dev/null 2>&1; then
  git -C source fetch origin channels
  git -C source switch -C channels origin/channels
else
  git -C source switch --orphan channels
  git -C source rm -rf . >/dev/null 2>&1 || true
fi

cp "$stage/retained-releases.jsonl" source/releases.jsonl

for channel in stable snapshot; do
  mkdir -p "source/$channel"
  jq -r --arg channel "$channel" \
    'select(.channel == $channel) | .version' \
    source/releases.jsonl > "source/$channel/versions.txt"

  if [ -s "source/$channel/versions.txt" ]; then
    head -n 1 "source/$channel/versions.txt" > "source/$channel/latest.txt"
  else
    rm -f "source/$channel/latest.txt"
  fi
done

if [ -z "$(git -C source status --porcelain -- releases.jsonl stable snapshot)" ]; then
  exit 0
fi

git -C source config user.name github-actions[bot]
git -C source config user.email github-actions[bot]@users.noreply.github.com
git -C source add releases.jsonl stable snapshot
git -C source commit -m "Sync release channel metadata"
git -C source push origin HEAD:channels
