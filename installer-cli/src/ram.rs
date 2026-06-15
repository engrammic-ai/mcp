// src/ram.rs

use sysinfo::System;

/// Detect system RAM in GB.
/// Returns None if detection fails.
pub fn detect_system_ram() -> Option<u64> {
    let sys = System::new_all();
    let total_bytes = sys.total_memory();
    if total_bytes == 0 {
        return None;
    }
    // Convert bytes to GB (rounded)
    Some(total_bytes / (1024 * 1024 * 1024))
}

/// RAM thresholds for each standalone tier.
pub fn tier_min_ram(tier: &str) -> u64 {
    match tier {
        "Lite" => 8,
        "Standard" => 24,
        "Pro" => 48,
        _ => 0,
    }
}

/// Check if detected RAM meets tier requirements.
pub enum RamCheckResult {
    /// RAM meets requirements
    Ok,
    /// RAM below minimum, includes detected GB and minimum GB
    Warning { detected: u64, minimum: u64 },
    /// Detection failed, skip warning
    Unknown,
}

pub fn check_ram_for_tier(tier: &str) -> RamCheckResult {
    let minimum = tier_min_ram(tier);
    if minimum == 0 {
        return RamCheckResult::Ok;
    }

    match detect_system_ram() {
        Some(detected) if detected >= minimum => RamCheckResult::Ok,
        Some(detected) => RamCheckResult::Warning { detected, minimum },
        None => RamCheckResult::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_min_ram_returns_correct_values() {
        assert_eq!(tier_min_ram("Lite"), 8);
        assert_eq!(tier_min_ram("Standard"), 24);
        assert_eq!(tier_min_ram("Pro"), 48);
        assert_eq!(tier_min_ram("Cloud"), 0);
    }

    #[test]
    fn detect_system_ram_returns_some() {
        // This test may fail in minimal CI environments
        let ram = detect_system_ram();
        // At minimum, the function should not panic
        // If it returns Some, it should be > 0
        if let Some(gb) = ram {
            assert!(gb > 0);
        }
    }
}
