#!/usr/bin/env python3
"""Read-only CoinBot status dashboard.

Run from the repo root (or via scripts/status.sh) on the Pi. It uses the root
.env plus Docker Compose, Postgres, the signal-gateway audit volume, and a
loopback curl to opencode. No application changes and no writes.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timedelta, timezone
from decimal import Decimal, InvalidOperation
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ENV_PATH = ROOT / ".env"
CANDLE = timedelta(hours=4)
DEFAULT_DELAY = 90
WARN_OLD_ORDER_MIN = 10
STUCK_CLOSING_MIN = 30
ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
DB_ERRORS: list[str] = []

RESET = "\x1b[0m"
DIM = "\x1b[2m"
BOLD = "\x1b[1m"
GREEN = "\x1b[32m"
AMBER = "\x1b[33m"
RED = "\x1b[31m"

SERVICE_LABELS = [
    ("api_gateway-prod", "api"),
    ("deposit-worker-prod", "deposit"),
    ("withdraw-worker-prod", "withdraw"),
    ("trade-engine-prod", "engine"),
    ("signal-gateway-prod", "signal-gateway"),
    ("opencode", "opencode"),
    ("redis", "redis"),
    ("postgres", "postgres"),
]


def load_env() -> dict[str, str]:
    data: dict[str, str] = {}
    if not ENV_PATH.exists():
        return data
    for line in ENV_PATH.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        data[key.strip()] = value.strip().strip('"').strip("'")
    return data


def compose(env: dict[str, str]) -> list[str]:
    return [
        "docker",
        "compose",
        "--env-file",
        ".env",
        "-f",
        "docker-compose.yml",
        "-f",
        "docker-compose.prod.yml",
        "--profile",
        "prod",
        "--profile",
        "tools",
    ]


def run(cmd: list[str], timeout: int = 20) -> subprocess.CompletedProcess | None:
    try:
        return subprocess.run(
            cmd,
            cwd=ROOT,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None


def psql(env: dict[str, str], query: str) -> list[list[str]] | None:
    user = env.get("POSTGRES_USER", "postgres")
    db = env.get("POSTGRES_DB", "coinbot")
    cmd = compose(env) + [
        "exec",
        "-T",
        "postgres",
        "psql",
        "-U",
        user,
        "-d",
        db,
        "-v",
        "ON_ERROR_STOP=1",
        "-tA",
        "-F",
        "|",
        "-c",
        query,
    ]
    result = run(cmd)
    if result is None or result.returncode != 0:
        DB_ERRORS.append(" ".join(query.split())[:120])
        return None
    return [line.split("|") for line in result.stdout.splitlines() if line.strip()]


def one(env: dict[str, str], query: str) -> list[str]:
    rows = psql(env, query)
    return rows[0] if rows else []


def dec(value: object, default: str = "0") -> Decimal:
    text = str(value).strip() if value is not None else default
    try:
        parsed = Decimal(text or default)
    except (InvalidOperation, ValueError):
        return Decimal(default)
    return parsed if parsed.is_finite() else Decimal(default)


def fmt_money(value: object) -> str:
    return f"{dec(value):,.2f}"


def fmt_qty(value: object) -> str:
    text = f"{dec(value):.8f}".rstrip("0").rstrip(".")
    return text or "0"


def fmt_usdt(value: object) -> str:
    return fmt_qty(value)


def norm_ticker(value: object) -> str:
    return str(value).strip().upper().replace("/", "-")


def pretty_ticker(value: str) -> str:
    return value.replace("-", "/")


def parse_dt(value: str | None) -> datetime | None:
    if not value:
        return None
    text = value.strip().replace(" ", "T").replace("Z", "+00:00")
    text = re.sub(r"(\.\d{6})\d+", r"\1", text)
    text = re.sub(r"([+-]\d{2})$", r"\1:00", text)
    try:
        parsed = datetime.fromisoformat(text)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def ago(value: str | datetime | None, reference: datetime | None = None) -> str:
    dt = value if isinstance(value, datetime) else parse_dt(value)
    if dt is None:
        return "unknown"
    ref = reference or datetime.now(timezone.utc)
    seconds = max(0, int((ref - dt).total_seconds()))
    if seconds < 60:
        return f"{seconds}s"
    if seconds < 3600:
        return f"{seconds // 60}m"
    if seconds < 86400:
        return f"{seconds // 3600}h"
    return f"{seconds // 86400}d"


def floor4(dt: datetime) -> datetime:
    return dt.replace(hour=(dt.hour // 4) * 4, minute=0, second=0, microsecond=0)


def current_candle_open(now: datetime) -> datetime:
    return floor4(now) - CANDLE


def next_cycle(now: datetime, delay: int) -> datetime:
    return floor4(now - timedelta(seconds=delay)) + CANDLE + timedelta(seconds=delay)


def clock(dt: datetime) -> str:
    return dt.strftime("%H:%M")


def visible_len(text: str) -> int:
    return len(ANSI_RE.sub("", text))


def truncate(text: str, width: int) -> str:
    if visible_len(text) <= width:
        return text
    plain = ANSI_RE.sub("", text)
    return plain[: max(0, width - 1)] + "…"


def pad(text: str, width: int) -> str:
    return text + " " * max(0, width - visible_len(text))


def paint(text: str, color: str, enabled: bool) -> str:
    return f"{color}{text}{RESET}" if enabled else text


def badge(level: str, enabled: bool) -> str:
    if level == "ok":
        return paint("●", GREEN, enabled)
    if level == "warn":
        return paint("◐", AMBER, enabled)
    return paint("✗", RED, enabled)


def signal_state(entry: dict | None) -> str:
    if not entry:
        return "—"
    verdict = str(entry.get("verdict") or "?")
    validation = str(entry.get("validation") or "")
    ack = str(entry.get("ack") or "")
    if verdict == "NO-TRADE":
        return "NO-TRADE"
    if validation == "valid":
        if "accepted=false" in ack:
            return f"{verdict} rejected"
        if "duplicate" in ack:
            return f"{verdict} duplicate"
        return f"{verdict} ok" if "accepted=true" in ack else f"{verdict} sent"
    if validation.startswith("rejected"):
        return f"{verdict} rejected"
    if validation.startswith("agent failed"):
        return "agent failed"
    return f"{verdict} {validation}".strip()


def fetch_services(env: dict[str, str]) -> dict[str, dict]:
    result = run(compose(env) + ["ps", "-a", "--format", "json"])
    services: dict[str, dict] = {}
    if result is None or result.returncode != 0:
        return services
    text = result.stdout.strip()
    rows: list[dict] = []
    if text.startswith("["):
        try:
            rows = json.loads(text)
        except json.JSONDecodeError:
            rows = []
    else:
        for line in text.splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    for row in rows:
        name = str(row.get("Service") or "")
        if name:
            services[name] = row
    return services


def fetch_contracts(env: dict[str, str]) -> list[dict]:
    rows = psql(
        env,
        """
        SELECT ticker,
               MAX(CASE WHEN status = 'active' THEN 1 ELSE 0 END),
               STRING_AGG(DISTINCT status, ',' ORDER BY status),
               COALESCE(SUM(init_fund), 0),
               COALESCE(SUM(available_fund), 0),
               COUNT(*),
               MAX(updated_at)
        FROM contracts
        WHERE status IN ('active', 'inactive', 'closing')
        GROUP BY ticker
        ORDER BY MAX(CASE WHEN status = 'active' THEN 1 ELSE 0 END) DESC, ticker
        """,
    )
    if rows is None:
        return []
    out = []
    for row in rows:
        if len(row) < 7:
            continue
        out.append(
            {
                "ticker": row[0],
                "has_active": row[1] == "1",
                "statuses": row[2] or "unknown",
                "init_fund": row[3],
                "available_fund": row[4],
                "count": int(row[5] or 0),
                "updated_at": row[6],
            }
        )
    return out


def fetch_money(env: dict[str, str]) -> dict:
    liabilities = one(env, "SELECT COALESCE(SUM(balance + locked_balance), 0) FROM users")
    funds = one(
        env,
        """
        SELECT COALESCE(SUM(init_fund), 0), COALESCE(SUM(available_fund), 0)
        FROM contracts WHERE status IN ('active', 'inactive', 'closing')
        """,
    )
    fees = one(env, "SELECT COALESCE(SUM(amount), 0) FROM platform_fees")
    borrow = one(env, "SELECT COALESCE(SUM(borrow_interest), 0) FROM contracts")
    return {
        "liabilities": liabilities[0] if liabilities else "0",
        "allocated": funds[0] if funds else "0",
        "available": funds[1] if len(funds) > 1 else "0",
        "fees": fees[0] if fees else "0",
        "borrow_interest": borrow[0] if borrow else "0",
    }


def fetch_loans(env: dict[str, str]) -> dict:
    per_currency = psql(
        env,
        """
        SELECT currency,
               COALESCE(SUM(principal), 0),
               COALESCE(SUM(outstanding), 0),
               COALESCE(SUM(interest_usdt), 0),
               COUNT(*),
               MIN(opened_at)
        FROM margin_loans
        WHERE repaid_at IS NULL
        GROUP BY currency
        ORDER BY currency
        """,
    ) or []
    counts = one(
        env,
        """
        SELECT
          COUNT(*) FILTER (WHERE position_closed_at IS NOT NULL AND repaid_at IS NULL),
          COUNT(*) FILTER (WHERE position_closed_at IS NULL AND repaid_at IS NULL),
          COUNT(*) FILTER (WHERE repay_attempted_at IS NOT NULL AND repaid_at IS NULL)
        FROM margin_loans
        """,
    )
    return {
        "currencies": per_currency,
        "unrepaid_closed": counts[0] if counts else "0",
        "open_positions": counts[1] if len(counts) > 1 else "0",
        "repay_pending": counts[2] if len(counts) > 2 else "0",
    }


def fetch_orders(env: dict[str, str]) -> dict:
    counts = psql(env, "SELECT status, COUNT(*) FROM trade_orders GROUP BY status") or []
    by_status = {row[0]: int(row[1]) for row in counts if len(row) == 2}
    today = one(
        env,
        """
        SELECT
          COUNT(*) FILTER (WHERE action = 'close' AND status = 'filled' AND updated_at >= CURRENT_DATE),
          COALESCE(SUM(pnl) FILTER (WHERE action = 'close' AND status = 'filled' AND updated_at >= CURRENT_DATE), 0),
          COUNT(*) FILTER (WHERE action = 'close' AND status = 'filled' AND updated_at >= CURRENT_DATE AND pnl >= 0)
        FROM trade_orders
        """,
    )
    old = one(
        env,
        f"""
        SELECT COUNT(*) FROM trade_orders
        WHERE status IN ('pending', 'open') AND created_at < NOW() - INTERVAL '{WARN_OLD_ORDER_MIN} minutes'
        """,
    )
    filled = int(today[0]) if today else 0
    wins = int(today[2]) if len(today) > 2 else 0
    return {
        "pending": by_status.get("pending", 0),
        "open": by_status.get("open", 0),
        "filled_today": filled,
        "failed": by_status.get("failed", 0) + by_status.get("cancelled", 0),
        "pnl_today": today[1] if len(today) > 1 else "0",
        "win_rate": (wins * 100.0 / filled) if filled else 0.0,
        "old_open": int(old[0]) if old else 0,
    }


def fetch_flows(env: dict[str, str]) -> dict:
    withdrawals = psql(
        env,
        """
        SELECT status, COUNT(*), COALESCE(SUM(total), 0), MIN(created_at)
        FROM withdrawals GROUP BY status
        """,
    ) or []
    deposits = psql(env, "SELECT status, COUNT(*) FROM deposits GROUP BY status") or []
    by_status = {row[0]: row for row in withdrawals if len(row) >= 4}
    inflight_names = ("pending", "processing", "submitted")
    inflight = [by_status[name] for name in inflight_names if name in by_status]
    inflight_count = sum(int(row[1]) for row in inflight)
    inflight_total = sum((dec(row[2]) for row in inflight), Decimal("0"))
    oldest = min((row[3] for row in inflight if row[3]), default=None)
    expired = int(by_status.get("expired", ["expired", "0", "0", None])[1])
    failed = int(by_status.get("failed", ["failed", "0", "0", None])[1])
    unswept = one(
        env,
        "SELECT COUNT(*) FROM deposits WHERE status = 'confirmed' AND transferred_at IS NULL",
    )
    awaiting = sum(int(row[1]) for row in deposits if row[0] == "pending")
    dep_failed = sum(int(row[1]) for row in deposits if row[0] == "failed")
    return {
        "inflight": inflight_count,
        "inflight_total": str(inflight_total),
        "oldest": oldest,
        "expired": expired,
        "failed": failed,
        "awaiting": awaiting,
        "unswept": int(unswept[0]) if unswept else 0,
        "dep_failed": dep_failed,
    }


def fetch_stuck(env: dict[str, str]) -> list[list[str]]:
    return psql(
        env,
        f"""
        SELECT id, ticker, updated_at
        FROM contracts
        WHERE status = 'closing'
          AND updated_at < NOW() - INTERVAL '{STUCK_CLOSING_MIN} minutes'
        ORDER BY updated_at
        """,
    ) or []


def read_audit(env: dict[str, str]) -> tuple[list[dict], bool]:
    result = run(compose(env) + ["ps", "-q", "signal-gateway-prod"])
    if result is None or not result.stdout.strip():
        return [], False
    container = result.stdout.strip().splitlines()[0]
    configured = env.get("SIGNAL_GATEWAY_AUDIT_PATH", "")
    if configured:
        container_path = configured if configured.startswith("/") else "/app/" + configured.removeprefix("./")
    else:
        container_path = "/app/state/audit.jsonl"
    tmpdir = tempfile.mkdtemp(prefix="coinbot-status-")
    path = Path(tmpdir) / "audit.jsonl"
    try:
        copied = run(["docker", "cp", f"{container}:{container_path}", str(path)])
        if copied is None or copied.returncode != 0:
            return [], False
        lines = path.read_text(errors="replace").splitlines()[-2000:]
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)
    entries = []
    for line in lines:
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(row, dict):
            entries.append(row)
    return entries, True


def opencode_status() -> str:
    if shutil.which("curl") is None:
        return "unknown"
    result = run(
        [
            "curl",
            "-sS",
            "--max-time",
            "2",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "http://127.0.0.1:4096/",
        ],
        timeout=5,
    )
    if result is None:
        return "down"
    return "up" if result.stdout.strip() not in ("", "000") else "down"


def default_db_values() -> tuple[dict, dict, dict, dict]:
    money = {
        "liabilities": "0",
        "allocated": "0",
        "available": "0",
        "fees": "0",
        "borrow_interest": "0",
    }
    loans = {"currencies": [], "unrepaid_closed": "0", "open_positions": "0", "repay_pending": "0"}
    orders = {
        "pending": 0,
        "open": 0,
        "filled_today": 0,
        "failed": 0,
        "pnl_today": "0",
        "win_rate": 0.0,
        "old_open": 0,
    }
    flows = {
        "inflight": 0,
        "inflight_total": "0",
        "oldest": None,
        "expired": 0,
        "failed": 0,
        "awaiting": 0,
        "unswept": 0,
        "dep_failed": 0,
    }
    return money, loans, orders, flows


def build(
    env: dict[str, str],
    services: dict[str, dict],
    contracts: list[dict],
    money: dict,
    loans: dict,
    orders: dict,
    flows: dict,
    stuck: list[list[str]],
    audit: list[dict],
    delay: int,
    db_ok: bool,
    audit_ok: bool,
    db_errors: int = 0,
) -> dict:
    now = datetime.now(timezone.utc)
    candle = current_candle_open(now)
    nxt = next_cycle(now, delay)
    cycle_start = candle + CANDLE + timedelta(seconds=delay)

    active_tickers = sorted(
        norm_ticker(c["ticker"]) for c in contracts if c.get("has_active")
    )
    total_contracts = sum(c.get("count", 0) for c in contracts)

    last_by_symbol: dict[str, dict] = {}
    done: set[str] = set()
    if audit_ok:
        for entry in audit:
            symbol = norm_ticker(entry.get("symbol") or "")
            if not symbol:
                continue
            at = parse_dt(entry.get("at"))
            prev = last_by_symbol.get(symbol)
            prev_at = parse_dt(prev.get("at")) if prev else None
            if prev is None or (at and prev_at and at > prev_at):
                last_by_symbol[symbol] = entry
            if parse_dt(entry.get("candle_open")) == candle:
                done.add(symbol)
        done &= set(active_tickers)

    window_min = max(30, len(active_tickers) * 8)
    analyzing = [
        s
        for s in active_tickers
        if s not in done
        and cycle_start <= now < cycle_start + timedelta(minutes=window_min)
    ]
    missing = [
        s
        for s in active_tickers
        if s not in done and now >= cycle_start + timedelta(minutes=window_min)
    ]

    issues: list[tuple[str, str]] = []
    service_ok = set()
    for service, _ in SERVICE_LABELS:
        row = services.get(service)
        state = str(row.get("State") or "missing") if row else "missing"
        health = str(row.get("Health") or "") if row else ""
        if state == "running" and health != "unhealthy":
            service_ok.add(service)
        else:
            issues.append(
                (
                    "crit" if state != "running" or health == "unhealthy" else "warn",
                    f"{service} is {health or state}",
                )
            )

    opencode = opencode_status()
    if opencode == "down" and "opencode" in service_ok:
        issues.append(("crit", "opencode is unreachable on 127.0.0.1:4096"))
    if not db_ok:
        issues.append(("crit", "postgres is unreachable"))
    elif db_errors:
        issues.append(("warn", f"{db_errors} database query/queries failed; values may be incomplete"))
    if not audit_ok and "signal-gateway-prod" in service_ok:
        issues.append(("warn", "signal-gateway audit is unavailable"))

    for row in stuck:
        issues.append(("warn", f"contract {row[1]} stuck closing since {ago(row[2])}"))
    if db_ok:
        if int(loans["unrepaid_closed"]) > 0:
            issues.append(("warn", f"{loans['unrepaid_closed']} closed loan(s) not repaid"))
        if int(loans["repay_pending"]) > 0:
            issues.append(("warn", f"{loans['repay_pending']} repay attempt(s) pending"))
        if orders["old_open"]:
            issues.append(("warn", f"{orders['old_open']} open order(s) older than {WARN_OLD_ORDER_MIN}m"))
        if flows["inflight"] and flows["oldest"] and parse_dt(flows["oldest"]):
            age = now - parse_dt(flows["oldest"])
            if age > timedelta(minutes=30):
                issues.append(("warn", f"{flows['inflight']} withdrawal(s) in flight; oldest {ago(flows['oldest'])}"))
        if flows["unswept"]:
            issues.append(("warn", f"{flows['unswept']} confirmed deposit(s) not swept"))
    if audit_ok:
        for symbol in analyzing:
            issues.append(("warn", f"{pretty_ticker(symbol)} analyzing for {ago(cycle_start)}"))
        for symbol in missing:
            issues.append(("warn", f"{pretty_ticker(symbol)} has no signal for the current candle"))

    level = "crit" if any(i[0] == "crit" for i in issues) else ("warn" if issues else "ok")
    return {
        "at": now,
        "candle": candle,
        "next": nxt,
        "cycle_start": cycle_start,
        "delay": delay,
        "services": services,
        "service_ok": service_ok,
        "contracts": contracts,
        "total_contracts": total_contracts,
        "active_tickers": active_tickers,
        "last_by_symbol": last_by_symbol,
        "done": done,
        "analyzing": analyzing,
        "missing": missing,
        "money": money,
        "loans": loans,
        "orders": orders,
        "flows": flows,
        "stuck": stuck,
        "audit": audit,
        "opencode": opencode,
        "db_ok": db_ok,
        "audit_ok": audit_ok,
        "db_errors": db_errors,
        "issues": issues,
        "level": level,
    }


def signal_text(entry: dict | None) -> str:
    at = parse_dt(entry.get("at")) if entry else None
    if at is None:
        return "—"
    today = datetime.now(timezone.utc).date()
    stamp = at.strftime("%H:%M") if at.date() == today else at.strftime("%m-%d %H:%M")
    return f"{stamp} {signal_state(entry)}"


def render_text(data: dict, width: int, color: bool, plain: bool) -> str:
    now = data["at"]
    elapsed = lambda value: ago(value, now)
    issues = data["issues"]
    level = data["level"]
    lines: list[str] = []

    bits = [
        f"{badge(level if issues else 'ok', color)} {len(issues)} issue(s)" if issues else f"{badge('ok', color)} healthy",
        f"· {data['total_contracts']} contracts",
        f"· {len(data['active_tickers'])} symbols",
    ]
    debt = [f"{fmt_qty(row[2])} {row[0]}" for row in data["loans"]["currencies"] if len(row) >= 3]
    if debt:
        bits.append("· " + " + ".join(debt) + " debt")

    if plain:
        lines.append(f"coinbot status · {now.strftime('%Y-%m-%d %H:%M:%S')} UTC")
        lines.append(" ".join(bits))
        lines.append("")
        lines.append("SERVICES")
        for service, label in SERVICE_LABELS:
            row = data["services"].get(service, {})
            state = str(row.get("State") or "missing")
            health = str(row.get("Health") or "")
            ok = state == "running" and health != "unhealthy"
            lines.append(f"  {'ok' if ok else 'FAIL':4} {label:16} {health or state}")
        lines.append("")
        if data["db_ok"]:
            lines.append("CONTRACTS")
            lines.append("  ticker      n  status    allocated   last signal             next")
            for c in data["contracts"]:
                key = norm_ticker(c["ticker"])
                sig = "—"
                if key in data["analyzing"]:
                    sig = f"analyzing {elapsed(data['cycle_start'])}"
                elif key in data["missing"]:
                    sig = "no signal"
                elif key in data["last_by_symbol"]:
                    sig = signal_text(data["last_by_symbol"][key])
                nxt = clock(data["next"]) if c.get("has_active") else "—"
                status = c["statuses"].replace(",", "/")
                lines.append(
                    f"  {c['ticker']:<11} {c['count']:>2} {status:<9} {fmt_money(c['init_fund']):>10}   {sig:<22} {nxt}"
                )
            lines.append("")
            lines.append("MONEY")
            lines.append(
                f"  liabilities {fmt_money(data['money']['liabilities'])} USDT · allocated {fmt_money(data['money']['allocated'])} · available {fmt_money(data['money']['available'])}"
            )
            for row in data["loans"]["currencies"]:
                lines.append(
                    f"  debt {row[0]} {fmt_qty(row[2])} · interest {fmt_usdt(row[3])} · {row[4]} loan(s)"
                )
            lines.append("")
            lines.append("FLOWS")
            lines.append(
                f"  orders {data['orders']['pending']} pending · {data['orders']['open']} open · {data['orders']['filled_today']} filled today · PnL {fmt_money(data['orders']['pnl_today'])}"
            )
            lines.append(
                f"  withdrawals {data['flows']['inflight']} in flight ({fmt_money(data['flows']['inflight_total'])}) · expired {data['flows']['expired']} · failed {data['flows']['failed']}"
            )
            lines.append(
                f"  deposits {data['flows']['awaiting']} awaiting · {data['flows']['unswept']} unswept · {data['flows']['dep_failed']} failed"
            )
        else:
            lines.append("DATABASE UNAVAILABLE")
        if data["audit_ok"]:
            lines.append("")
            lines.append("SIGNALS")
            lines.append(
                f"  {len(data['done'])}/{len(data['active_tickers'])} done · {len(data['analyzing'])} running · {len(data['missing'])} missing · opencode {data['opencode']}"
            )
        if issues:
            lines.append("")
            lines.append("ISSUES")
            for kind, text in issues:
                lines.append(f"  {kind:4} {text}")
        lines.append("")
        lines.append(f"exit {2 if level == 'crit' else 1 if level == 'warn' else 0}")
        return "\n".join(lines)

    def box(title: str, rows: list[str]) -> None:
        inner = width - 2
        lines.append("╭─ " + title + " " + "─" * max(0, width - len(title) - 5) + "╮")
        for row in rows:
            lines.append("│" + pad(" " + truncate(row, inner - 1), inner) + "│")
        lines.append("╰" + "─" * inner + "╯")

    left = paint("CoinBot", BOLD, color)
    right = now.strftime("%H:%M UTC · %a %d %b")
    lines.append(left + " " * max(0, width - visible_len(left) - len(right)) + right)
    lines.append(" ".join(bits))

    health_rows: list[str] = []
    row: list[str] = []
    for service, label in SERVICE_LABELS:
        entry = data["services"].get(service, {})
        state = str(entry.get("State") or "missing")
        health = str(entry.get("Health") or "")
        ok = state == "running" and health != "unhealthy"
        if service == "signal-gateway-prod" and ok:
            text = f"{badge('ok', color)} {label}  next {clock(data['next'])}"
        else:
            text = f"{badge('ok' if ok else 'crit', color)} {label}"
        row.append(text)
        if len(row) == 4:
            health_rows.append("   ".join(row))
            row = []
    if row:
        health_rows.append("   ".join(row))
    box("Health", health_rows)

    if data["db_ok"]:
        portfolio = [
            f"  {'liabilities':<12}{fmt_money(data['money']['liabilities']):>12} USDT   {'allocated':<10}{fmt_money(data['money']['allocated']):>10}",
            f"  {'available':<12}{fmt_money(data['money']['available']):>12}        {'fees':<10}{fmt_money(data['money']['fees']):>10}",
            f"  {'interest':<12}{fmt_money(data['money']['borrow_interest']):>12} USDT",
        ]
        box("Portfolio", portfolio)

        contract_rows = [f"  {'ticker':<11} {'n':>2} {'status':<9} {'allocated':>9}   {'last signal':<22} next"]
        for c in data["contracts"]:
            key = norm_ticker(c["ticker"])
            sig = "—"
            if key in data["analyzing"]:
                sig = f"analyzing {elapsed(data['cycle_start'])}"
            elif key in data["missing"]:
                sig = "no signal"
            elif key in data["last_by_symbol"]:
                sig = signal_text(data["last_by_symbol"][key])
            nxt = clock(data["next"]) if c.get("has_active") else "—"
            status = c["statuses"].replace(",", "/")
            contract_rows.append(
                f"  {c['ticker']:<11} {c['count']:>2} {status:<9} {fmt_money(c['init_fund']):>9}   {sig:<22} {nxt}"
            )
        box("Contracts", contract_rows)

        debt_rows = []
        for row in data["loans"]["currencies"]:
            if len(row) >= 6:
                debt_rows.append(
                    f"  {row[0]:<6} {fmt_qty(row[2]):>12}  interest {fmt_usdt(row[3]):>10}  {row[4]} loan(s) · oldest {elapsed(row[5])}"
                )
        if not debt_rows:
            debt_rows.append("  no open margin debt")
        debt_rows.append(
            f"  unrepaid {data['loans']['unrepaid_closed']} · open positions {data['loans']['open_positions']} · repay pending {data['loans']['repay_pending']}"
        )
        box("Debt / Loans", debt_rows)

        oldest = elapsed(data["flows"]["oldest"]) if data["flows"]["oldest"] else "—"
        flow_rows = [
            f"  orders       {data['orders']['pending']} pending · {data['orders']['open']} open · {data['orders']['filled_today']} filled today · PnL {fmt_money(data['orders']['pnl_today'])} USDT",
            f"  win rate     {data['orders']['win_rate']:.0f}% · failed {data['orders']['failed']}",
            f"  withdrawals  {data['flows']['inflight']} in flight ({fmt_money(data['flows']['inflight_total'])}) · oldest {oldest} · expired {data['flows']['expired']} · failed {data['flows']['failed']}",
            f"  deposits     {data['flows']['awaiting']} awaiting · {data['flows']['unswept']} unswept · {data['flows']['dep_failed']} failed",
        ]
        box("Flows", flow_rows)
    else:
        box("Database", ["  unavailable — check postgres and migrations"])

    if data["audit_ok"]:
        total = len(data["active_tickers"])
        done = len(data["done"])
        box(
            f"Signals {clock(data['cycle_start'])}",
            [
                f"  {done}/{total} done · {len(data['analyzing'])} running · {len(data['missing'])} missing",
                f"  opencode {data['opencode']} · cycle {clock(data['cycle_start'])} · next {clock(data['next'])}",
            ],
        )
    else:
        box("Signals", ["  audit unavailable — signal-gateway running but audit file not readable"])

    if issues:
        issue_rows = [
            f"  {badge('crit' if kind == 'crit' else 'warn', color)} {text}"
            for kind, text in issues
        ]
        box("Issues", issue_rows)

    exit_code = 2 if level == "crit" else 1 if level == "warn" else 0
    lines.append(f"exit {exit_code} · {len(issues)} issue(s)" if issues else "exit 0 · healthy")
    return "\n".join(lines)


def render_json(data: dict) -> str:
    last = {
        key: {
            "at": entry.get("at"),
            "candle_open": entry.get("candle_open"),
            "symbol": entry.get("symbol"),
            "verdict": entry.get("verdict"),
            "validation": entry.get("validation"),
            "ack": entry.get("ack"),
            "signal_id": entry.get("signal_id"),
        }
        for key, entry in data["last_by_symbol"].items()
    }
    payload = {
        "at": data["at"].isoformat(),
        "level": data["level"],
        "cycle": data["cycle_start"].isoformat(),
        "next": data["next"].isoformat(),
        "delay": data["delay"],
        "db_ok": data["db_ok"],
        "audit_ok": data["audit_ok"],
        "db_errors": data.get("db_errors", 0),
        "opencode": data["opencode"],
        "services": {
            service: {
                "state": data["services"].get(service, {}).get("State"),
                "health": data["services"].get(service, {}).get("Health"),
                "ok": service in data["service_ok"],
            }
            for service, _ in SERVICE_LABELS
        },
        "contracts": data["contracts"],
        "signals": {
            "active": data["active_tickers"],
            "done": sorted(data["done"]),
            "analyzing": data["analyzing"],
            "missing": data["missing"],
            "last": last,
        },
        "money": data["money"],
        "loans": data["loans"],
        "orders": data["orders"],
        "flows": data["flows"],
        "issues": [{"level": kind, "text": text} for kind, text in data["issues"]],
        "exit": 2 if data["level"] == "crit" else 1 if data["level"] == "warn" else 0,
    }
    return json.dumps(payload, indent=2, default=str)


def demo_data() -> dict:
    real_now = datetime.now(timezone.utc)
    candle = current_candle_open(real_now)
    cycle = candle + CANDLE + timedelta(seconds=DEFAULT_DELAY)
    now = cycle + timedelta(minutes=10)
    nxt = next_cycle(now, DEFAULT_DELAY)
    services = {name: {"Service": name, "State": "running", "Health": "healthy"} for name, _ in SERVICE_LABELS}
    contracts = [
        {"ticker": "SOL/USDT", "has_active": True, "statuses": "active", "init_fund": "100", "available_fund": "101.75", "count": 1, "updated_at": now.isoformat()},
        {"ticker": "XRP/USDT", "has_active": True, "statuses": "active", "init_fund": "50", "available_fund": "50.00", "count": 1, "updated_at": now.isoformat()},
        {"ticker": "ETH/USDT", "has_active": True, "statuses": "active", "init_fund": "200", "available_fund": "198.40", "count": 1, "updated_at": now.isoformat()},
        {"ticker": "BTC/USDT", "has_active": False, "statuses": "closing", "init_fund": "500", "available_fund": "500.00", "count": 1, "updated_at": now.isoformat()},
    ]
    last = {
        "SOL-USDT": {"at": (now - timedelta(minutes=4)).isoformat(), "verdict": "SHORT", "validation": "valid", "ack": "accepted=true"},
        "ETH-USDT": {"at": (now - timedelta(hours=4)).isoformat(), "verdict": "NO-TRADE", "validation": "no-trade", "ack": None},
    }
    return {
        "at": now,
        "candle": candle,
        "next": nxt,
        "cycle_start": cycle,
        "delay": DEFAULT_DELAY,
        "services": services,
        "service_ok": set(services),
        "contracts": contracts,
        "total_contracts": sum(c["count"] for c in contracts),
        "active_tickers": ["SOL-USDT", "XRP-USDT", "ETH-USDT"],
        "last_by_symbol": last,
        "done": {"SOL-USDT", "ETH-USDT"},
        "analyzing": ["XRP-USDT"],
        "missing": [],
        "money": {"liabilities": "1240.50", "allocated": "1100.00", "available": "980.20", "fees": "12.40", "borrow_interest": "0.83"},
        "loans": {
            "currencies": [["SOL", "0.001121", "0.001121", "0.000118", "1", (now - timedelta(hours=3)).isoformat()]],
            "unrepaid_closed": "0", "open_positions": "0", "repay_pending": "0",
        },
        "orders": {"pending": 0, "open": 1, "filled_today": 4, "failed": 0, "pnl_today": "12.40", "win_rate": 66.7, "old_open": 0},
        "flows": {"inflight": 2, "inflight_total": "12.50", "oldest": (now - timedelta(minutes=18)).isoformat(), "expired": 0, "failed": 1, "awaiting": 0, "unswept": 0, "dep_failed": 0},
        "stuck": [],
        "audit": [],
        "opencode": "up",
        "db_ok": True,
        "audit_ok": True,
        "db_errors": 0,
        "issues": [("warn", "XRP/USDT analyzing")],
        "level": "warn",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Read-only CoinBot status dashboard")
    parser.add_argument("--json", action="store_true", help="machine-readable output")
    parser.add_argument("--no-color", action="store_true", help="disable ANSI color")
    parser.add_argument("--plain", action="store_true", help="no boxes, no color")
    parser.add_argument("--box", action="store_true", help="force boxed layout")
    parser.add_argument("--watch", type=int, default=0, metavar="SECS", help="refresh every SECS seconds")
    parser.add_argument("--width", type=int, default=0, help="override terminal width")
    parser.add_argument("--demo", action="store_true", help="render sample data")
    args = parser.parse_args()

    if not args.demo and not ENV_PATH.exists():
        print(f"no .env in {ROOT}", file=sys.stderr)
        return 2

    env = load_env()
    raw_delay = env.get("SIGNAL_GATEWAY_CYCLE_DELAY_SECS", str(DEFAULT_DELAY))
    try:
        delay = int(raw_delay)
    except ValueError:
        delay = DEFAULT_DELAY
    if delay < 0 or delay > 3600:
        delay = DEFAULT_DELAY

    def once() -> tuple[str, int]:
        if args.demo:
            data = demo_data()
        else:
            DB_ERRORS.clear()
            db_ok = psql(env, "SELECT 1") is not None
            if db_ok:
                contracts = fetch_contracts(env)
                money = fetch_money(env)
                loans = fetch_loans(env)
                orders = fetch_orders(env)
                flows = fetch_flows(env)
                stuck = fetch_stuck(env)
            else:
                contracts = []
                money, loans, orders, flows = default_db_values()
                stuck = []
            audit, audit_ok = read_audit(env)
            services = fetch_services(env)
            data = build(
                env, services, contracts, money, loans, orders, flows, stuck,
                audit, delay, db_ok, audit_ok,
                len(DB_ERRORS) if db_ok else 0,
            )
        if args.json:
            return render_json(data), 2 if data["level"] == "crit" else 1 if data["level"] == "warn" else 0
        tty = sys.stdout.isatty()
        plain = args.plain or (not tty and not args.box) or os.environ.get("TERM") == "dumb"
        color = (
            tty
            and not args.no_color
            and not args.plain
            and "NO_COLOR" not in os.environ
            and os.environ.get("TERM") != "dumb"
        )
        width = args.width or shutil.get_terminal_size((100, 24)).columns
        width = max(60, min(width, 120))
        text = render_text(data, width, color, plain)
        return text, 2 if data["level"] == "crit" else 1 if data["level"] == "warn" else 0

    if args.watch and sys.stdout.isatty() and not args.json:
        import time

        try:
            while True:
                text, _ = once()
                sys.stdout.write("\x1b[2J\x1b[H" + text + "\n")
                sys.stdout.flush()
                time.sleep(max(2, args.watch))
        except KeyboardInterrupt:
            return 0
    text, code = once()
    print(text)
    return code


if __name__ == "__main__":
    sys.exit(main())
