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
