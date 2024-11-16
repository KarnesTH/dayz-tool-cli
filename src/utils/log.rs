use simplelog::*;

pub fn init_logger() -> Result<(), Box<dyn std::error::Error>> {
    let log_path = std::path::Path::new(".dayz-tool/logs");
    if !log_path.exists() {
        std::fs::create_dir_all(log_path)?;
    }

    let log_file = std::fs::File::create(log_path.join(format!(
        "dayz-tool_{}.log",
        chrono::Local::now().format("%Y-%m-%d")
    )))?;

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(LevelFilter::Debug, Config::default(), log_file),
    ])?;

    Ok(())
}
