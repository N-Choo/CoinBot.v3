"""Benchmark RSI divergence signals against forward price movement."""

from __future__ import annotations

import logging
import os
from typing import Any

import pandas as pd
from pandas import Timestamp

from analyzer.config import Config
from analyzer.fetchers.kucoin import KuCoinFetchError, fetch_klines
from analyzer.indicators.sma import calculate_sma
from analyzer.signals.generator import generate_signals




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


def _score_signal(
    action: str, entry: float, forward: float | None
) -> tuple[bool | None, float | None]:
    if forward is None:
        return None, None
    if action == "sell":
        pct = float(((entry - forward) / entry) * 100)
        return pct > 0, pct
    pct = float(((forward - entry) / entry) * 100)
    return pct > 0, pct


def _make_divergence_label(row_data: dict) -> str:
    if row_data.get("Reg_Bullish_Div"):
        return "regular_bullish"
    if row_data.get("Reg_Bearish_Div"):
        return "regular_bearish"
    if row_data.get("Hid_Bullish_Div"):
        return "hidden_bullish"
    if row_data.get("Hid_Bearish_Div"):
        return "hidden_bearish"
    return "unknown"


def _apply_sma_filter(
    close_series: pd.Series,
    idx: int,
    action: str,
    sma_period: int,
) -> bool:
    window = close_series.iloc[: idx + 1]
    if len(window) < sma_period:
        return True
    sma_val = calculate_sma(window, period=sma_period).iloc[-1]
    price = float(close_series.iloc[idx])
    if action == "buy":
        return bool(price > sma_val)
    return bool(price < sma_val)


