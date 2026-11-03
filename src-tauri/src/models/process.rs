use serde::{Deserialize, Serialize};

/// Detailed factual status describing what the operating system permitted
/// during process query and handle acquisition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessQueryStatus {
    /// `OpenProcess` succeeded with `PROCESS_QUERY_LIMITED_INFORMATION`, and all
    /// primary detail queries (`QueryFullProcessImageNameW`, `GetProcessTimes`, `K32GetProcessMemoryInfo`) succeeded.
    Full,
    /// `OpenProcess` succeeded, but at least one detail query (e.g. path or memory) returned an error.
    /// Succeeded fields remain populated.
    Partial,
    /// `OpenProcess` failed due to operating system access restrictions (`ERROR_ACCESS_DENIED`, etc.),
    /// or special pseudo-process constraints (PID 0 returning `ERROR_INVALID_PARAMETER` per Microsoft specification).
    /// Snapshot metadata (`pid`, `name`, `parent_pid`) remains valid.
    HandleUnavailable,
    /// `OpenProcess` on a nonzero PID returned `ERROR_INVALID_PARAMETER` (87), indicating the process
    /// was enumerated in the snapshot but terminated before a query handle could be opened.
    Exited,
}

/// Internal domain representation of an enumerated Windows process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRecord {
    /// Deterministic process identity token (`v1:{pid}:{creationTicksHex}` or `eph:{snapshotSeq}:{pid}`).
    pub id: String,
    /// Operating system Process Identifier (PID).
    pub pid: u32,
    /// Historical parent process identifier reported at creation time by Toolhelp32.
    pub parent_pid: u32,
    /// Base executable image name (e.g., "code.exe", "explorer.exe").
    pub name: String,
    /// Full canonical executable path if accessible.
    pub executable_path: Option<String>,
    /// Raw 64-bit Windows FILETIME ticks (100-nanosecond intervals since 1601-01-01).
    pub creation_time_ticks: Option<u64>,
    /// Process creation timestamp formatted as an ISO-8601 UTC string.
    pub created_at: Option<String>,
    /// Physical resident memory (working set) in bytes.
    pub working_set_bytes: Option<u64>,
    /// Cumulative CPU execution time in milliseconds (kernel + user).
    pub cpu_time_ms: Option<u64>,
    /// Terminal Services session identifier (e.g. 1 for interactive user, 0 for system services).
    pub session_id: Option<u32>,
    /// Factual query status returned by the operating system.
    pub status: ProcessQueryStatus,
}

/// Utility for generating standardized, non-authoritative process identity keys.
///
/// # Security & Authorization Boundary
/// Process identity strings are strictly UI reconciliation keys and client-side diff anchors.
/// They are NEVER authorization tokens or parameters for privileged backend actions.
///
/// # Semantics & Guarantees
/// - Anchored identities (`v1:{pid}:{creationTicksHex}`) incorporate nonzero 64-bit creation FILETIME ticks
///   to guard against accidental UI misattribution under typical PID recycling.
/// - No claims are made regarding kernel-level cryptographic uniqueness or sub-tick clock resolution.
/// - When raw creation ticks are zero, invalid, or inaccessible, an ephemeral identity (`eph:{snapshotSeq}:{pid}`)
///   is issued, scoped strictly to the snapshot sequence to prevent false continuity across scans.
pub struct ProcessIdentity;

impl ProcessIdentity {
    /// Generates an anchored identity from valid nonzero creation FILETIME ticks.
    ///
    /// Format: `v1:{pid}:{creationTicksHex}` (e.g. `v1:14820:01db2d5843a0e120`).
    /// Preserves raw 64-bit ticks with full 16-hex digit precision.
    pub fn anchored(pid: u32, creation_ticks: u64) -> String {
        format!("v1:{pid}:{creation_ticks:016x}")
    }

    /// Generates an ephemeral identity when process handle opening or creation-time retrieval failed.
    ///
    /// Format: `eph:{snapshotSeq}:{pid}` (e.g. `eph:42:14820`).
    /// Scoped strictly to snapshot sequence `snapshot_seq`, preventing false continuity across scans.
    pub fn ephemeral(snapshot_seq: u64, pid: u32) -> String {
        format!("eph:{snapshot_seq}:{pid}")
    }

    /// Resolves the canonical identity for a process:
    /// Returns an anchored identity when nonzero creation ticks are present,
    /// or falls back to an ephemeral identity when ticks are None or zero.
    pub fn resolve(pid: u32, creation_ticks: Option<u64>, snapshot_seq: u64) -> String {
        match creation_ticks {
            Some(ticks) if ticks > 0 => Self::anchored(pid, ticks),
            _ => Self::ephemeral(snapshot_seq, pid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_identity_anchored_formatting() {
        let id = ProcessIdentity::anchored(1234, 0x01db2d5843a0e120);
        assert_eq!(id, "v1:1234:01db2d5843a0e120");

        // Preserves leading zeros in lower 64-bit ticks
        let id_low = ProcessIdentity::anchored(42, 0x0000000000000001);
        assert_eq!(id_low, "v1:42:0000000000000001");
    }

    #[test]
    fn test_process_identity_anchored_distinctness() {
        let id1 = ProcessIdentity::anchored(100, 0x1000);
        let id2 = ProcessIdentity::anchored(100, 0x2000);
        let id3 = ProcessIdentity::anchored(200, 0x1000);
        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_process_identity_ephemeral_formatting() {
        let id = ProcessIdentity::ephemeral(5, 1234);
        assert_eq!(id, "eph:5:1234");
    }

    #[test]
    fn test_process_identity_ephemeral_scoped_across_snapshots() {
        let snap1 = ProcessIdentity::ephemeral(1, 100);
        let snap2 = ProcessIdentity::ephemeral(2, 100);
        assert_ne!(snap1, snap2);
    }

    #[test]
    fn test_process_identity_resolve_semantics() {
        // Nonzero ticks -> Anchored
        let id_valid = ProcessIdentity::resolve(100, Some(0x01db2d5843a0e120), 42);
        assert_eq!(id_valid, "v1:100:01db2d5843a0e120");

        // Zero ticks -> Ephemeral (do not anchor to zero ticks)
        let id_zero = ProcessIdentity::resolve(100, Some(0), 42);
        assert_eq!(id_zero, "eph:42:100");

        // None ticks -> Ephemeral
        let id_none = ProcessIdentity::resolve(100, None, 42);
        assert_eq!(id_none, "eph:42:100");
    }

    #[test]
    fn test_process_query_status_serde_camel_case() {
        assert_eq!(
            serde_json::to_string(&ProcessQueryStatus::Full).unwrap(),
            r#""full""#
        );
        assert_eq!(
            serde_json::to_string(&ProcessQueryStatus::Partial).unwrap(),
            r#""partial""#
        );
        assert_eq!(
            serde_json::to_string(&ProcessQueryStatus::HandleUnavailable).unwrap(),
            r#""handleUnavailable""#
        );
        assert_eq!(
            serde_json::to_string(&ProcessQueryStatus::Exited).unwrap(),
            r#""exited""#
        );
    }
}
