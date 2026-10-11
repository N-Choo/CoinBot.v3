use std::io::Write;

use chrono::Local;

/// Initialise the global logger.
///
/// Reads `RUST_LOG` (default `info`) and prints compact
/// `HH:MM:SS  LEVEL  process  message` lines: local time, the crate root as the
/// process label, and a colored 5-wide level.
pub fn init_logger() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .write_style(env_logger::WriteStyle::Always)
        .format(|buf, record| {
            let level = record.level();
            let style = buf.default_level_style(level);
            let target = record
                .target()
                .split("::")
                .next()
                .unwrap_or("server")
                .replace('_', "-");
            writeln!(
                buf,
                "{}  {style}{level:<5}{style:#}  {target}  {}",
                Local::now().format("%H:%M:%S"),
                record.args(),
            )
        })
        .init();
}
