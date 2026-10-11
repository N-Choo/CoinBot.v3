import json
import logging
import os
import time

import redis

from analyzer.config import Config
from analyzer.fetchers.kucoin import KuCoinFetchError, fetch_klines
from analyzer.indicators.rsi import calculate_rsi
from analyzer.publisher.redis_pub import publish_signals
from analyzer.signals.generator import generate_signals


def _load_dotenv(path: str = ".env") -> None:
    """Load key=value pairs from a .env file into os.environ."""
    try:
        with open(path) as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if "=" not in line:
                    continue
                key, _, val = line.partition("=")
                key = key.strip()
                val = val.split("#")[0].strip().strip("\"'")
                if key and key not in os.environ:
                    os.environ[key] = val
    except FileNotFoundError:
        pass


_load_dotenv()


logger = logging.getLogger(__name__)

RESET = "\x1b[0m"
DIM = "\x1b[2m"
RED = "\x1b[31m"
GREEN = "\x1b[32m"
YELLOW = "\x1b[33m"
CYAN = "\x1b[36m"


class _TimeFormatter(logging.Formatter):
    """Prefix each record with a local `HH:MM:SS`; all other layout lives in the message."""

    def __init__(self) -> None:
        super().__init__(fmt="%(message)s")

    def format(self, record: logging.LogRecord) -> str:
        return f"{self.formatTime(record, '%H:%M:%S')}  {super().format(record)}"


def configure_logging() -> None:
    handler = logging.StreamHandler()
    handler.setFormatter(_TimeFormatter())
    root = logging.getLogger()
    root.handlers.clear()
    root.addHandler(handler)
    root.setLevel(logging.INFO)


def analyze_ticker(
    ticker: str,
    cfg: Config,
    redis_client: redis.Redis,
    branch: str = "└",
) -> None:
    prefix = f"{DIM}{branch}{RESET}"
    child = "   " if branch == "└" else "│  "
    t0 = time.monotonic()

    try:
        symbol = ticker.replace("/", "-")
        df = fetch_klines(
            symbol, timeframe=cfg.kucoin_timeframe, base_url=cfg.kuCoin_base_url
        )
    except KuCoinFetchError as e:
        logger.error(
            f"{prefix} {CYAN}{ticker:<10}{RESET}  {RED}ERROR{RESET}  fetch failed — {e}"
        )
        return
    except Exception as e:
        logger.exception(
            f"{prefix} {CYAN}{ticker:<10}{RESET}  {RED}ERROR{RESET}  unexpected error — {e}"
        )
        return

    if len(df) < cfg.rsi_period + 1:
        logger.warning(
            f"{prefix} {CYAN}{ticker:<10}{RESET}  {YELLOW}WARN{RESET}  "
            f"only {len(df)} candles (need >{cfg.rsi_period})"
        )
        return

    signals = generate_signals(
        df,
        ticker=ticker,
        rsi_period=cfg.rsi_period,
        rsi_order=cfg.rsi_divergence_order,
        price_diff_threshold=cfg.price_diff_threshold,
        sma_enabled=cfg.sma_enabled,
        sma_period=cfg.sma_period,
        atr_period=cfg.atr_period,
        atr_sl_multiplier=cfg.atr_sl_multiplier,
        atr_tp_multiplier=cfg.atr_tp_multiplier,
        atr_pct_high=cfg.atr_pct_high,
        atr_high_adjust=cfg.atr_high_adjust,
        atr_sl_max_pct=cfg.atr_sl_max_pct,
    )

    elapsed = (time.monotonic() - t0) * 1000

    latest_close = df["Close"].iloc[-1]
    latest_rsi = calculate_rsi(df["Close"], period=cfg.rsi_period).iloc[-1]

    buy_n = sum(1 for s in signals if s.action == "buy")
    sell_n = sum(1 for s in signals if s.action == "sell")

    logger.info(
        f"{prefix} {CYAN}{ticker:<10}{RESET}  "
        f"{('$' + format(latest_close, ',.2f')):>12}   RSI {latest_rsi:>4.1f}  {len(df)}c  "
        f"{GREEN}{buy_n}▲{RESET} {RED}{sell_n}▼{RESET}  {round(elapsed)}ms"
    )
    for s in signals:
        arrow, color = ("▲", GREEN) if s.action == "buy" else ("▼", RED)
        logger.info(
            f"{DIM}{child}└{RESET} {color}{arrow} {s.action.upper()}{RESET}  "
            f"conf {s.confidence:.2f}  @${s.entry_price:,.2f}  "
            f"SL ${s.sl_price:,.2f}  TP ${s.tp_price:,.2f}  {s.reason}"
        )

    publish_signals(redis_client, signals, channel=cfg.result_channel)


def main() -> None:
    configure_logging()
    cfg = Config()

    r = redis.Redis(
        host=cfg.redis_host,
        port=cfg.redis_port,
        db=cfg.redis_db,
        decode_responses=True,
    )
    pubsub = r.pubsub()
    pubsub.subscribe(cfg.ticker_channel)

    logger.info(
        f"ANALYZER  started · redis {cfg.redis_host}:{cfg.redis_port} · "
        f"kuCoin {cfg.kucoin_timeframe} · rsi {cfg.rsi_period}/{cfg.rsi_divergence_order} · "
        f"sma {'on' if cfg.sma_enabled else 'off'} · "
        f"atr {cfg.atr_period} (sl {cfg.atr_sl_multiplier:.1f}x tp {cfg.atr_tp_multiplier:.1f}x)"
    )

    for message in pubsub.listen():
        if message["type"] != "message":
            continue

        try:
            tickers = json.loads(message["data"])
        except json.JSONDecodeError:
            logger.warning(f"{YELLOW}WARN{RESET}  invalid JSON on {cfg.ticker_channel}")
            continue

        logger.info(f"ANALYZER  batch · {len(tickers)} tickers")

        last = len(tickers) - 1
        for i, ticker in enumerate(tickers):
            time.sleep(0.1)
            analyze_ticker(ticker, cfg, r, branch="└" if i == last else "├")


if __name__ == "__main__":
    main()
