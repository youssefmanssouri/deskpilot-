use crate::error::DomainError;
use crate::models::process_dto::{ProcessDto, ProcessListResultDto};
use crate::platform::windows::{RawProcessCandidate, WindowsProcessProvider};
use crate::state::AppState;
use std::collections::BinaryHeap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum number of process records retained and returned across the IPC boundary.
pub const MAX_RETAINED_PROCESSES: usize = 2_000;

/// Bounded candidate container for Max-Heap top-K selection.
///
/// Implements `Ord` such that the "worst" (largest sort key) element sits at the root of the heap.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BoundedCandidate {
    /// Composite sort key: (case-insensitive name, pid ascending).
    pub sort_key: (String, u32),
    /// Lightweight process candidate metadata.
    pub raw: RawProcessCandidate,
}

impl Ord for BoundedCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Standard max-heap ordering: largest sort_key is at top
        self.sort_key.cmp(&other.sort_key)
    }
}

impl PartialOrd for BoundedCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Core domain service for Process Intelligence.
///
/// Orchestrates bounded Toolhelp32 process enumeration, deterministic Top-K selection,
/// limited-privilege detail extraction, and timestamping.
pub struct ProcessService;

impl ProcessService {
    /// Captures a point-in-time process snapshot with bounded memory and deterministic selection.
    ///
    /// # Architectural Guarantees:
    /// 1. Candidate retention memory remains strictly $O(K)$ where $K = 2{,}000$, independent of system process count.
    /// 2. `total_processes` reflects the exact number of entries enumerated during the snapshot walk.
    /// 3. `is_truncated == (total_processes > processes.len())`.
    /// 4. Processes outside the top 2,000 never receive `OpenProcess` calls.
    /// 5. Capture timestamp reflects the point in time (Unix epoch milliseconds) when snapshot acquisition began.
    pub fn list_processes(state: &AppState) -> Result<ProcessListResultDto, DomainError> {
        let snapshot_seq = state.next_process_snapshot_seq()?;
        let captured_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut heap: BinaryHeap<BoundedCandidate> =
            BinaryHeap::with_capacity(MAX_RETAINED_PROCESSES);

        // Pass 1: Stream Toolhelp32 entries directly into the bounded Max-Heap.
        // Memory used for candidate retention remains strictly <= MAX_RETAINED_PROCESSES.
        let total_observed = WindowsProcessProvider::enumerate_snapshot_entries(|raw| {
            let sort_key = (raw.name.to_lowercase(), raw.pid);
            let candidate = BoundedCandidate { sort_key, raw };

            if heap.len() < MAX_RETAINED_PROCESSES {
                heap.push(candidate);
            } else if let Some(worst) = heap.peek() {
                if candidate.sort_key < worst.sort_key {
                    heap.pop();
                    heap.push(candidate);
                }
            }
            Ok(())
        })?;

        // Convert heap to deterministic sorted order (ascending by name_lowercase, then pid)
        let retained_candidates = Self::finalize_heap_selection(heap);
        let is_truncated = total_observed > retained_candidates.len();

        // Pass 2: Extract detailed attributes ONLY for the retained candidates (<= 2,000).
        let mut processes = Vec::with_capacity(retained_candidates.len());
        for candidate in retained_candidates {
            let record = WindowsProcessProvider::extract_candidate_detail(&candidate, snapshot_seq);
            processes.push(ProcessDto::from(record));
        }

        Ok(ProcessListResultDto {
            processes,
            total_processes: total_observed,
            captured_at_ms,
            is_truncated,
        })
    }

    /// Pure deterministic Top-K selection algorithm extracted for deterministic unit testing.
    ///
    /// Consumes an iterator of `RawProcessCandidate` and produces at most `k` retained candidates,
    /// sorted deterministically by `(name.to_lowercase(), pid)` ascending, along with the exact count.
    pub fn select_top_k_candidates<I>(
        candidates: I,
        k: usize,
    ) -> Result<(Vec<RawProcessCandidate>, usize), DomainError>
    where
        I: IntoIterator<Item = RawProcessCandidate>,
    {
        let mut heap: BinaryHeap<BoundedCandidate> =
            BinaryHeap::with_capacity(k.min(MAX_RETAINED_PROCESSES));
        let mut total_observed: usize = 0;

        for raw in candidates {
            total_observed = total_observed.checked_add(1).ok_or_else(|| {
                DomainError::OperationFailed("Process count arithmetic overflow".to_string())
            })?;

            let sort_key = (raw.name.to_lowercase(), raw.pid);
            let candidate = BoundedCandidate { sort_key, raw };

            if heap.len() < k {
                heap.push(candidate);
            } else if let Some(worst) = heap.peek() {
                if candidate.sort_key < worst.sort_key {
                    heap.pop();
                    heap.push(candidate);
                }
            }
        }

        let sorted = Self::finalize_heap_selection(heap);
        Ok((sorted, total_observed))
    }

