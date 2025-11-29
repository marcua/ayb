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
    // Apply rlimit restrictions (works on all Unix platforms)
    #[cfg(unix)]
    apply_rlimits(limits)?;

    // Apply Landlock filesystem restrictions (Linux 5.13+)
    #[cfg(target_os = "linux")]
    apply_landlock(db_path)?;

    // Apply seccomp syscall filtering (Linux only)
    #[cfg(target_os = "linux")]
    apply_seccomp()?;

    // Suppress unused variable warning on non-Unix platforms
    #[cfg(not(unix))]
    let _ = (db_path, limits);

    Ok(())
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

/// Apply seccomp syscall filtering (Linux only)
/// This blocks dangerous syscalls that SQLite doesn't need
#[cfg(target_os = "linux")]
fn apply_seccomp() -> Result<(), AybError> {
    #[allow(unused_imports)]
    use libc;
    use seccompiler::{apply_filter_all_threads, SeccompAction, SeccompFilter, SeccompRule};
    use std::collections::BTreeMap;

    // Build the filter rules - we'll block dangerous syscalls
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();

    // Block dangerous syscalls that SQLite doesn't need
    // These could be used for container escape or privilege escalation

    // Process/namespace manipulation
    rules.insert(libc::SYS_ptrace, vec![]);
    rules.insert(libc::SYS_mount, vec![]);
    rules.insert(libc::SYS_umount2, vec![]);
    rules.insert(libc::SYS_chroot, vec![]);
    rules.insert(libc::SYS_pivot_root, vec![]);
    rules.insert(libc::SYS_unshare, vec![]);
    rules.insert(libc::SYS_setns, vec![]);

    // Network syscalls - SQLite doesn't need network
    rules.insert(libc::SYS_socket, vec![]);
    rules.insert(libc::SYS_connect, vec![]);
    rules.insert(libc::SYS_bind, vec![]);
    rules.insert(libc::SYS_listen, vec![]);
    rules.insert(libc::SYS_accept, vec![]);
    rules.insert(libc::SYS_accept4, vec![]);
    rules.insert(libc::SYS_sendto, vec![]);
    rules.insert(libc::SYS_recvfrom, vec![]);
    rules.insert(libc::SYS_sendmsg, vec![]);
    rules.insert(libc::SYS_recvmsg, vec![]);

    // Module loading
    rules.insert(libc::SYS_init_module, vec![]);
    rules.insert(libc::SYS_finit_module, vec![]);
    rules.insert(libc::SYS_delete_module, vec![]);

    // Kernel keyring
    rules.insert(libc::SYS_add_key, vec![]);
    rules.insert(libc::SYS_request_key, vec![]);
    rules.insert(libc::SYS_keyctl, vec![]);

    // BPF - could be used to bypass seccomp
    rules.insert(libc::SYS_bpf, vec![]);

    // Performance monitoring
    rules.insert(libc::SYS_perf_event_open, vec![]);

    // Get the target architecture
    #[cfg(target_arch = "x86_64")]
    let arch = seccompiler::TargetArch::x86_64;
    #[cfg(target_arch = "aarch64")]
    let arch = seccompiler::TargetArch::aarch64;

    // Create the filter with default allow
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Errno(libc::EPERM as u32), // Return EPERM for blocked syscalls
        SeccompAction::Allow,                     // Allow everything else
        arch,
    )
    .map_err(|e| AybError::Other {
        message: format!("Failed to create seccomp filter: {:?}", e),
    })?;

    // Compile to BPF
    let bpf_prog: seccompiler::BpfProgram = filter.try_into().map_err(|e| AybError::Other {
        message: format!("Failed to compile seccomp filter: {:?}", e),
    })?;

    // Apply the filter to all threads
    apply_filter_all_threads(&bpf_prog).map_err(|e| AybError::Other {
        message: format!("Failed to apply seccomp filter: {:?}", e),
    })?;

    Ok(())
}

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
