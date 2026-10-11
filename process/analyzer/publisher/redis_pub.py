import json
import logging

from analyzer.models.types import Signal

logger = logging.getLogger(__name__)


def publish_signals(
    redis_client, signals: list[Signal], channel: str = "signals:result"
) -> None:
    """Publish a list of signals to a Redis Pub/Sub channel.

    Args:
        redis_client: A redis.Redis client instance.
        signals: List of Signal dataclasses to publish.
        channel: Redis channel name.

    Silently skips if signals list is empty. Logs errors on publish
    failure but does not raise.
    """
    if not signals:
        return

    payload = json.dumps([s.to_dict() for s in signals])

    try:
        redis_client.publish(channel, payload)
        logger.debug("→ %s  %d signal(s)", channel, len(signals))
    except Exception:
        logger.exception("publish to %s failed", channel)
