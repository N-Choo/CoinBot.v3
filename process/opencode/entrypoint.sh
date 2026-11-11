#!/bin/sh
set -eu

: "${OPENCODE_SERVER_PASSWORD:?OPENCODE_SERVER_PASSWORD must be set}"

mkdir -p /root/.local/share/opencode

if [ -n "${OPENCODE_GO_API_KEY:-}" ]; then
	node -e 'const fs = require("fs"); const key = process.env.OPENCODE_GO_API_KEY; fs.writeFileSync("/root/.local/share/opencode/auth.json", JSON.stringify({ "opencode-go": { type: "api", key } }));'
fi

exec opencode serve --hostname 0.0.0.0 --port 4096
