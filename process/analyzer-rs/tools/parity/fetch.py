#!/usr/bin/env python3
"""Fetch KuCoin candles into a CSV fixture. Network is used only here."""
import argparse
import sys
from pathlib import Path

import pandas as pd

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT))
from analyzer.fetchers.kucoin import fetch_klines


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--symbol", required=True)
    parser.add_argument("--timeframe", default="1day")
    parser.add_argument("--limit", type=int, default=400)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    frame = fetch_klines(args.symbol, timeframe=args.timeframe, limit=args.limit)
    # KuCoin's fetcher returns a pandas Timestamp (already representing the correct instant).
    # Persist epoch milliseconds so emit.py never depends on the host timezone.
    frame = frame.assign(open_time_ms=frame["Timestamp"].astype("int64") // 1_000_000)
    frame = frame[["open_time_ms", "Open", "High", "Low", "Close", "Volume"]]
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    frame.to_csv(args.out, index=False)
    print(f"wrote {len(frame)} candles to {args.out}")


if __name__ == "__main__":
    main()
