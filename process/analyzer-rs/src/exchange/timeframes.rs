#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeframe {
    M15,
    H1,
    H4,
    D1,
}

impl Timeframe {
    pub fn from_kucoin(value: &str) -> Option<Self> {
        match value {
            "15min" => Some(Timeframe::M15),
            "1hour" => Some(Timeframe::H1),
            "4hour" => Some(Timeframe::H4),
            "1day" => Some(Timeframe::D1),
            _ => None,
        }
    }

    pub fn as_kucoin(self) -> &'static str {
        match self {
            Timeframe::M15 => "15min",
            Timeframe::H1 => "1hour",
            Timeframe::H4 => "4hour",
            Timeframe::D1 => "1day",
        }
    }

    pub fn millis(self) -> i64 {
        match self {
            Timeframe::M15 => 900_000,
            Timeframe::H1 => 3_600_000,
            Timeframe::H4 => 14_400_000,
            Timeframe::D1 => 86_400_000,
        }
    }

    pub fn is_closed(self, open_time_ms: i64, now_ms: i64, settle_ms: i64) -> bool {
        now_ms - settle_ms > open_time_ms + self.millis()
    }
}
