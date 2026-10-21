"""Benchmark the analyzer by replaying candles through a trade-engine sim."""

from __future__ import annotations

import argparse
import logging
import os
from dataclasses import asdict
from datetime import datetime
from pathlib import Path
from typing import Any

import pandas as pd

from analyzer.benchmark.engine import SimEngine
from analyzer.benchmark.replay import (
    ReplayRecorder,
    account_state,
    build_ticker_replay,
    config_dict,
    date_str as _date_str,
)
from analyzer.benchmark.report import write_report
from analyzer.config import Config
from analyzer.fetchers.kucoin import MAX_KLINES, KuCoinFetchError, fetch_klines
from analyzer.signals.generator import generate_signals

logger = logging.getLogger(__name__)


def _find_dotenv(start: Path | None = None) -> Path | None:
    """Find the nearest ``.env``, walking up from ``start`` to the repo root."""
    current = (start or Path.cwd()).resolve()
    for directory in (current, *current.parents):
        candidate = directory / ".env"
        if candidate.is_file():
            return candidate
        if (directory / ".git").exists():
            break
    return None


def _load_dotenv(path: str | Path | None = None) -> None:
    """Load the nearest ``.env``; exported environment variables win."""
    env_path = Path(path) if path is not None else _find_dotenv()
    if env_path is None:
        return
    try:
        with env_path.open() as f:
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


def _warmup(
    rsi_period: int,
    atr_period: int,
    sma_period: int,
    sma_enabled: bool,
) -> int:
    return max(rsi_period, atr_period, sma_period if sma_enabled else 0)


def run_benchmark(
    df: pd.DataFrame,
    ticker: str,
    *,
    rsi_period: int = 14,
    rsi_order: int = 5,
    price_diff_threshold: float = 0.05,
    sma_enabled: bool = False,
    sma_period: int = 20,
    atr_period: int = 14,
    atr_sl_multiplier: float = 2.0,
    atr_tp_multiplier: float = 3.0,
    atr_pct_high: float = 5.0,
    atr_high_adjust: float = 1.5,
    atr_sl_max_pct: float = 0.0,
    init_fund: float = 1000.0,
    fee_rate: float = 0.001,
    benchmark_days: int = 90,
    recorder: ReplayRecorder | None = None,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, Any]]:
    """Replay ``df`` candle-by-candle through a trade-engine simulation.

    Earlier candles warm up the indicators; the last ``benchmark_days`` candles
    are replayed (all post-warm-up candles are replayed when fewer than
    ``benchmark_days`` are available). ``generate_signals`` is evaluated on the
    slice up to each candle (causal), and its signal is fed to a `SimEngine`.
    """
    warmup = _warmup(rsi_period, atr_period, sma_period, sma_enabled)
    start = max(warmup, len(df) - benchmark_days)

    engine = SimEngine(init_fund=init_fund, fee_rate=fee_rate)

    if recorder is not None:
        recorder.begin(start, df)

    for i in range(start, len(df)):
        row = df.iloc[i]
        before_closed = len(engine.closed)
        engine.on_candle(
            i, float(row["High"]), float(row["Low"]), _date_str(df, i)
        )
        exits = engine.closed[before_closed:]

        signal = None
        opened = False
        rejected = None

        signals = generate_signals(
            df.iloc[: i + 1],
            ticker=ticker,
            rsi_period=rsi_period,
            rsi_order=rsi_order,
            price_diff_threshold=price_diff_threshold,
            sma_enabled=sma_enabled,
            sma_period=sma_period,
            atr_period=atr_period,
            atr_sl_multiplier=atr_sl_multiplier,
            atr_tp_multiplier=atr_tp_multiplier,
            atr_pct_high=atr_pct_high,
            atr_high_adjust=atr_high_adjust,
            atr_sl_max_pct=atr_sl_max_pct,
        )
        if signals:
            sig = signals[0]
            signal = sig
            if sig.tp_price <= 0 or sig.sl_price <= 0:
                rejected = "invalid_tp_sl"
                logger.warning(
                    "skip %s signal on %s - non-positive tp/sl (%.4f/%.4f)",
                    ticker,
                    _date_str(df, i),
                    sig.tp_price,
                    sig.sl_price,
                )
            else:
                opened = engine.on_signal(
                    sig.action,
                    sig.entry_price,
                    sig.tp_price,
                    sig.sl_price,
                    i,
                    _date_str(df, i),
                )

        if recorder is not None:
            recorder.record(
                i,
                float(row["Close"]),
                signal,
                opened,
                rejected,
                exits,
                engine,
            )

    if recorder is not None:
        recorder.finish(engine)

    trades = [asdict(t) for t in engine.closed]
    open_positions = [asdict(p) for p in engine.positions]
    return trades, open_positions, _summarize(ticker, engine, df)


