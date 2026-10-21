#!/usr/bin/env python3
"""Emit golden indicators, divergences, and per-window signals from the Python analyzer."""
import argparse
import hashlib
import json
import math
import os
import sys
from pathlib import Path

# scipy's find_peaks distance tie order comes from numpy's argsort. The SIMD argsort path is
# CPU-dependent and not portable; pin the scalar path so tie behavior is reproducible and
# matches the Rust port of NumPy's scalar argsort ordering.
os.environ.setdefault(
    "NPY_DISABLE_CPU_FEATURES",
    "AVX512F AVX512CD AVX512_SKX AVX512_CLX AVX512_CNL AVX512_ICL AVX2 FMA3 F16C",
)

import pandas as pd

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
from analyzer.indicators.atr import calculate_atr
from analyzer.indicators.rsi import calculate_rsi, detect_rsi_divergences
from analyzer.indicators.sma import calculate_sma
from analyzer.signals.generator import generate_signals

KIND = {
    "regular_bullish_divergence": "regular_bullish",
    "regular_bearish_divergence": "regular_bearish",
    "hidden_bullish_divergence": "hidden_bullish",
    "hidden_bearish_divergence": "hidden_bearish",
}


def number(value):
    if value is None:
        return None
    try:
        result = float(value)
    except (TypeError, ValueError):
        return None
    if not math.isfinite(result):
        return None
    return repr(result)


def column(frame, *names):
    for name in names:
        if name in frame.columns:
            return frame[name]
    raise KeyError(f"none of {names} in {list(frame.columns)}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", required=True)
    parser.add_argument("--symbol", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--window", type=int, default=100)
    parser.add_argument("--rsi-period", type=int, default=14)
    parser.add_argument("--rsi-order", type=int, default=5)
    parser.add_argument("--price-diff-threshold", type=float, default=0.05)
    parser.add_argument("--sma-enabled", action="store_true")
    parser.add_argument("--sma-period", type=int, default=20)
    parser.add_argument("--atr-period", type=int, default=14)
    parser.add_argument("--atr-sl-multiplier", type=float, default=2.0)
    parser.add_argument("--atr-tp-multiplier", type=float, default=3.0)
    parser.add_argument("--atr-pct-high", type=float, default=5.0)
    parser.add_argument("--atr-high-adjust", type=float, default=1.5)
    parser.add_argument("--atr-sl-max-pct", type=float, default=0.0)
    args = parser.parse_args()

    raw = Path(args.data).read_bytes()
    frame = pd.read_csv(args.data)
    if "Timestamp" in frame.columns:
        frame["Timestamp"] = pd.to_datetime(frame["Timestamp"])
        open_ms = [int(value.timestamp() * 1000) for value in frame["Timestamp"]]
    else:
        open_ms = [int(value) for value in frame["open_time_ms"]]

    frame = frame.rename(
        columns={
            "open": "Open",
            "high": "High",
            "low": "Low",
            "close": "Close",
            "volume": "Volume",
        }
    )

    open_ = column(frame, "Open", "open")
    high = column(frame, "High", "high")
    low = column(frame, "Low", "low")
    close = column(frame, "Close", "close")
    volume = column(frame, "Volume", "volume") if ("Volume" in frame.columns or "volume" in frame.columns) else None

    candles = []
    for i in range(len(frame)):
        candles.append(
            {
                "open_time_ms": open_ms[i],
                "open": number(open_.iloc[i]),
                "high": number(high.iloc[i]),
                "low": number(low.iloc[i]),
                "close": number(close.iloc[i]),
                "volume": number(volume.iloc[i]) if volume is not None else "1",
            }
        )

    rsi = calculate_rsi(frame["Close"], period=args.rsi_period)
    sma = calculate_sma(frame["Close"], period=args.sma_period)
    atr = calculate_atr(frame, period=args.atr_period)
    divergences = detect_rsi_divergences(frame, rsi_period=args.rsi_period, order=args.rsi_order)

    signals = []
    for end in range(args.window - 1, len(frame)):
        window = frame.iloc[end + 1 - args.window : end + 1]
        produced = generate_signals(
            window,
            ticker=args.symbol,
            rsi_period=args.rsi_period,
            rsi_order=args.rsi_order,
            price_diff_threshold=args.price_diff_threshold,
            sma_enabled=args.sma_enabled,
            sma_period=args.sma_period,
            atr_period=args.atr_period,
            atr_sl_multiplier=args.atr_sl_multiplier,
            atr_tp_multiplier=args.atr_tp_multiplier,
            atr_pct_high=args.atr_pct_high,
            atr_high_adjust=args.atr_high_adjust,
            atr_sl_max_pct=args.atr_sl_max_pct,
        )
        if not produced:
            signals.append({"window_end": end, "signal": None})
            continue
        signal = produced[0]
        absolute_divergence_index = end + 1 - args.window + signal.divergence_idx
        signals.append(
            {
                "window_end": end,
                "signal": {
                    "action": "buy" if signal.action == "buy" else "sell",
                    "reason": signal.reason,
                    "kind": KIND[signal.reason],
                    "confidence": number(signal.confidence),
                    "entry": number(signal.entry_price),
                    "sl": number(signal.sl_price),
                    "tp": number(signal.tp_price),
                    "divergence_index": signal.divergence_idx,
                    "divergence_open_time_ms": open_ms[absolute_divergence_index],
                    "divergence_age_candles": args.window - 1 - signal.divergence_idx,
                    "signal_candle_open_time_ms": open_ms[end],
                },
            }
        )

    payload = {
        "symbol": args.symbol,
        "window": args.window,
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "versions": {
            "pandas": pd.__version__,
            "scipy": __import__("scipy").__version__,
            "numpy": __import__("numpy").__version__,
        },
        "config": {
            "rsi_period": args.rsi_period,
            "rsi_order": args.rsi_order,
            "price_diff_threshold": repr(args.price_diff_threshold),
            "sma_enabled": args.sma_enabled,
            "sma_period": args.sma_period,
            "atr_period": args.atr_period,
            "atr_sl_multiplier": repr(args.atr_sl_multiplier),
            "atr_tp_multiplier": repr(args.atr_tp_multiplier),
            "atr_pct_high": repr(args.atr_pct_high),
            "atr_high_adjust": repr(args.atr_high_adjust),
            "atr_sl_max_pct": repr(args.atr_sl_max_pct),
            "window": args.window,
        },
        "candles": candles,
        "indicators": {
            "rsi": [number(value) for value in rsi],
            "sma": [number(value) for value in sma],
            "atr": [number(value) for value in atr],
        },
        "divergences": {
            "regular_bullish": [int(value) for value in divergences["Reg_Bullish_Div"]],
            "regular_bearish": [int(value) for value in divergences["Reg_Bearish_Div"]],
            "hidden_bullish": [int(value) for value in divergences["Hid_Bullish_Div"]],
            "hidden_bearish": [int(value) for value in divergences["Hid_Bearish_Div"]],
        },
        "signals": signals,
    }

    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(args.out).write_text(json.dumps(payload, indent=2), encoding="utf-8")
    positives = sum(1 for item in signals if item["signal"] is not None)
    print(f"wrote {args.out}: {len(candles)} candles, {positives} signals")


if __name__ == "__main__":
    main()
