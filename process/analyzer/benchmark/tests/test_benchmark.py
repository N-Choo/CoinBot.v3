import logging
import os

import numpy as np
import pandas as pd
import pytest

from analyzer.benchmark.main import _fmt_price, print_ledger, run_benchmark
from analyzer.models.types import Signal


def test_run_benchmark_returns_trades_and_summary():
    """End-to-end: the replay produces valid trade and summary structures."""
    np.random.seed(99)
    n = 200
    close = np.linspace(100, 80, n) + np.random.randn(n) * 0.3
    high = close + np.random.rand(n) * 2
    low = close - np.random.rand(n) * 2
    df = pd.DataFrame({"High": high, "Low": low, "Close": close})

    trades, open_positions, summary = run_benchmark(
        df,
        ticker="TEST-USDT",
        rsi_period=14,
        rsi_order=10,
        sma_enabled=False,
        sma_period=20,
        benchmark_days=90,
        init_fund=1000.0,
        fee_rate=0.001,
    )

    assert isinstance(trades, list)
    assert isinstance(open_positions, list)
    for key in (
        "ticker",
        "trades",
        "wins",
        "win_rate",
        "realized_pnl",
        "unrealized_pnl",
        "open_count",
        "init_fund",
        "available_fund",
        "equity",
    ):
        assert key in summary
    assert summary["ticker"] == "TEST-USDT"
    assert summary["equity"] == pytest.approx(
        summary["init_fund"]
        + summary["realized_pnl"]
        + summary["unrealized_pnl"]
    )
    for t in trades:
        assert t["side"] in ("buy", "sell")
        assert t["reason"] in ("TP", "SL")


def test_run_benchmark_simulates_trades_end_to_end(monkeypatch):
    """Drive the replay with a fixed signal and verify TP exits are booked."""
    import analyzer.benchmark.main as bm

    n = 60
    close = np.full(n, 100.0)
    close[40:] = 120.0
    df = pd.DataFrame(
        {"High": close + 1, "Low": close - 1, "Close": close}
    )

    def fake_signals(df_slice, **kwargs):
        entry = float(df_slice["Close"].iloc[-1])
        return [
            Signal(
                ticker="TEST-USDT",
                action="buy",
                confidence=0.6,
                entry_price=entry,
                reason="regular_bullish_divergence",
                sl_price=entry * 0.9,
                tp_price=entry * 1.1,
            )
        ]

    monkeypatch.setattr(bm, "generate_signals", fake_signals)

    trades, open_positions, summary = run_benchmark(
        df, ticker="TEST-USDT", benchmark_days=50,
        init_fund=1000.0, fee_rate=0.0,
    )

    assert summary["trades"] >= 1
    assert all(t["reason"] == "TP" for t in trades)
    assert all(t["pnl"] > 0 for t in trades)


def _summary(**overrides):
    base = {
        "ticker": "BTC-USDT",
        "trades": 0,
        "wins": 0,
        "win_rate": 0.0,
        "realized_pnl": 0.0,
        "unrealized_pnl": 0.0,
        "open_count": 0,
        "init_fund": 1000.0,
        "available_fund": 1000.0,
        "equity": 1000.0,
    }
    base.update(overrides)
    return base


def test_print_ledger_no_trades(capsys):
    print_ledger("BTC-USDT", [], [], _summary())
    out = capsys.readouterr().out
    assert "BTC-USDT" in out
    assert "(no trades)" in out


def test_print_ledger_with_trades(capsys):
    trades = [
        {
            "side": "buy", "entry_price": 91200.0, "exit_price": 96000.0,
            "E": 500.0, "pnl": 50.0, "pnl_pct": 5.0, "reason": "TP",
            "entry_date": "2026-05-12", "exit_date": "2026-05-19",
        },
        {
            "side": "buy", "entry_price": 88000.0, "exit_price": 85000.0,
            "E": 500.0, "pnl": -50.0, "pnl_pct": -10.0, "reason": "SL",
            "entry_date": "2026-05-28", "exit_date": "2026-06-02",
        },
    ]
    summary = _summary(trades=2, wins=1, win_rate=50.0, realized_pnl=0.0)
    print_ledger("BTC-USDT", trades, [], summary)
    out = capsys.readouterr().out
    assert "TP" in out
    assert "SL" in out
    assert "50%" in out
    assert "$91,200" in out
    assert "+50.00" in out
    assert "realized" in out
    assert "equity" in out
    assert "open: 0" in out


def test_fmt_price_buckets():
    assert _fmt_price(None) == "-"
    assert _fmt_price(91200.0) == "$91,200"
    assert _fmt_price(912.5) == "$912.50"
    assert _fmt_price(0.5) == "$0.5000"
    assert _fmt_price(0.001) == "$0.001000"


def test_print_ledger_open_positions_without_trades(capsys):
    open_positions = [
        {
            "side": "buy", "entry_price": 91200.0, "exit_price": 0.0,
            "E": 500.0, "pnl": 0.0, "pnl_pct": 0.0, "reason": "open",
            "entry_date": "2026-06-01", "exit_date": "",
            "entry_idx": 9, "exit_idx": 0,
        }
    ]
    print_ledger("BTC-USDT", [], open_positions, _summary(open_count=1))
    out = capsys.readouterr().out
    assert "2026-06-01" in out
    assert "open" in out
    assert "(no trades)" not in out


