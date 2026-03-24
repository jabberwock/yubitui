use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PivState {
    #[allow(dead_code)]
    pub slots: Vec<SlotInfo>,
}

#[derive(Debug, Clone)]
pub struct SlotInfo {
    #[allow(dead_code)]
    pub slot: String,
    #[allow(dead_code)]
    pub algorithm: Option<String>,
    #[allow(dead_code)]
    pub subject: Option<String>,
}

pub fn get_piv_state() -> Result<PivState> {
    // Try to use ykman if available, otherwise return empty
    let output = Command::new("ykman").args(["piv", "info"]).output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Ok(parse_piv_info(&stdout));
        }
    }

    // If ykman not available or fails, return empty state
    Ok(PivState { slots: vec![] })
}

fn parse_piv_info(output: &str) -> PivState {
    let mut slots = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        // Look for slot information like "Slot 9a:"
        if line.starts_with("Slot ") {
            if let Some(slot_id) = line.split(':').next() {
                let slot_name = slot_id.trim_start_matches("Slot ").to_string();

                // Parse algorithm and subject from following lines
                // This is a simplified parser - real implementation would be more robust
                slots.push(SlotInfo {
                    slot: slot_name,
                    algorithm: None,
                    subject: None,
                });
            }
        }
    }

    PivState { slots }
}
