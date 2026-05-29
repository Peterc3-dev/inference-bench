use serde::{Deserialize, Serialize};
use std::fs;

/// Memory snapshot from /proc/meminfo and GPU sysfs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// System available memory in MiB
    pub system_available_mib: u64,
    /// GPU VRAM used in MiB (0 if unreadable)
    pub gpu_vram_used_mib: u64,
    /// GPU GTT used in MiB (0 if unreadable)
    pub gpu_gtt_used_mib: u64,
}

impl MemorySnapshot {
    pub fn capture() -> Self {
        Self {
            system_available_mib: read_meminfo_available(),
            gpu_vram_used_mib: read_gpu_mem_used("vram"),
            gpu_gtt_used_mib: read_gpu_mem_used("gtt"),
        }
    }
}

fn read_meminfo_available() -> u64 {
    let content = match fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return 0,
    };
    for line in content.lines() {
        if line.starts_with("MemAvailable:") {
            // Format: "MemAvailable:   12345678 kB"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(kb) = parts[1].parse::<u64>() {
                    return kb / 1024;
                }
            }
        }
    }
    0
}

fn read_gpu_mem_used(mem_type: &str) -> u64 {
    // Try each card under /sys/class/drm/
    let drm_dir = "/sys/class/drm";
    let entries = match fs::read_dir(drm_dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let total_path = format!("{}/{}/device/mem_info_{}_total", drm_dir, name, mem_type);
        let used_path = format!("{}/{}/device/mem_info_{}_used", drm_dir, name, mem_type);
        if let (Ok(total_s), Ok(used_s)) = (
            fs::read_to_string(&total_path),
            fs::read_to_string(&used_path),
        ) {
            let total: u64 = total_s.trim().parse().unwrap_or(0);
            let used: u64 = used_s.trim().parse().unwrap_or(0);
            if total > 0 {
                return used / (1024 * 1024);
            }
        }
    }
    0
}

/// Compute mean and standard deviation of a slice.
pub fn mean_std(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    if values.len() == 1 {
        return (mean, 0.0);
    }
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    (mean, variance.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_std_empty() {
        assert_eq!(mean_std(&[]), (0.0, 0.0));
    }

    #[test]
    fn mean_std_single() {
        // A single sample has no spread; std is defined as 0.
        assert_eq!(mean_std(&[5.0]), (5.0, 0.0));
    }

    #[test]
    fn mean_std_known_values() {
        // Sample std (n-1 denominator) of [2,4,4,4,5,5,7,9] is 2.13809...
        let (mean, std) = mean_std(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        assert!((mean - 5.0).abs() < 1e-9);
        assert!((std - 2.138_089_935_299_395).abs() < 1e-9);
    }

    #[test]
    fn mean_std_identical_values_zero_std() {
        let (mean, std) = mean_std(&[3.0, 3.0, 3.0]);
        assert!((mean - 3.0).abs() < 1e-9);
        assert!(std.abs() < 1e-9);
    }
}
