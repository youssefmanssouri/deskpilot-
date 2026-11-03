use deskpilot_lib::core::ProcessService;
use deskpilot_lib::models::process::{ProcessIdentity, ProcessQueryStatus};
use deskpilot_lib::models::process_dto::ProcessListResultDto;
use deskpilot_lib::platform::windows::{
    RawProcessCandidate, WindowsProcessProvider, filetime_ticks_to_iso8601,
};
use deskpilot_lib::state::AppState;
use std::sync::Arc;
use std::thread;

#[test]
fn test_filetime_ticks_to_iso8601_epoch_math() {
    // 116444736000000000 is the exact number of 100ns intervals between 1601-01-01 and 1970-01-01
    let unix_epoch_ticks = 116_444_736_000_000_000;
    assert_eq!(
        filetime_ticks_to_iso8601(unix_epoch_ticks),
        Some("1970-01-01T00:00:00.000Z".to_string())
    );

    // One second later
    let one_sec_later = unix_epoch_ticks + 10_000_000;
    assert_eq!(
        filetime_ticks_to_iso8601(one_sec_later),
        Some("1970-01-01T00:00:01.000Z".to_string())
    );

    // One millisecond later
    let one_ms_later = unix_epoch_ticks + 10_000;
    assert_eq!(
        filetime_ticks_to_iso8601(one_ms_later),
        Some("1970-01-01T00:00:00.001Z".to_string())
    );

    // Value prior to Unix epoch returns None
    assert_eq!(filetime_ticks_to_iso8601(unix_epoch_ticks - 1), None);
    assert_eq!(filetime_ticks_to_iso8601(0), None);
}

#[test]
fn test_pid_zero_status_is_handle_unavailable() {
    let candidate = RawProcessCandidate {
        pid: 0,
        parent_pid: 0,
        name: "System Idle Process".to_string(),
    };
    let record = WindowsProcessProvider::extract_candidate_detail(&candidate, 100);

    // Per Microsoft specification, OpenProcess always fails for PID 0.
    // It must be classified as HandleUnavailable, NEVER Exited.
    assert_eq!(record.pid, 0);
    assert_eq!(record.status, ProcessQueryStatus::HandleUnavailable);
    assert_eq!(record.id, "eph:100:0");
    assert!(record.executable_path.is_none());
    assert!(record.working_set_bytes.is_none());
}

#[test]
fn test_terminated_nonzero_pid_status_is_exited() {
    // Use an impossibly high or known dead PID (Windows PIDs are multiples of 4 up to a ceiling)
    // 0xFFFFFFFC or an unused random high PID that fails OpenProcess with ERROR_INVALID_PARAMETER
    let dead_candidate = RawProcessCandidate {
        pid: 0xFFFFFFFC,
        parent_pid: 1,
        name: "dead_process.exe".to_string(),
    };
    let record = WindowsProcessProvider::extract_candidate_detail(&dead_candidate, 77);

    // OpenProcess on a non-existent PID returns ERROR_INVALID_PARAMETER (87),
    // which indicates the kernel object has terminated.
    assert_eq!(record.pid, 0xFFFFFFFC);
    assert_eq!(record.status, ProcessQueryStatus::Exited);
    assert_eq!(record.id, "eph:77:4294967292");
    assert!(record.executable_path.is_none());
}

#[test]
fn test_identity_formatting_anchored_and_ephemeral() {
    let anchored = ProcessIdentity::anchored(1234, 0x01db2d5843a0e120);
    assert_eq!(anchored, "v1:1234:01db2d5843a0e120");

    let ephemeral = ProcessIdentity::ephemeral(99, 1234);
    assert_eq!(ephemeral, "eph:99:1234");
    assert_ne!(anchored, ephemeral);
}

#[test]
fn test_snapshot_sequence_allocation_thread_safety() {
    let state = Arc::new(AppState::new("DeskPilotTest", "1.0.0"));
    let mut handles = Vec::new();

    for _ in 0..10 {
        let state_clone = Arc::clone(&state);
        handles.push(thread::spawn(move || {
            let mut seqs = Vec::new();
            for _ in 0..100 {
                seqs.push(state_clone.next_process_snapshot_seq().unwrap());
            }
            seqs
        }));
    }

    let mut all_seqs = Vec::new();
    for h in handles {
        all_seqs.extend(h.join().unwrap());
    }

    assert_eq!(all_seqs.len(), 1000);
    all_seqs.sort_unstable();
    all_seqs.dedup();
    assert_eq!(all_seqs.len(), 1000);
    assert_eq!(all_seqs[0], 1);
    assert_eq!(all_seqs[999], 1000);
}

