use crate::error::AybError;
use std::env::current_exe;
use std::path::{Path, PathBuf};

use crate::hosted_db::paths::pathbuf_to_parent;

/// Apply Landlock filesystem and network restrictions, plus resource
/// limits via setrlimit, to the current process. This is called by the
/// query daemon at startup, so the daemon sandboxes itself before
/// processing any queries.
///
/// On Linux with Landlock enforced (kernel 5.13+):
/// - Filesystem: only the database file (read-write) and shared
///   libraries (read-only) are accessible.
/// - Network: all TCP bind/connect denied (on kernel 6.7+).
/// - Memory: 64 MB virtual memory limit (RLIMIT_AS).
/// - File size: 75 MB max file size (RLIMIT_FSIZE).
/// - File descriptors: 10 max open files (RLIMIT_NOFILE).
///
/// On any other platform or older Linux kernel, a loud warning is
/// printed at startup and the daemon runs without isolation.
///
/// Configurable per-database limits and per-process CPU/thread
/// limitation is future work.
pub fn apply_sandbox(db_path: &Path) -> Result<(), AybError> {
    #[cfg(target_os = "linux")]
    {
        apply_landlock_restrictions(db_path)?;
        apply_resource_limits()?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = db_path;
        print_unsandboxed_warning("Landlock is unavailable on this non-Linux platform");
    }
    Ok(())
}

/// Print a loud, multi-line warning to stderr when the daemon cannot
/// enforce isolation. Meant to be visible in both terminals and log
/// aggregators. Do not run multi-tenant workloads when this fires.
pub fn print_unsandboxed_warning(reason: &str) {
    eprintln!("======================================================================");
    eprintln!("WARNING: ayb query daemon is running WITHOUT isolation.");
    eprintln!("Reason: {reason}");
    eprintln!("Filesystem and network access are NOT restricted for this daemon.");
    eprintln!("Do NOT run multi-tenant workloads in this configuration.");
    eprintln!("See https://github.com/marcua/ayb#isolation for details.");
    eprintln!("======================================================================");
}

/// Apply Landlock filesystem and network restrictions.
#[cfg(target_os = "linux")]
fn apply_landlock_restrictions(db_path: &Path) -> Result<(), AybError> {
    use landlock::{
        path_beneath_rules, Access, AccessFs, AccessNet, Ruleset, RulesetAttr, RulesetCreatedAttr,
        RulesetStatus, ABI,
    };

    // Use the highest ABI we can, with best-effort degradation.
    let abi = ABI::V5;

    let access_all = AccessFs::from_all(abi);
    let access_read = AccessFs::from_read(abi);

    let mut ruleset =
        Ruleset::default()
            .handle_access(access_all)
            .map_err(|e| AybError::Other {
                message: format!("Landlock: failed to handle filesystem access: {e}"),
            })?;

    // Handle network access if supported (ABI v4+, kernel 6.7+).
    // On older kernels, AccessNet::from_all() returns empty flags and
    // handle_access would error, so we skip it gracefully.
    let access_net = AccessNet::from_all(abi);
    let network_supported = !access_net.is_empty();
    if network_supported {
        ruleset = ruleset
            .handle_access(access_net)
            .map_err(|e| AybError::Other {
                message: format!("Landlock: failed to handle network access: {e}"),
            })?;
    }

    let mut ruleset_created = ruleset.create().map_err(|e| AybError::Other {
        message: format!("Landlock: failed to create ruleset: {e}"),
    })?;

    // Allow read-only access to shared libraries and system paths.
    let read_only_paths: Vec<&str> = vec!["/lib", "/lib64", "/usr"];
    let existing_read_only: Vec<&str> = read_only_paths
        .into_iter()
        .filter(|p| Path::new(p).exists())
        .collect();

    if !existing_read_only.is_empty() {
        ruleset_created = ruleset_created
            .add_rules(path_beneath_rules(existing_read_only, access_read))
            .map_err(|e| AybError::Other {
                message: format!("Landlock: failed to add read-only rules: {e}"),
            })?;
    }

    // Allow read-write access to the database file's parent directory.
    // SQLite needs access to the directory for journal/WAL files.
    let db_dir = db_path.parent().ok_or(AybError::Other {
        message: format!(
            "Cannot determine parent directory of database: {}",
            db_path.display()
        ),
    })?;
    ruleset_created = ruleset_created
        .add_rules(path_beneath_rules(&[db_dir], access_all))
        .map_err(|e| AybError::Other {
            message: format!("Landlock: failed to add database directory rule: {e}"),
        })?;

    // No network rules added = all TCP bind/connect denied (if network
    // access was handled above).

    let status = ruleset_created
        .restrict_self()
        .map_err(|e| AybError::Other {
            message: format!("Landlock: failed to restrict self: {e}"),
        })?;

    if status.ruleset == RulesetStatus::NotEnforced {
        print_unsandboxed_warning("Landlock is not enforced on this kernel (requires Linux 5.13+)");
    } else if !network_supported {
        eprintln!(
            "Note: Landlock network isolation unavailable on this kernel \
             (requires Linux 6.7+). Filesystem isolation is active."
        );
    }

    Ok(())
}

/// Apply resource limits via setrlimit.
#[cfg(target_os = "linux")]
fn apply_resource_limits() -> Result<(), AybError> {
    set_rlimit(libc::RLIMIT_AS, 64 * 1024 * 1024)?; // 64 MB memory
    set_rlimit(libc::RLIMIT_FSIZE, 75 * 1024 * 1024)?; // 75 MB file size
    set_rlimit(libc::RLIMIT_NOFILE, 10)?; // 10 file descriptors
    Ok(())
}

#[cfg(target_os = "linux")]
fn set_rlimit(resource: libc::__rlimit_resource_t, limit: u64) -> Result<(), AybError> {
    let rlim = libc::rlimit {
        rlim_cur: limit,
        rlim_max: limit,
    };
    let ret = unsafe { libc::setrlimit(resource, &rlim) };
    if ret != 0 {
        return Err(AybError::Other {
            message: format!(
                "Failed to set resource limit {}: {}",
                resource,
                std::io::Error::last_os_error()
            ),
        });
    }
    Ok(())
}

/// Build command for running the query daemon.
pub fn build_daemon_command(db_path: &PathBuf) -> Result<tokio::process::Command, AybError> {
    let ayb_path = current_exe()?;
    let query_daemon_path = pathbuf_to_parent(&ayb_path)?.join("ayb_query_daemon");

    let mut cmd = tokio::process::Command::new(&query_daemon_path);
    cmd.arg(db_path);

    Ok(cmd)
}