def run_benchmark(
    df: pd.DataFrame,
    ticker: str,
    rsi_period: int = 14,
    rsi_order: int = 5,
    forward_days: int = 7,
    sma_enabled: bool = False,
    sma_period: int = 20,
    atr_period: int = 14,
    atr_sl_multiplier: float = 2.0,
    atr_tp_multiplier: float = 3.0,
    atr_pct_high: float = 5.0,
    atr_high_adjust: float = 1.5,
    atr_sl_max_pct: float = 0.0,
    allow_pyramiding: bool = False,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    from analyzer.indicators.rsi import detect_rsi_divergences

    signals: list[dict[str, Any]] = []
    buy_total = buy_hits = sell_total = sell_hits = 0
    last_div_idx: int | None = None
    last_exit_idx: int = -1

    div_columns = ["Reg_Bullish_Div", "Reg_Bearish_Div",
                   "Hid_Bullish_Div", "Hid_Bearish_Div"]

    for i in range(rsi_period, len(df) - forward_days):
        df_slice = df.iloc[: i + 1]
        div_df = detect_rsi_divergences(
            df_slice, rsi_period=rsi_period, order=rsi_order
        )

        # find the most recent divergence index
        found_idx = -1
        for j in range(len(div_df) - 1, -1, -1):
            if any(div_df.iloc[j][c] == 1 for c in div_columns):
                found_idx = j
                break

        if found_idx == -1:
            last_div_idx = None
            continue

        if found_idx == last_div_idx:
            continue
        last_div_idx = found_idx

        # no pyramiding: skip if previous position hasn't closed yet
        if not allow_pyramiding and i < last_exit_idx:
            continue

        # only call generate_signals when we have a new divergence
        result = generate_signals(
            df_slice,
            ticker=ticker,
            rsi_period=rsi_period,
            rsi_order=rsi_order,
            sma_enabled=sma_enabled,
            sma_period=sma_period,
            atr_period=atr_period,
            atr_sl_multiplier=atr_sl_multiplier,
            atr_tp_multiplier=atr_tp_multiplier,
            atr_pct_high=atr_pct_high,
            atr_high_adjust=atr_high_adjust,
            atr_sl_max_pct=atr_sl_max_pct,
        )
        if not result:
            continue
        sig = result[0]

        if "Timestamp" in df.columns:
            ts = df["Timestamp"].iloc[i]
        else:
            ts = df.index[i]
        from datetime import datetime as dt
        if isinstance(ts, (dt, Timestamp)):
            date_str = ts.strftime("%Y-%m-%d")
        else:
            date_str = str(ts)

        entry_price = sig.entry_price
        sl_price = sig.sl_price
        tp_price = sig.tp_price

        exit_price: float | None = None
        exit_reason = "expiry"
        exit_idx = len(df) - 1

        window = df.iloc[i + 1:]
        if len(window) > 0:
            if sig.action == "buy":
                for j in range(len(window)):
                    row = window.iloc[j]
                    if tp_price > 0 and float(row["High"]) >= tp_price:
                        exit_price = tp_price
                        exit_reason = "TP"
                        exit_idx = i + 1 + j
                        break
                    if sl_price > 0 and float(row["Low"]) <= sl_price:
                        exit_price = sl_price
                        exit_reason = "SL"
                        exit_idx = i + 1 + j
                        break
            else:
                for j in range(len(window)):
                    row = window.iloc[j]
                    if tp_price > 0 and float(row["Low"]) <= tp_price:
                        exit_price = tp_price
                        exit_reason = "TP"
                        exit_idx = i + 1 + j
                        break
                    if sl_price > 0 and float(row["High"]) >= sl_price:
                        exit_price = sl_price
                        exit_reason = "SL"
                        exit_idx = i + 1 + j
                        break

            if exit_price is None:
                exit_price = float(window["Close"].iloc[-1])
        else:
            exit_price = None

        hit, pct = _score_signal(sig.action, entry_price, exit_price)

        signals.append({
            "ticker": ticker,
            "date": date_str,
            "action": sig.action,
            "divergence": sig.reason,
            "entry_price": entry_price,
            "forward_price": exit_price,
            "pct": pct,
            "hit": hit,
            "sl_price": sl_price,
            "tp_price": tp_price,
            "exit": exit_reason,
        })

        if hit is not None:
            if sig.action == "buy":
                buy_total += 1
                if hit:
                    buy_hits += 1
            else:
                sell_total += 1
                if hit:
                    sell_hits += 1

        last_exit_idx = exit_idx

    return signals, {
        "ticker": ticker,
        "buy_hits": buy_hits,
        "buy_total": buy_total,
        "sell_hits": sell_hits,
        "sell_total": sell_total,
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
    signals: list[dict[str, Any]],
    summary: dict[str, Any],
    forward_days: int,
) -> None:
    ticker = summary["ticker"]
    W = "\u2550" * 65
    H = "\u2500" * 65
    print()
    print(f"  {ticker}")
    print(H)

    if not signals:
        print("  (no signals)")
        print(W)
        print()
        return

    print(
        f"  {'date':<10} {'dir':<5} {'entry':>10} "
        f"{'exit':>10} {'SL':>10} {'TP':>10} "
        f"{'pct':>8}  hit"
    )
    print(
        f"  {'-'*10} {'-'*5} {'-'*10} "
        f"{'-'*10} {'-'*10} {'-'*10} "
        f"{'-'*8}  {'-'*3}"
    )

    for s in signals:
        mark = "\u2713" if s["hit"] is True else ("\u2717" if s["hit"] is False else "-")
        exit_reason = s.get("exit", "expiry")
        if s.get("forward_price") is not None:
            base = _fmt_price(s["forward_price"])
            exit_str = f"{base} {exit_reason}" if exit_reason != "expiry" else base
        else:
            exit_str = "-"
        pct_str = (
            f"{s['pct']:+.1f}%" if s["pct"] is not None else "N/A"
        )
        entry_str = _fmt_price(s["entry_price"])
        sl_str = _fmt_price(s.get("sl_price")) if s.get("sl_price", 0) else "-"
        tp_str = _fmt_price(s.get("tp_price")) if s.get("tp_price", 0) else "-"
        dir_str = s["action"].upper()[:4]
        print(
            f"  {s['date']:<10} {dir_str:<5} {entry_str:>10} "
            f"{exit_str:>10} {sl_str:>10} {tp_str:>10} "
            f"{pct_str:>8}  {mark}"
        )

    print(H)

    buy_pcts = [
        s["pct"] for s in signals
        if s["action"] == "buy" and s["pct"] is not None
    ]
    sell_pcts = [
        s["pct"] for s in signals
        if s["action"] == "sell" and s["pct"] is not None
    ]
    buy_avg = sum(buy_pcts) / len(buy_pcts) if buy_pcts else 0.0
    sell_avg = sum(sell_pcts) / len(sell_pcts) if sell_pcts else 0.0

    buy_rate = (
        f"BUY  {summary['buy_hits']}/{summary['buy_total']}"
        f" ({summary['buy_hits'] / summary['buy_total'] * 100:.0f}%)"
        f" avg {buy_avg:+.1f}%"
        if summary["buy_total"] > 0
        else "BUY  -"
    )
    sell_rate = (
        f"SELL {summary['sell_hits']}/{summary['sell_total']}"
        f" ({summary['sell_hits'] / summary['sell_total'] * 100:.0f}%)"
        f" avg {sell_avg:+.1f}%"
        if summary["sell_total"] > 0
        else "SELL -"
    )
    print(f"  {buy_rate}  |  {sell_rate}")
    print(W)
    print()


def main() -> None:
    _load_dotenv()
    cfg = Config()

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s  %(levelname)-7s  %(message)s",
        datefmt="%H:%M:%S",
    )
    logger = logging.getLogger("benchmark")

    logger.info(
        "benchmark started  days=%d  forward=%dd  tickers=%s",
        cfg.benchmark_days,
        cfg.benchmark_forward_days,
        ", ".join(cfg.benchmark_tickers),
    )
    logger.info(
        "rsi period=%d  order=%d  sma=%s  atr=%d sl=%.1fx tp=%.1fx high=%d%% adj=%.1fx  pyramid=%s",
        cfg.rsi_period,
        cfg.rsi_divergence_order,
        "on" if cfg.sma_enabled else "off",
        cfg.atr_period,
        cfg.atr_sl_multiplier,
        cfg.atr_tp_multiplier,
        int(cfg.atr_pct_high),
        cfg.atr_high_adjust,
        "yes" if cfg.allow_pyramiding else "no",
    )

    for ticker in cfg.benchmark_tickers:
        logger.info("fetching %s klines...", ticker)
        try:
            df = fetch_klines(
                ticker,
                timeframe=cfg.kucoin_timeframe,
                base_url=cfg.kuCoin_base_url,
            )
        except KuCoinFetchError as e:
            logger.error("  skip %s - fetch error: %s", ticker, e)
            continue

        if len(df) < cfg.rsi_period + cfg.benchmark_forward_days + 1:
            logger.warning(
                "  skip %s - only %d candles", ticker, len(df)
            )
            continue

        # limit to the configured benchmark window
        needed = cfg.benchmark_days + cfg.benchmark_forward_days
        if len(df) > needed:
            df = df.iloc[-needed:]
            logger.info("  %s  using last %d candles", ticker, len(df))

        signals, summary = run_benchmark(
            df,
            ticker=ticker,
            rsi_period=cfg.rsi_period,
            rsi_order=cfg.rsi_divergence_order,
            forward_days=cfg.benchmark_forward_days,
            sma_enabled=cfg.sma_enabled,
            sma_period=cfg.sma_period,
            atr_period=cfg.atr_period,
            atr_sl_multiplier=cfg.atr_sl_multiplier,
            atr_tp_multiplier=cfg.atr_tp_multiplier,
            atr_pct_high=cfg.atr_pct_high,
            atr_high_adjust=cfg.atr_high_adjust,
            atr_sl_max_pct=cfg.atr_sl_max_pct,
            allow_pyramiding=cfg.allow_pyramiding,
        )

        print_ledger(signals, summary, cfg.benchmark_forward_days)

    logger.info("done")


if __name__ == "__main__":
    main()
