import os


class Config:
    def __init__(self) -> None:
        self.redis_host = os.getenv("REDIS_HOST", "localhost")
        self.redis_port = int(os.getenv("REDIS_PORT", "6379"))
        self.redis_db = int(os.getenv("REDIS_DB", "0"))
        self.kuCoin_base_url = os.getenv(
            "KUCOIN_BASE_URL", "https://api.kucoin.com"
        )
        self.kucoin_timeframe = os.getenv("KUCOIN_TIMEFRAME", "1day")
        self.rsi_period = int(os.getenv("RSI_PERIOD", "14"))
        self.rsi_divergence_order = int(os.getenv("RSI_DIVERGENCE_ORDER", "5"))
        self.price_diff_threshold = float(
            os.getenv("PRICE_DIFF_THRESHOLD", "0.05")
        )
        self.sma_enabled = os.getenv("SMA_ENABLED", "false").lower() in (
            "1",
            "true",
            "yes",
            "on",
        )
        self.sma_period = int(os.getenv("SMA_PERIOD", "20"))
        self.atr_period = int(os.getenv("ATR_PERIOD", "14"))
        self.atr_sl_multiplier = float(
            os.getenv("ATR_SL_MULTIPLIER", "2.0")
        )
        self.atr_tp_multiplier = float(
            os.getenv("ATR_TP_MULTIPLIER", "3.0")
        )
        self.atr_pct_high = float(os.getenv("ATR_PCT_HIGH", "5.0"))
        self.atr_high_adjust = float(
            os.getenv("ATR_HIGH_ADJUST", "1.5")
        )
        self.atr_sl_max_pct = float(
            os.getenv("ATR_SL_MAX_PCT", "0.0")
        )
        self.benchmark_days = int(os.getenv("BENCHMARK_DAYS", "90"))
        self.benchmark_forward_days = int(
            os.getenv("BENCHMARK_FORWARD_DAYS", "7")
        )
        _tickers = os.getenv(
            "BENCHMARK_TICKERS",
            "BTC-USDT,ETH-USDT,SOL-USDT",
        )
        self.benchmark_tickers = [
            t.strip() for t in _tickers.split(",") if t.strip()
        ]
        self.allow_pyramiding = (
            os.getenv("ALLOW_PYRAMIDING", "false").lower()
            in ("1", "true", "yes", "on")
        )
        self.result_channel = os.getenv("RESULT_CHANNEL", "signals:result")
        self.ticker_channel = os.getenv("TICKER_CHANNEL", "tickers:analyze")