def _summarize(
    ticker: str, engine: SimEngine, df: pd.DataFrame
) -> dict[str, Any]:
    last_close = float(df["Close"].iloc[-1])
    state = account_state(engine, last_close)
    total = len(engine.closed)
    wins = sum(1 for t in engine.closed if t.pnl > 0)

    return {
        "ticker": ticker,
        "trades": total,
        "wins": wins,
        "win_rate": (wins / total * 100) if total else 0.0,
        "init_fund": engine.init_fund,
        **state,
    }


def _fmt_price(p: float | None) -> str:
    if p is None:
        return "-"
    if abs(p) >= 1000:
        return f"${p:,.0f}"
    if abs(p) >= 1:
        return f"${p:,.2f}"
    if abs(p) >= 0.01:
        return f"${p:,.4f}"
    return f"${p:,.6f}"


def print_ledger(
    ticker: str,
    trades: list[dict[str, Any]],
    open_positions: list[dict[str, Any]],
    summary: dict[str, Any],
) -> None:
    """Print one ticker's round-trip ledger and account summary."""
    rule = "\u2500" * 66
    wall = "\u2550" * 66
    print()
    print(f"  {ticker}")
    print(rule)

    if not trades and not open_positions:
        print("  (no trades)")
    else:
        print(
            f"  {'entry':<10} {'dir':<4} {'entry$':>10} "
            f"{'exit':<10} {'exit$':>10} {'pnl%':>7} {'pnl$':>8}  why"
        )
        for t in trades:
            print(
                f"  {t['entry_date']:<10} {t['side'].upper()[:4]:<4} "
                f"{_fmt_price(t['entry_price']):>10} "
                f"{t['exit_date']:<10} {_fmt_price(t['exit_price']):>10} "
                f"{t['pnl_pct']:>+6.1f}% {t['pnl']:>+8.2f}  {t['reason']}"
            )
        for p in open_positions:
            print(
                f"  {p['entry_date']:<10} {p['side'].upper()[:4]:<4} "
                f"{_fmt_price(p['entry_price']):>10} "
                f"{'open':<10} {'-':>10} "
                f"{'-':>7} {'-':>8}  open"
            )

    print(rule)
    print(
        f"  open: {summary['open_count']}  |  "
        f"trades {summary['trades']}  "
        f"wins {summary['wins']}/{summary['trades']} "
        f"({summary['win_rate']:.0f}%)  "
        f"realized {summary['realized_pnl']:+.2f}  "
        f"equity {summary['equity']:.2f} "
        f"(init {summary['init_fund']:.0f})"
    )
    print(wall)
    print()


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Replay analyzer benchmark candles."
    )
    parser.add_argument(
        "--web",
        action="store_true",
        help="write a self-contained HTML replay report instead of the text ledger",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="output path for --web (default: benchmark/out/benchmark.html)",
    )
    args = parser.parse_args(argv)
    if args.out is not None and not args.web:
        parser.error("--out requires --web")
    return args