#[test]
fn test_snapshot_sequence_checked_overflow_policy() {
    let state = AppState::new("DeskPilotTest", "1.0.0");
    // Boundary test: set counter to u64::MAX - 1
    state.set_process_snapshot_seq_for_testing(u64::MAX - 1);
    assert_eq!(state.current_process_snapshot_seq(), u64::MAX - 1);

    // Allocating u64::MAX - 1 succeeds and moves counter to u64::MAX
    let seq = state.next_process_snapshot_seq().unwrap();
    assert_eq!(seq, u64::MAX - 1);
    assert_eq!(state.current_process_snapshot_seq(), u64::MAX);

    // Further allocations fail and do NOT wrap around to 0 or 1
    let err1 = state.next_process_snapshot_seq();
    assert!(err1.is_err(), "Must return Err on sequence exhaustion");
    assert_eq!(state.current_process_snapshot_seq(), u64::MAX);

    let err2 = state.next_process_snapshot_seq();
    assert!(err2.is_err(), "Must stay exhausted without wrapping");
    assert_eq!(state.current_process_snapshot_seq(), u64::MAX);
}

#[test]
fn test_filetime_and_identity_semantics_synthetic() {
    let pid = 4321;
    let seq = 88;

    // Case 1: Valid modern ticks (post-1970)
    let modern_ticks = 116_444_736_000_000_000 + 10_000_000;
    let iso = filetime_ticks_to_iso8601(modern_ticks);
    assert!(iso.is_some());
    let id_modern = ProcessIdentity::resolve(pid, Some(modern_ticks), seq);
    assert_eq!(id_modern, format!("v1:{pid}:{modern_ticks:016x}"));

    // Case 2: Pre-Unix-epoch nonzero ticks (valid raw FILETIME, but ISO conversion unavailable)
    let pre_epoch_ticks = 100_000_000; // prior to 1970
    let iso_pre = filetime_ticks_to_iso8601(pre_epoch_ticks);
    assert!(iso_pre.is_none());
    // Raw nonzero ticks MUST be preserved in anchored identity
    let id_pre = ProcessIdentity::resolve(pid, Some(pre_epoch_ticks), seq);
    assert_eq!(id_pre, format!("v1:{pid}:{pre_epoch_ticks:016x}"));

    // Case 3: Zero ticks (strictly invalid/unrecorded creation time)
    // MUST NOT create an anchored identity from zero ticks
    let id_zero = ProcessIdentity::resolve(pid, Some(0), seq);
    assert_eq!(id_zero, format!("eph:{seq}:{pid}"));

    // Case 4: None ticks (handle unavailable)
    let id_none = ProcessIdentity::resolve(pid, None, seq);
    assert_eq!(id_none, format!("eph:{seq}:{pid}"));
}

#[test]
fn test_bounded_top_k_selection_deterministic_order() {
    let input = vec![
        RawProcessCandidate {
            pid: 400,
            parent_pid: 1,
            name: "chrome.exe".to_string(),
        },
        RawProcessCandidate {
            pid: 100,
            parent_pid: 1,
            name: "Chrome.exe".to_string(),
        },
        RawProcessCandidate {
            pid: 50,
            parent_pid: 1,
            name: "Code.exe".to_string(),
        },
        RawProcessCandidate {
            pid: 200,
            parent_pid: 1,
            name: "Apple.exe".to_string(),
        },
    ];

    let (retained, total) = ProcessService::select_top_k_candidates(input, 10).unwrap();
    assert_eq!(total, 4);
    assert_eq!(retained.len(), 4);

    // Primary: name lowercase ascending; Secondary: PID ascending
    assert_eq!(retained[0].name, "Apple.exe");
    assert_eq!(retained[0].pid, 200);

    assert_eq!(retained[1].name, "Chrome.exe");
    assert_eq!(retained[1].pid, 100);

    assert_eq!(retained[2].name, "chrome.exe");
    assert_eq!(retained[2].pid, 400);

    assert_eq!(retained[3].name, "Code.exe");
    assert_eq!(retained[3].pid, 50);
}

