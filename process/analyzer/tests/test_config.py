import os
from unittest import mock
from analyzer.config import Config


def test_config_reads_env_with_defaults():
    with mock.patch.dict(os.environ, {}, clear=True):
        c = Config()
        assert c.redis_host == "localhost"
        assert c.redis_port == 6379
        assert c.redis_db == 0
        assert c.kuCoin_base_url == "https://api.kucoin.com"
        assert c.kucoin_timeframe == "1day"
        assert c.rsi_period == 14
        assert c.rsi_divergence_order == 5
        assert c.price_diff_threshold == 0.05
        assert c.result_channel == "signals:result"
        assert c.ticker_channel == "tickers:analyze"
        assert c.sma_enabled is False
        assert c.sma_period == 20
        assert c.atr_period == 14
        assert c.atr_sl_multiplier == 2.0
        assert c.atr_tp_multiplier == 3.0
        assert c.atr_pct_high == 5.0
        assert c.atr_high_adjust == 1.5
        assert c.atr_sl_max_pct == 0.0
        assert c.benchmark_days == 90
        assert c.benchmark_forward_days == 7
        assert c.benchmark_tickers == ["BTC-USDT", "ETH-USDT", "SOL-USDT"]
        assert c.allow_pyramiding is False


def test_config_reads_custom_env():
    env = {
        "REDIS_HOST": "redis.local",
        "REDIS_PORT": "6380",
        "REDIS_DB": "1",
        "KUCOIN_BASE_URL": "https://api-futures.kucoin.com",
        "KUCOIN_TIMEFRAME": "4hour",
        "RSI_PERIOD": "10",
        "RSI_DIVERGENCE_ORDER": "7",
        "PRICE_DIFF_THRESHOLD": "0.08",
        "RESULT_CHANNEL": "signals:v1",
        "TICKER_CHANNEL": "tickers:v2",
        "SMA_ENABLED": "true",
        "SMA_PERIOD": "50",
        "ATR_PERIOD": "10",
        "ATR_SL_MULTIPLIER": "2.5",
        "ATR_TP_MULTIPLIER": "3.5",
        "ATR_PCT_HIGH": "6.0",
        "ATR_HIGH_ADJUST": "2.0",
        "ATR_SL_MAX_PCT": "0.10",
        "BENCHMARK_DAYS": "180",
        "BENCHMARK_FORWARD_DAYS": "14",
        "BENCHMARK_TICKERS": "XRP-USDT,DOGE-USDT",
    }
    with mock.patch.dict(os.environ, env, clear=True):
        c = Config()
        assert c.redis_host == "redis.local"
        assert c.redis_port == 6380
        assert c.redis_db == 1
        assert c.kuCoin_base_url == "https://api-futures.kucoin.com"
        assert c.kucoin_timeframe == "4hour"
        assert c.rsi_period == 10
        assert c.rsi_divergence_order == 7
        assert c.price_diff_threshold == 0.08
        assert c.result_channel == "signals:v1"
        assert c.ticker_channel == "tickers:v2"
        assert c.sma_enabled is True
        assert c.sma_period == 50
        assert c.atr_period == 10
        assert c.atr_sl_multiplier == 2.5
        assert c.atr_tp_multiplier == 3.5
        assert c.atr_pct_high == 6.0
        assert c.atr_high_adjust == 2.0
        assert c.atr_sl_max_pct == 0.10
        assert c.benchmark_days == 180
        assert c.benchmark_forward_days == 14
        assert c.benchmark_tickers == ["XRP-USDT", "DOGE-USDT"]