    /// Converts the Max-Heap into an ascending sorted vector.
    fn finalize_heap_selection(heap: BinaryHeap<BoundedCandidate>) -> Vec<RawProcessCandidate> {
        // BinaryHeap::into_sorted_vec() returns elements in ASCENDING order according to Ord.
        // Because our Ord is self.sort_key.cmp(&other.sort_key), this returns elements from
        // smallest sort_key to largest sort_key (exact deterministic ascending order).
        heap.into_sorted_vec().into_iter().map(|c| c.raw).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top_k_selection_deterministic_ordering() {
        let input = vec![
            RawProcessCandidate {
                pid: 100,
                parent_pid: 1,
                name: "Zeta.exe".to_string(),
            },
            RawProcessCandidate {
                pid: 20,
                parent_pid: 1,
                name: "alpha.exe".to_string(),
            },
            RawProcessCandidate {
                pid: 10,
                parent_pid: 1,
                name: "Alpha.exe".to_string(),
            },
            RawProcessCandidate {
                pid: 50,
                parent_pid: 1,
                name: "beta.exe".to_string(),
            },
        ];

        let (retained, total) = ProcessService::select_top_k_candidates(input, 10).unwrap();
        assert_eq!(total, 4);
        assert_eq!(retained.len(), 4);

        // Alpha.exe with PID 10, then alpha.exe with PID 20, then beta.exe, then Zeta.exe
        assert_eq!(retained[0].name, "Alpha.exe");
        assert_eq!(retained[0].pid, 10);
        assert_eq!(retained[1].name, "alpha.exe");
        assert_eq!(retained[1].pid, 20);
        assert_eq!(retained[2].name, "beta.exe");
        assert_eq!(retained[3].name, "Zeta.exe");
    }

    #[test]
    fn test_top_k_selection_truncation_bound() {
        // Generate 3,000 synthetic candidates
        let mut input = Vec::with_capacity(3000);
        for i in (0..3000).rev() {
            input.push(RawProcessCandidate {
                pid: i as u32,
                parent_pid: 1,
                name: format!("proc_{:04}.exe", i),
            });
        }

        let (retained, total) = ProcessService::select_top_k_candidates(input, 2000).unwrap();
        assert_eq!(total, 3000);
        assert_eq!(retained.len(), 2000);

        // Retained candidates must be the top 2000 (proc_0000.exe to proc_1999.exe)
        assert_eq!(retained[0].name, "proc_0000.exe");
        assert_eq!(retained[1999].name, "proc_1999.exe");
    }

    #[test]
    fn test_top_k_selection_matches_reference_full_sort() {
        use std::collections::HashSet;

        // Generate 10,000 random-ish candidates
        let mut input = Vec::with_capacity(10000);
        for i in 0..10000 {
            // pseudo-random pseudo-names
            let name = format!("app_{:04}.exe", (i * 37) % 5000);
            input.push(RawProcessCandidate {
                pid: i as u32,
                parent_pid: 1,
                name,
            });
        }

        // Reference full sort implementation
        let mut reference = input.clone();
        reference.sort_by(|a, b| {
            let key_a = (a.name.to_lowercase(), a.pid);
            let key_b = (b.name.to_lowercase(), b.pid);
            key_a.cmp(&key_b)
        });
        reference.truncate(2000);

        // Heap-based top-K selection
        let (retained, total) = ProcessService::select_top_k_candidates(input, 2000).unwrap();
        assert_eq!(total, 10000);
        assert_eq!(retained.len(), 2000);

        // Both must match exactly item for item
        for (i, (h, r)) in retained.iter().zip(reference.iter()).enumerate() {
            assert_eq!(h.name, r.name, "Mismatch at index {}", i);
            assert_eq!(h.pid, r.pid, "PID mismatch at index {}", i);
        }

        // Distinct PIDs
        let pids: HashSet<u32> = retained.iter().map(|c| c.pid).collect();
        assert_eq!(pids.len(), 2000);
    }
}
