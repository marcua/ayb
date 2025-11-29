//! Process isolation enforcement.
//!
//! This module applies sandbox restrictions to the daemon process using
//! native Linux primitives instead of nsjail.

use crate::error::AybError;
use crate::hosted_db::sandbox_capabilities::ResourceLimits;
use std::path::Path;

/// Apply all available isolation mechanisms to the current process.
/// This should be called early in the daemon process before handling any queries.
pub fn apply_isolation(db_path: &Path, limits: &ResourceLimits) -> Result<(), AybError> {
    // Apply cgroup CPU limits first (Linux only, requires cgroups v2)
    // This is critical for multi-tenant isolation
    #[cfg(target_os = "linux")]
    apply_cgroup_limits(limits)?;

    // Apply rlimit restrictions (works on all Unix platforms)
    #[cfg(unix)]
    {
        if let Err(e) = apply_rlimits(limits) {
            eprintln!("Warning: Failed to apply rlimits: {}", e);
        }
    }

    // Apply Landlock filesystem restrictions (Linux 5.13+)
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = apply_landlock(db_path) {
            eprintln!("Warning: Failed to apply Landlock: {}", e);
        }
    }

    // Note: seccomp syscall filtering is NOT applied here because:
    // 1. It causes segfaults in constrained environments (gVisor, containers)
    // 2. The container runtime already provides syscall filtering
    // 3. SQLite authorizer + Landlock + rlimits provide sufficient isolation
    // If you need seccomp, run ayb in a container with --security-opt seccomp=...

    // Suppress unused variable warning on non-Unix platforms
    #[cfg(not(unix))]
    let _ = (db_path, limits);

    Ok(())
}

/// Apply cgroup v2 CPU limits (Linux only)
/// Creates a cgroup for this process and sets CPU quota
#[cfg(target_os = "linux")]
fn apply_cgroup_limits(limits: &ResourceLimits) -> Result<(), AybError> {
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    let pid = std::process::id();
    let cgroup_base = PathBuf::from("/sys/fs/cgroup");

    // Check if cgroups v2 is available
    if !cgroup_base.join("cgroup.controllers").exists() {
        eprintln!(
            "Warning: cgroups v2 not available. CPU rate limits NOT enforced. \
             This is REQUIRED for multi-tenant isolation."
        );
        return Ok(());
    }

    // Create the ayb parent cgroup if it doesn't exist
    let ayb_cgroup = cgroup_base.join("ayb");
    if !ayb_cgroup.exists() {
        if let Err(e) = fs::create_dir(&ayb_cgroup) {
            eprintln!(
                "Warning: Cannot create cgroup directory {:?}: {}. \
                 CPU rate limits NOT enforced. \
                 For Docker: use --cgroupns=host or enable cgroup delegation.",
                ayb_cgroup, e
            );
            return Ok(());
        }
    }

    // Enable cpu controller in the parent cgroup
    let subtree_control = ayb_cgroup.join("cgroup.subtree_control");
    if let Err(e) = fs::write(&subtree_control, "+cpu") {
        eprintln!(
            "Warning: Cannot enable cpu controller: {}. \
             CPU rate limits NOT enforced.",
            e
        );
        return Ok(());
    }

    // Create a cgroup for this specific daemon
    let daemon_cgroup = ayb_cgroup.join(format!("daemon-{}", pid));
    if let Err(e) = fs::create_dir(&daemon_cgroup) {
        eprintln!(
            "Warning: Cannot create daemon cgroup {:?}: {}. \
             CPU rate limits NOT enforced.",
            daemon_cgroup, e
        );
        return Ok(());
    }

    // Set CPU quota: cpu.max format is "quota period" in microseconds
    // period is typically 100000 (100ms)
    // quota is the max microseconds of CPU time per period
    let period_us: u64 = 100_000; // 100ms
    let quota_us = (period_us * limits.cpu_percent as u64) / 100;
    let cpu_max = format!("{} {}", quota_us, period_us);

    let cpu_max_path = daemon_cgroup.join("cpu.max");
    if let Err(e) = fs::write(&cpu_max_path, &cpu_max) {
        eprintln!(
            "Warning: Cannot set cpu.max to {}: {}. \
             CPU rate limits NOT enforced.",
            cpu_max, e
        );
        // Clean up the cgroup we created
        let _ = fs::remove_dir(&daemon_cgroup);
        return Ok(());
    }

    // Move this process into the cgroup
    let procs_path = daemon_cgroup.join("cgroup.procs");
    let mut file = match fs::File::create(&procs_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "Warning: Cannot open cgroup.procs: {}. \
                 CPU rate limits NOT enforced.",
                e
            );
            let _ = fs::remove_dir(&daemon_cgroup);
            return Ok(());
        }
    };

    if let Err(e) = writeln!(file, "{}", pid) {
        eprintln!(
            "Warning: Cannot move process to cgroup: {}. \
             CPU rate limits NOT enforced.",
            e
        );
        let _ = fs::remove_dir(&daemon_cgroup);
        return Ok(());
    }

    // Note: The cgroup will be cleaned up by the DaemonRegistry when the daemon exits
    // The cgroup directory path is stored in an environment variable for cleanup
    std::env::set_var("AYB_CGROUP_PATH", daemon_cgroup.to_string_lossy().as_ref());

    Ok(())
}

