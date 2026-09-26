use std::io::Write;

pub fn init_logger() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .write_style(env_logger::WriteStyle::Always)
        .format(|buf, record| {
            let style = buf.default_level_style(record.level());
            writeln!(
                buf,
                "[{}] [{style}{level:^9}{style:#}] [{:^20}] {}",
                buf.timestamp(),
                record.target().split("::").last().unwrap_or("SERVER"),
                record.args(),
                style = style,
                level = record.level(),
            )
        })
        .init();
}
