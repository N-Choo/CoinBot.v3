"""Render the self-contained benchmark replay report."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

TEMPLATE_PATH = Path(__file__).with_name("replay_template.html")
PLACEHOLDER = "__REPLAY_DATA__"


def render_report(report: dict[str, Any]) -> str:
    """Inject a JSON payload into the static report template."""
    template = TEMPLATE_PATH.read_text(encoding="utf-8")
    if PLACEHOLDER not in template:
        raise ValueError(f"template missing {PLACEHOLDER}")
    payload = json.dumps(report, separators=(",", ":"), allow_nan=False)
    payload = payload.replace("<", "\\u003c")
    return template.replace(PLACEHOLDER, payload)


def write_report(report: dict[str, Any], path: Path) -> Path:
    """Write the report, creating parent directories as needed."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(render_report(report), encoding="utf-8")
    return path
