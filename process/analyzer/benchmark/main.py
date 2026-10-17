"""Benchmark the analyzer by replaying candles through a trade-engine sim."""

from __future__ import annotations

import logging
import os
from dataclasses import asdict
from typing import Any

import pandas as pd
from pandas import Timestamp

from analyzer.benchmark.engine import SimEngine
from analyzer.config import Config
from analyzer.fetchers.kucoin import KuCoinFetchError, fetch_klines
from analyzer.signals.generator import generate_signals

logger = logging.getLogger(__name__)


def _load_dotenv(path: str = ".env") -> None:
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


def _date_str(df: pd.DataFrame, i: int) -> str:
    ts = df["Timestamp"].iloc[i] if "Timestamp" in df.columns else df.index[i]
    from datetime import datetime as dt

    if isinstance(ts, (dt, Timestamp)):
        return ts.strftime("%Y-%m-%d")
    return str(ts)


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

    for i in range(start, len(df)):
        row = df.iloc[i]
        engine.on_candle(
            i, float(row["High"]), float(row["Low"]), _date_str(df, i)
        )

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
        if not signals:
            continue

        sig = signals[0]
        if sig.tp_price <= 0 or sig.sl_price <= 0:
            logger.warning(
                "skip %s signal on %s - non-positive tp/sl (%.4f/%.4f)",
                ticker,
                _date_str(df, i),
                sig.tp_price,
                sig.sl_price,
            )
            continue

        engine.on_signal(
            sig.action,
            sig.entry_price,
            sig.tp_price,
            sig.sl_price,
            i,
            _date_str(df, i),
        )

    trades = [asdict(t) for t in engine.closed]
    open_positions = [asdict(p) for p in engine.positions]
    return trades, open_positions, _summarize(ticker, engine, df)


def _summarize(
    ticker: str, engine: SimEngine, df: pd.DataFrame
) -> dict[str, Any]:
    last_close = float(df["Close"].iloc[-1])
    unrealized = 0.0
    for pos in engine.positions:
        ret = (last_close - pos.entry_price) / pos.entry_price
        if pos.side == "sell":
            ret = -ret
        unrealized += pos.E * ret

    reserved = sum(p.E for p in engine.positions)
    realized = sum(t.pnl for t in engine.closed)
    total = len(engine.closed)
    wins = sum(1 for t in engine.closed if t.pnl > 0)

    return {
        "ticker": ticker,
        "trades": total,
        "wins": wins,
        "win_rate": (wins / total * 100) if total else 0.0,
        "realized_pnl": realized,
        "unrealized_pnl": unrealized,
        "open_count": len(engine.positions),
        "init_fund": engine.init_fund,
        "available_fund": engine.available_fund,
        "equity": engine.available_fund + reserved + unrealized,
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


def main() -> None:
    _load_dotenv()
    cfg = Config()

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s  %(levelname)-7s  %(message)s",
        datefmt="%H:%M:%S",
    )

    logger.info(
        "benchmark started  days=%d  init_fund=%.0f  fee=%.4f  tickers=%s",
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

    # Warm-up floor plus a small margin so at least two candles are replayed.
    min_len = (
        _warmup(
            cfg.rsi_period, cfg.atr_period, cfg.sma_period, cfg.sma_enabled
        )
        + 2
    )

    for ticker in cfg.benchmark_tickers:
        symbol = ticker.replace("/", "-")
        logger.info("fetching %s klines...", symbol)
        try:
            df = fetch_klines(
                symbol,
                timeframe=cfg.kucoin_timeframe,
                base_url=cfg.kuCoin_base_url,
            )
        except KuCoinFetchError as e:
            logger.error("  skip %s - fetch error: %s", symbol, e)
            continue
        except Exception as e:
            logger.exception("  skip %s - unexpected fetch error: %s", symbol, e)
            continue

        if len(df) < min_len:
            logger.warning(
                "  skip %s - only %d candles (need >= %d)",
                symbol,
                len(df),
                min_len,
            )
            continue

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
        )

        print_ledger(ticker, trades, open_positions, summary)

    logger.info("done")


if __name__ == "__main__":
    main()