#[test]
fn test_bounded_top_k_selection_truncation_with_10k_synthetic_dataset() {
    // Generate 10,000 synthetic candidates
    let mut input = Vec::with_capacity(10000);
    for i in 0..10000 {
        let name = format!("service_{:05}.exe", (i * 73) % 8000);
        input.push(RawProcessCandidate {
            pid: i as u32,
            parent_pid: 1,
            name,
        });
    }

    // Reference sort: full vector sort and truncate to 2000
    let mut reference = input.clone();
    reference.sort_by(|a, b| {
        let key_a = (a.name.to_lowercase(), a.pid);
        let key_b = (b.name.to_lowercase(), b.pid);
        key_a.cmp(&key_b)
    });
    reference.truncate(2000);

    // Bounded heap selection
    let (retained, total) = ProcessService::select_top_k_candidates(input, 2000).unwrap();
    assert_eq!(total, 10000);
    assert_eq!(retained.len(), 2000);

    // Assert exact equality with reference full sort
    for (i, (retained_item, ref_item)) in retained.iter().zip(reference.iter()).enumerate() {
        assert_eq!(
            retained_item.name, ref_item.name,
            "Name mismatch at index {}",
            i
        );
        assert_eq!(
            retained_item.pid, ref_item.pid,
            "PID mismatch at index {}",
            i
        );
    }
}

#[test]
fn test_live_current_process_self_discovery() {
    let state = AppState::new("DeskPilotTest", "1.0.0");
    let result: ProcessListResultDto = ProcessService::list_processes(&state).unwrap();

    let current_pid = std::process::id();
    assert!(result.total_processes > 0);
    assert!(!result.processes.is_empty());
    assert!(result.captured_at_ms > 0);

    // Search for current process in the snapshot
    let current_proc = result.processes.iter().find(|p| p.pid == current_pid);
    assert!(
        current_proc.is_some(),
        "Current test process (PID {}) must appear in the snapshot",
        current_pid
    );

    let proc = current_proc.unwrap();
    // For the current running process, OpenProcess should have succeeded
    assert!(
        proc.status == ProcessQueryStatus::Full || proc.status == ProcessQueryStatus::Partial,
        "Current process must be Full or Partial, got {:?}",
        proc.status
    );

    // Working set should be positive if query succeeded
    if let Some(ws) = proc.working_set_bytes {
        assert!(ws > 0, "Working set bytes must be positive");
    }

    // Executable path should be populated for the current process
    if let Some(ref path) = proc.executable_path {
        assert!(
            path.to_lowercase().ends_with(".exe"),
            "Executable path must end with .exe: {}",
            path
        );
    }
}

#[test]
fn test_live_access_denied_graceful_handling() {
    let state = AppState::new("DeskPilotTest", "1.0.0");
    let result = ProcessService::list_processes(&state).unwrap();

    // Verify all processes have valid PIDs and non-empty names
    for p in &result.processes {
        assert!(!p.name.is_empty(), "Process name must not be empty");
        // Verify statuses are valid
        match p.status {
            ProcessQueryStatus::Full => {
                assert!(p.executable_path.is_some());
                assert!(
                    p.created_at.is_some(),
                    "Full status requires a valid created_at timestamp"
                );
                assert!(
                    p.created_at.is_some(),
                    "Full status requires a valid created_at timestamp"
                );
                assert!(p.working_set_bytes.is_some());
            }
            ProcessQueryStatus::Partial => {
                // At least one detail query was unavailable
            }
            ProcessQueryStatus::HandleUnavailable => {
                // Access restricted or PID 0; details should be None
                assert!(p.executable_path.is_none());
                assert!(p.working_set_bytes.is_none());
            }
            ProcessQueryStatus::Exited => {
                // Terminated before handle acquisition; details should be None
                assert!(p.executable_path.is_none());
                assert!(p.working_set_bytes.is_none());
            }
        }
    }
}

#[test]
fn test_repeated_snapshots_stability() {
    let state = AppState::new("DeskPilotTest", "1.0.0");

    let mut last_captured_at = 0;
    for _ in 0..5 {
        let res = ProcessService::list_processes(&state).unwrap();
        assert!(res.total_processes > 0);
        assert!(res.captured_at_ms >= last_captured_at);
        last_captured_at = res.captured_at_ms;
    }
}
