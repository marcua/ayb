//! Sandbox capabilities detection and management.
//!
//! This module provides a multi-layer defense approach to database isolation
//! that works in Docker containers and on various Linux kernels.
//!
//! Layers:
//! 1. SQLite Authorizer - Blocks ATTACH DATABASE and dangerous PRAGMAs
//! 2. rlimit - Memory, file size, file descriptor, and process limits
//! 3. Landlock - Filesystem isolation (Linux 5.13+)
//! 4. seccomp - Syscall filtering
//! 5. cgroups v2 - CPU limits (optional)

use std::fmt;

/// Represents the available sandbox capabilities on the current system
#[derive(Debug, Clone)]
pub struct SandboxCapabilities {
    /// Landlock ABI version if available (Linux 5.13+)
    pub landlock_abi: Option<u8>,
    /// Whether cgroups v2 is available and writable
    pub cgroups_v2: bool,
    /// Whether rlimit is available (always true on Unix)
    pub rlimit: bool,
    /// Whether seccomp is available
    pub seccomp: bool,
    /// The current platform
    pub platform: Platform,
}

/// The platform we're running on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Unknown,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::Linux => write!(f, "Linux"),
            Platform::MacOS => write!(f, "macOS"),
            Platform::Windows => write!(f, "Windows"),
            Platform::Unknown => write!(f, "Unknown"),
        }
    }
}

impl SandboxCapabilities {
    /// Detect available sandbox capabilities on the current system
    pub fn detect() -> Self {
        let platform = if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "macos") {
            Platform::MacOS
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else {
            Platform::Unknown
        };

        let landlock_abi = Self::detect_landlock();
        let cgroups_v2 = Self::detect_cgroups_v2();
        let rlimit = cfg!(unix);
        let seccomp = cfg!(target_os = "linux");

        SandboxCapabilities {
            landlock_abi,
            cgroups_v2,
            rlimit,
            seccomp,
            platform,
        }
    }

    /// Check if Landlock is available
    #[cfg(target_os = "linux")]
    fn detect_landlock() -> Option<u8> {
        use landlock::{Access, AccessFs, Ruleset, RulesetAttr, ABI};

        // Try to create a minimal ruleset to detect Landlock support
        // We try different ABIs from newest to oldest
        for abi in [ABI::V5, ABI::V4, ABI::V3, ABI::V2, ABI::V1] {
            if Ruleset::default()
                .handle_access(AccessFs::from_all(abi))
                .is_ok()
            {
                return Some(abi as u8);
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    fn detect_landlock() -> Option<u8> {
        None
    }

    /// Check if cgroups v2 is available and writable
    #[cfg(target_os = "linux")]
    fn detect_cgroups_v2() -> bool {
        use std::fs;
        use std::path::Path;

        // Check if cgroups v2 is mounted
        let cgroup_path = Path::new("/sys/fs/cgroup");
        if !cgroup_path.exists() {
            return false;
        }

        // Check for cgroups v2 unified hierarchy
        let controllers_path = cgroup_path.join("cgroup.controllers");
        if !controllers_path.exists() {
            return false;
        }

        // Check if we can read the controllers (indicates access)
        fs::read_to_string(&controllers_path).is_ok()
    }

    #[cfg(not(target_os = "linux"))]
    fn detect_cgroups_v2() -> bool {
        false
    }

    /// Check if full isolation is available
    pub fn has_full_isolation(&self) -> bool {
        self.platform == Platform::Linux && self.landlock_abi.is_some() && self.seccomp
    }

    /// Print sandbox status at server startup
    pub fn print_startup_status(&self) {
        match self.platform {
            Platform::Linux => {
                if self.landlock_abi.is_none() {
                    eprintln!(
                        "Warning: Landlock not available (kernel < 5.13). \
                         Database filesystem isolation will be LIMITED. \
                         Upgrade to kernel 5.13+ for full isolation."
                    );
                }

                if !self.cgroups_v2 {
                    eprintln!(
                        "Warning: cgroups v2 not available or not writable. \
                         CPU limits will NOT be enforced. \
                         For Docker: run with --cgroupns=host or enable cgroup delegation."
                    );
                }

                if !self.seccomp {
                    eprintln!(
                        "Warning: seccomp not available. Syscall filtering will be disabled."
                    );
                }

                // Success message if all available
                if self.landlock_abi.is_some() && self.seccomp {
                    println!("Multi-tenant isolation enabled:");
                    if let Some(abi) = self.landlock_abi {
                        println!("  - Landlock ABI v{} (filesystem isolation)", abi);
                    }
                    if self.cgroups_v2 {
                        println!("  - cgroups v2 (CPU limits)");
                    }
                    println!("  - rlimit (memory/file/process limits)");
                    println!("  - seccomp (syscall filtering)");
                    println!("  - SQLite authorizer (ATTACH blocking)");
                }
            }

            Platform::MacOS => {
                eprintln!("Warning: Running on macOS with limited sandboxing:");
                eprintln!("  - rlimit: Available (memory/file/process limits)");
                eprintln!("  - Landlock: Linux-only");
                eprintln!("  - cgroups: Linux-only");
                eprintln!("  - seccomp: Linux-only");
                eprintln!("  - SQLite authorizer: Available (ATTACH blocking)");
                eprintln!();
                eprintln!("NOT RECOMMENDED for multi-tenant production use.");
                eprintln!("Use Linux for proper database isolation.");
            }

            Platform::Windows => {
                eprintln!("Warning: Running on Windows with minimal sandboxing:");
                eprintln!("  - SQLite authorizer: Available (ATTACH blocking)");
                eprintln!("  - All Linux security features: Unavailable");
                eprintln!();
                eprintln!("NOT RECOMMENDED for multi-tenant production use.");
                eprintln!("Use Linux for proper database isolation.");
            }

            Platform::Unknown => {
                eprintln!("Warning: Unknown platform - sandboxing unavailable");
            }
        }
    }
}

/// Resource limits configuration for sandboxed processes
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum virtual memory in bytes (default: 64 MB)
    pub memory_bytes: u64,
    /// Maximum file size in bytes (default: 75 MB)
    pub max_file_size_bytes: u64,
    /// Maximum number of open file descriptors (default: 10)
    pub max_file_descriptors: u64,
    /// Maximum number of processes (default: 2)
    pub max_processes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 64 * 1024 * 1024,        // 64 MB
            max_file_size_bytes: 75 * 1024 * 1024, // 75 MB
            max_file_descriptors: 10,
            max_processes: 2,
        }
    }
}