def test_main_skips_ticker_on_transport_error(monkeypatch, caplog):
    import analyzer.benchmark.main as bm

    monkeypatch.setenv("BENCHMARK_TICKERS", "BTC-USDT")

    def boom(*args, **kwargs):
        raise ConnectionError("network down")

    monkeypatch.setattr(bm, "fetch_klines", boom)
    with caplog.at_level(logging.ERROR):
        bm.main()  # must not raise

    assert "skip BTC-USDT" in caplog.text


def test_main_web_writes_report(monkeypatch, tmp_path):
    import analyzer.benchmark.main as bm

    monkeypatch.setenv("BENCHMARK_TICKERS", "TEST-USDT")
    monkeypatch.setenv("SMA_ENABLED", "false")
    monkeypatch.setenv("RSI_PERIOD", "1")
    monkeypatch.setenv("ATR_PERIOD", "1")
    monkeypatch.setenv("BENCHMARK_DAYS", "50")

    n = 80
    close = np.full(n, 100.0)
    frame = pd.DataFrame(
        {"Open": close, "High": close + 2, "Low": close - 2, "Close": close}
    )

    captured = {}

    def fetch(symbol, **kwargs):
        captured.update(kwargs)
        return frame

    monkeypatch.setattr(bm, "fetch_klines", fetch)
    monkeypatch.setattr(bm, "generate_signals", lambda *a, **k: [])

    out = tmp_path / "replay.html"
    bm.main(web=True, out=out)

    assert out.exists()
    html = out.read_text(encoding="utf-8")
    assert html.startswith("<!DOCTYPE html>")
    assert "TEST-USDT" in html
    assert "replay-data" in html
    assert captured["limit"] == 61


def test_main_web_reports_all_failures(monkeypatch, tmp_path):
    import analyzer.benchmark.main as bm

    monkeypatch.setenv("BENCHMARK_TICKERS", "BTC-USDT,SOL-USDT")

    def boom(*args, **kwargs):
        raise ConnectionError("network down")

    monkeypatch.setattr(bm, "fetch_klines", boom)

    out = tmp_path / "replay.html"
    bm.main(web=True, out=out)

    html = out.read_text(encoding="utf-8")
    assert "BTC-USDT" in html
    assert "SOL-USDT" in html
    assert "network down" in html
    assert "No tickers were replayed" in html


def test_main_web_keeps_successful_ticker_when_another_fails(
    monkeypatch, tmp_path
):
    import analyzer.benchmark.main as bm

    monkeypatch.setenv("BENCHMARK_TICKERS", "BTC-USDT,SOL-USDT")
    monkeypatch.setenv("SMA_ENABLED", "false")
    monkeypatch.setenv("RSI_PERIOD", "1")
    monkeypatch.setenv("ATR_PERIOD", "1")
    monkeypatch.setenv("BENCHMARK_DAYS", "50")

    n = 80
    close = np.full(n, 100.0)
    frame = pd.DataFrame(
        {"Open": close, "High": close + 2, "Low": close - 2, "Close": close}
    )

    def fetch(symbol, **kwargs):
        if symbol == "SOL-USDT":
            raise ConnectionError("down")
        return frame

    monkeypatch.setattr(bm, "fetch_klines", fetch)
    monkeypatch.setattr(bm, "generate_signals", lambda *a, **k: [])

    out = tmp_path / "replay.html"
    bm.main(web=True, out=out)

    html = out.read_text(encoding="utf-8")
    assert "BTC-USDT" in html
    assert "SOL-USDT" in html


def test_parse_args_rejects_out_without_web(capsys):
    from analyzer.benchmark.main import _parse_args

    with pytest.raises(SystemExit):
        _parse_args(["--out", "replay.html"])

    assert "--out requires --web" in capsys.readouterr().err


def test_load_dotenv_finds_parent_file(tmp_path, monkeypatch):
    from analyzer.benchmark.main import _load_dotenv

    root = tmp_path
    child = root / "process"
    child.mkdir()
    (root / ".env").write_text("BENCHMARK_DAYS=111\n", encoding="utf-8")

    monkeypatch.delenv("BENCHMARK_DAYS", raising=False)
    monkeypatch.chdir(child)

    _load_dotenv()

    assert os.environ["BENCHMARK_DAYS"] == "111"


def test_load_dotenv_prefers_nearest_file(tmp_path, monkeypatch):
    from analyzer.benchmark.main import _load_dotenv

    root = tmp_path
    child = root / "process"
    child.mkdir()
    (root / ".env").write_text("BENCHMARK_DAYS=111\n", encoding="utf-8")
    (child / ".env").write_text("BENCHMARK_DAYS=222\n", encoding="utf-8")

    monkeypatch.delenv("BENCHMARK_DAYS", raising=False)
    monkeypatch.chdir(child)

    _load_dotenv()

    assert os.environ["BENCHMARK_DAYS"] == "222"


def test_load_dotenv_keeps_existing_environment(tmp_path, monkeypatch):
    from analyzer.benchmark.main import _load_dotenv

    (tmp_path / ".env").write_text("BENCHMARK_DAYS=111\n", encoding="utf-8")

    monkeypatch.setenv("BENCHMARK_DAYS", "999")
    monkeypatch.chdir(tmp_path)

    _load_dotenv()

    assert os.environ["BENCHMARK_DAYS"] == "999"
