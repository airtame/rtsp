#!/bin/sh
set -e

RESPONSE=$(curl --silent --show-error -X POST https://crates.io/api/v1/trusted_publishing/tokens \
  -H "Content-Type: application/json" \
  -H "User-Agent: airtame-rtsp-gitlab-ci (https://gitlab.com/airtame/crates/rtsp)" \
  -d "{\"jwt\": \"$CRATES_IO_ID_TOKEN\"}")
echo "$RESPONSE" | jq -r '.token // empty'
