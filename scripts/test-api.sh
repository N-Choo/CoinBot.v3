#!/bin/bash
set -euo pipefail

BASE="${BASE_URL:-http://localhost:8081}"
WALLET="${TEST_WALLET:-0x1234567890abcdef}"
PASS=0
FAIL=0

pass()  { echo "PASS: $1"; PASS=$((PASS + 1)); }
fail()  { echo "FAIL: $1"; FAIL=$((FAIL + 1)); }
check() { local msg="$1"; shift; if "$@" > /dev/null 2>&1; then pass "$msg"; else fail "$msg"; fi; }
header() { echo; echo "=== $1 ==="; }

cleanup() { echo; echo "---"; echo "PASS: $PASS | FAIL: $FAIL"; [ "$FAIL" -eq 0 ]; }
trap cleanup EXIT

# ── CORS Preflight ──
header "CORS Preflight"

status=$(curl -s -o /dev/null -w "%{http_code}" -X OPTIONS \
    -H "Origin: http://127.0.0.1:5173" \
    -H "Access-Control-Request-Method: GET" \
    "$BASE/api/")
check "OPTIONS /api/ returns 200" test "$status" = 200

headers=$(curl -sI -X OPTIONS \
    -H "Origin: http://127.0.0.1:5173" \
    -H "Access-Control-Request-Method: GET" \
    "$BASE/api/")
check "OPTIONS /api/ has Access-Control-Allow-Origin" \
    echo "$headers" | grep -qi "access-control-allow-origin"

status=$(curl -s -o /dev/null -w "%{http_code}" -X OPTIONS \
    -H "Origin: http://127.0.0.1:5173" \
    -H "Access-Control-Request-Method: GET" \
    "$BASE/kucoin-api/")
check "OPTIONS /kucoin-api/ responds" test "$status" = 200

# ── Endpoint Reachability ──
header "Endpoint Reachability"

status=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE/api/user/auth?wallet_address=$WALLET")
check "GET /api/user/auth?wallet_address=... returns 200" test "$status" = 200

status=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE/api/config")
check "GET /api/config returns 200" test "$status" = 200

status=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE/api/nonexistent")
check "GET /api/nonexistent returns 404" test "$status" = 404

# ── Auth Flow ──
header "Auth Flow"

auth_resp=$(curl -s "$BASE/api/user/auth?wallet_address=$WALLET")
check "GET /api/user/auth returns nonce" \
    echo "$auth_resp" | grep -q '"nonce"'

NONCE=$(echo "$auth_resp" | python3 -c "import sys,json; print(json.load(sys.stdin)['nonce'])" 2>/dev/null || echo "")

if [ -n "$NONCE" ]; then
    status=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
        "$BASE/api/user/auth")
    check "POST /api/user/auth with empty body returns 400" test "$status" = 400

    status=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
        -H "Content-Type: application/json" \
        -d "{\"msg\":\"$NONCE\",\"signature\":\"0xdeadbeef\"}" \
        "$BASE/api/user/auth")
    check "POST /api/user/auth with invalid signature returns 401" test "$status" = 401
else
    fail "Could not extract nonce from response"
fi

# ── Endpoint shape ──
header "Endpoint Shape"

config_json=$(curl -s "$BASE/api/config")
check "GET /api/config returns valid JSON" \
    python3 -c "import sys,json; json.load(sys.stdin)" <<< "$config_json"
