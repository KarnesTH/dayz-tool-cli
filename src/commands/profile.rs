use log::info;

pub fn show_profile() -> Result<(), Box<dyn std::error::Error>> {
    info!("Showing profile settings...");
    Ok(())
}
