use super::process::{ProcessQueryStatus, ProcessRecord};
use serde::{Deserialize, Serialize};

/// Safe frontend Data Transfer Object representing a discovered Windows process.
///
/// Encapsulates process metadata without leaking unverified machine internals
/// or granting executable capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDto {
    /// Deterministic process identity token (`v1:{pid}:{creationTicksHex}` or `eph:{snapshotSeq}:{pid}`).
    pub id: String,
    /// Operating system Process Identifier (PID).
    pub pid: u32,
    /// Historical parent process identifier reported at creation time by Toolhelp32.
    pub parent_pid: u32,
    /// Base executable image name (e.g. "code.exe", "explorer.exe").
    pub name: String,
    /// Full canonical executable path if accessible, otherwise `None`.
    pub executable_path: Option<String>,
    /// Process creation time formatted as an ISO-8601 UTC string if accessible.
    pub created_at: Option<String>,
    /// Physical resident memory (working set) in bytes if accessible.
    pub working_set_bytes: Option<u64>,
    /// Cumulative CPU execution time in milliseconds (kernel + user) if accessible.
    pub cpu_time_ms: Option<u64>,
    /// Terminal Services session identifier if accessible.
    pub session_id: Option<u32>,
    /// Factual query status returned by the operating system.
    pub status: ProcessQueryStatus,
}

impl From<ProcessRecord> for ProcessDto {
    fn from(record: ProcessRecord) -> Self {
        Self {
            id: record.id,
            pid: record.pid,
            parent_pid: record.parent_pid,
            name: record.name,
            executable_path: record.executable_path,
            created_at: record.created_at,
            working_set_bytes: record.working_set_bytes,
            cpu_time_ms: record.cpu_time_ms,
            session_id: record.session_id,
            status: record.status,
        }
    }
}

/// Payload returned by the process intelligence service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessListResultDto {
    /// Deterministically ordered list of at most K retained processes.
    pub processes: Vec<ProcessDto>,
    /// Exact count of all processes enumerated in the snapshot walk before truncation.
    pub total_processes: usize,
    /// Point in time (Unix epoch milliseconds) when snapshot acquisition succeeded.
    pub captured_at_ms: u64,
    /// True if total observed processes exceeded the maximum retention ceiling (2,000).
    pub is_truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_dto_serialization_camel_case() {
        let dto = ProcessDto {
            id: "v1:1024:01db2d5843a0e120".to_string(),
            pid: 1024,
            parent_pid: 512,
            name: "test.exe".to_string(),
            executable_path: Some(r"C:\Windows\System32\test.exe".to_string()),
            created_at: Some("2026-11-03T12:00:00.000Z".to_string()),
            working_set_bytes: Some(10485760),
            cpu_time_ms: Some(250),
            session_id: Some(1),
            status: ProcessQueryStatus::Full,
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains(r#""id":"v1:1024:01db2d5843a0e120""#));
        assert!(json.contains(r#""pid":1024"#));
        assert!(json.contains(r#""parentPid":512"#));
        assert!(json.contains(r#""name":"test.exe""#));
        assert!(json.contains(r#""executablePath":"C:\\Windows\\System32\\test.exe""#));
        assert!(json.contains(r#""createdAt":"2026-11-03T12:00:00.000Z""#));
        assert!(json.contains(r#""workingSetBytes":10485760"#));
        assert!(json.contains(r#""cpuTimeMs":250"#));
        assert!(json.contains(r#""sessionId":1"#));
        assert!(json.contains(r#""status":"full""#));
    }

    #[test]
    fn test_process_list_result_dto_serialization_fields() {
        let result = ProcessListResultDto {
            processes: vec![],
            total_processes: 42,
            captured_at_ms: 1730635200000,
            is_truncated: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""processes":[]"#));
        assert!(json.contains(r#""totalProcesses":42"#));
        assert!(json.contains(r#""capturedAtMs":1730635200000"#));
        assert!(json.contains(r#""isTruncated":false"#));
    }

    #[test]
    fn test_from_process_record_mapping() {
        let record = ProcessRecord {
            id: "eph:1:42".to_string(),
            pid: 42,
            parent_pid: 0,
            name: "sample.exe".to_string(),
            executable_path: None,
            creation_time_ticks: None,
            created_at: None,
            working_set_bytes: None,
            cpu_time_ms: None,
            session_id: None,
            status: ProcessQueryStatus::HandleUnavailable,
        };

        let dto: ProcessDto = record.into();
        assert_eq!(dto.id, "eph:1:42");
        assert_eq!(dto.pid, 42);
        assert_eq!(dto.parent_pid, 0);
        assert_eq!(dto.name, "sample.exe");
        assert_eq!(dto.status, ProcessQueryStatus::HandleUnavailable);
        assert!(dto.executable_path.is_none());
    }
}
