import json
import re

import pytest

from analyzer.benchmark.report import render_report, write_report


def _embedded(html: str) -> dict:
    match = re.search(
        r'<script id="replay-data" type="application/json">(.*?)</script>',
        html,
        re.S,
    )
    assert match is not None
    return json.loads(match.group(1))


def test_render_report_embeds_parseable_json():
    report = {
        "generated_at": "2026-10-20T12:00:00",
        "tickers": [{"ticker": "BTC-USDT"}],
        "failures": [],
    }
    html = render_report(report)
    assert html.startswith("<!DOCTYPE html>")
    assert _embedded(html) == report
    assert 'id="price-svg"' in html
    assert 'id="rsi-svg"' in html
    assert 'id="track"' in html


def test_render_report_escapes_script_breakout():
    bad = "</script><script>alert(1)</script><b>x</b>"
    html = render_report({"x": bad})
    assert "</script><script>alert(1)" not in html
    assert "\\u003c/script>" in html
    assert "\\u003cb>" in html
    assert "<b>x</b>" not in html
    assert _embedded(html)["x"] == bad


def test_render_report_rejects_non_finite_json():
    with pytest.raises(ValueError):
        render_report({"x": float("nan")})


def test_write_report_creates_parent_directories(tmp_path):
    path = write_report({"tickers": [], "failures": []}, tmp_path / "a" / "b.html")
    assert path.exists()
    assert path.read_text(encoding="utf-8").startswith("<!DOCTYPE html>")
