#!/usr/bin/env python3
"""Deterministic synthetic OHLCV CSV for offline parity tests."""
import argparse
import csv
import math
import random


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--n", type=int, default=400)
    args = parser.parse_args()

    rng = random.Random(42)
    rows = []
    start_ms = 1_700_000_000_000
    for i in range(args.n):
        close = 100.0 + 10.0 * math.sin(i / 6.0) + 2.0 * math.sin(i / 17.0) + rng.uniform(-0.2, 0.2)
        open_ = close * (1.0 + rng.uniform(-0.01, 0.01))
        high = max(open_, close) * (1.0 + abs(rng.uniform(0.0, 0.008)))
        low = min(open_, close) * (1.0 - abs(rng.uniform(0.0, 0.008)))
        rows.append((start_ms + i * 86_400_000, open_, high, low, close, rng.uniform(1.0, 10.0)))

    with open(args.out, "w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["open_time_ms", "open", "high", "low", "close", "volume"])
        writer.writerows(rows)
    print(f"wrote {len(rows)} synthetic candles to {args.out}")


if __name__ == "__main__":
    main()