/// Clean up a cgroup directory when daemon exits
/// This should be called by the parent process (DaemonRegistry) after the daemon terminates
#[cfg(target_os = "linux")]
pub fn cleanup_cgroup(pid: u32) {
    use std::fs;
    use std::path::PathBuf;

    let daemon_cgroup = PathBuf::from("/sys/fs/cgroup/ayb").join(format!("daemon-{}", pid));
    if daemon_cgroup.exists() {
        // The cgroup should be empty after the process exits
        if let Err(e) = fs::remove_dir(&daemon_cgroup) {
            eprintln!(
                "Warning: Failed to clean up cgroup {:?}: {}",
                daemon_cgroup, e
            );
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn cleanup_cgroup(_pid: u32) {
    // No-op on non-Linux platforms
}

/// Apply resource limits using rlimit (Unix only)
#[cfg(unix)]
fn apply_rlimits(limits: &ResourceLimits) -> Result<(), AybError> {
    use rlimit::{setrlimit, Resource};

    // Memory limit (address space)
    setrlimit(Resource::AS, limits.memory_bytes, limits.memory_bytes).map_err(|e| {
        AybError::Other {
            message: format!("Failed to set memory limit: {}", e),
        }
    })?;

    // File size limit
    setrlimit(
        Resource::FSIZE,
        limits.max_file_size_bytes,
        limits.max_file_size_bytes,
    )
    .map_err(|e| AybError::Other {
        message: format!("Failed to set file size limit: {}", e),
    })?;

    // File descriptor limit
    setrlimit(
        Resource::NOFILE,
        limits.max_file_descriptors,
        limits.max_file_descriptors,
    )
    .map_err(|e| AybError::Other {
        message: format!("Failed to set file descriptor limit: {}", e),
    })?;

    // Process limit - note: on Linux this applies per-user, not per-process
    // We still set it for defense in depth
    #[cfg(target_os = "linux")]
    {
        setrlimit(Resource::NPROC, limits.max_processes, limits.max_processes).map_err(|e| {
            AybError::Other {
                message: format!("Failed to set process limit: {}", e),
            }
        })?;
    }

    Ok(())
}

/// Apply Landlock filesystem restrictions (Linux 5.13+)
#[cfg(target_os = "linux")]
fn apply_landlock(db_path: &Path) -> Result<(), AybError> {
    use landlock::{
        Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr, ABI,
    };

    // Try to detect the best available ABI by attempting to create a ruleset
    let abi = [ABI::V5, ABI::V4, ABI::V3, ABI::V2, ABI::V1]
        .into_iter()
        .find(|&abi| {
            Ruleset::default()
                .handle_access(AccessFs::from_all(abi))
                .is_ok()
        });

    let abi = match abi {
        Some(abi) => abi,
        None => {
            // Landlock not available, skip
            eprintln!(
                "Warning: Landlock not available on this kernel. \
                 Filesystem isolation disabled."
            );
            return Ok(());
        }
    };

    // Get the database directory (parent of database file)
    let db_dir = db_path.parent().ok_or_else(|| AybError::Other {
        message: "Database path has no parent directory".to_string(),
    })?;

    // Create ruleset with all filesystem access types handled
    let ruleset = match Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| AybError::Other {
            message: format!("Failed to create Landlock ruleset: {}", e),
        })?
        .create()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "Warning: Failed to create Landlock ruleset: {}. Skipping.",
                e
            );
            return Ok(());
        }
    };

    // Read-only access to system libraries needed by SQLite
    let read_access = AccessFs::from_read(abi);

    // Collect all rules to add
    let mut rules_to_add: Vec<PathBeneath<PathFd>> = Vec::new();

    for path in ["/lib", "/lib64", "/usr", "/etc"] {
        if std::path::Path::new(path).exists() {
            if let Ok(fd) = PathFd::new(path) {
                rules_to_add.push(PathBeneath::new(fd, read_access));
            }
        }
    }

    // Read-write access to the database directory only
    // This allows creating -wal, -shm, -journal files
    let write_access = AccessFs::ReadFile
        | AccessFs::WriteFile
        | AccessFs::ReadDir
        | AccessFs::MakeReg
        | AccessFs::Truncate;

    if db_dir.exists() {
        if let Ok(fd) = PathFd::new(db_dir) {
            rules_to_add.push(PathBeneath::new(fd, write_access));
        }
    }

    // Also need access to /tmp for some operations
    if std::path::Path::new("/tmp").exists() {
        if let Ok(fd) = PathFd::new("/tmp") {
            rules_to_add.push(PathBeneath::new(fd, write_access));
        }
    }

    // Add all rules and then restrict self
    // add_rules expects an iterator of Result<T, E>, so wrap in Ok
    let rules_iter = rules_to_add
        .into_iter()
        .map(Ok::<_, landlock::RulesetError>);
    match ruleset.add_rules(rules_iter) {
        Ok(ruleset) => {
            if let Err(e) = ruleset.restrict_self() {
                eprintln!(
                    "Warning: Failed to apply Landlock restrictions: {}. Continuing.",
                    e
                );
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to add Landlock rules: {}. Continuing.", e);
        }
    }

    Ok(())
}

// Note: seccomp implementation removed - causes issues in gVisor and similar environments.
// Seccomp filtering should be handled at the container level instead.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_resource_limits() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_bytes, 64 * 1024 * 1024);
        assert_eq!(limits.max_file_size_bytes, 75 * 1024 * 1024);
        assert_eq!(limits.max_file_descriptors, 10);
        assert_eq!(limits.max_processes, 2);
    }
}