def main(web: bool = False, out: Path | None = None) -> None:
    _load_dotenv()
    cfg = Config()

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s  %(levelname)-7s  %(message)s",
        datefmt="%H:%M:%S",
    )

    logger.info(
        "benchmark started  timeframe=%s  days=%d  init_fund=%.0f  fee=%.4f  tickers=%s",
        cfg.kucoin_timeframe,
        cfg.benchmark_days,
        cfg.benchmark_init_fund,
        cfg.benchmark_fee_rate,
        ", ".join(cfg.benchmark_tickers),
    )
    logger.info(
        "rsi period=%d  order=%d  sma=%s  atr=%d sl=%.1fx tp=%.1fx high=%d%% adj=%.1fx",
        cfg.rsi_period,
        cfg.rsi_divergence_order,
        "on" if cfg.sma_enabled else "off",
        cfg.atr_period,
        cfg.atr_sl_multiplier,
        cfg.atr_tp_multiplier,
        int(cfg.atr_pct_high),
        cfg.atr_high_adjust,
    )

    replay_tickers: list[dict[str, Any]] = []
    failures: list[dict[str, str]] = []

    # Warm-up floor plus a small margin so at least two candles are replayed.
    warmup = _warmup(
        cfg.rsi_period, cfg.atr_period, cfg.sma_period, cfg.sma_enabled
    )
    min_len = warmup + 2

    # Ask for the warm-up plus the full replay window, with a small buffer.
    fetch_limit = warmup + cfg.benchmark_days + 10
    if fetch_limit > MAX_KLINES:
        logger.warning(
            "requested %d candles; KuCoin returns at most %d",
            fetch_limit,
            MAX_KLINES,
        )
        fetch_limit = MAX_KLINES

    for ticker in cfg.benchmark_tickers:
        symbol = ticker.replace("/", "-")
        logger.info("fetching %s klines (%d candles)...", symbol, fetch_limit)
        try:
            df = fetch_klines(
                symbol,
                timeframe=cfg.kucoin_timeframe,
                base_url=cfg.kuCoin_base_url,
                limit=fetch_limit,
            )
        except KuCoinFetchError as e:
            logger.error("  skip %s - fetch error: %s", symbol, e)
            if web:
                failures.append({"symbol": symbol, "error": str(e)})
            continue
        except Exception as e:
            logger.exception("  skip %s - unexpected fetch error: %s", symbol, e)
            if web:
                failures.append(
                    {"symbol": symbol, "error": f"{type(e).__name__}: {e}"}
                )
            continue

        if len(df) < min_len:
            logger.warning(
                "  skip %s - only %d candles (need >= %d)",
                symbol,
                len(df),
                min_len,
            )
            if web:
                failures.append(
                    {
                        "symbol": symbol,
                        "error": f"only {len(df)} candles (need >= {min_len})",
                    }
                )
            continue

        recorder = ReplayRecorder() if web else None
        trades, open_positions, summary = run_benchmark(
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
            init_fund=cfg.benchmark_init_fund,
            fee_rate=cfg.benchmark_fee_rate,
            benchmark_days=cfg.benchmark_days,
            recorder=recorder,
        )

        if web:
            assert recorder is not None and recorder.start is not None
            replay_tickers.append(
                build_ticker_replay(
                    df,
                    ticker=ticker,
                    start=recorder.start,
                    recorder=recorder,
                    trades=trades,
                    open_positions=open_positions,
                    summary=summary,
                    config=config_dict(cfg),
                    warmup=warmup,
                )
            )
        else:
            print_ledger(ticker, trades, open_positions, summary)

    if web:
        path = out or Path(__file__).resolve().parent / "out" / "benchmark.html"
        write_report(
            {
                "generated_at": datetime.now().isoformat(timespec="seconds"),
                "tickers": replay_tickers,
                "failures": failures,
            },
            path,
        )
        logger.info("report written to %s", path.resolve())

    logger.info("done")


if __name__ == "__main__":
    args = _parse_args()
    main(web=args.web, out=args.out)
