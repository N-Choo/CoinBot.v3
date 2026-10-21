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
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    frame.to_csv(args.out, index=False)
    print(f"wrote {len(frame)} candles to {args.out}")


if __name__ == "__main__":
    main()
