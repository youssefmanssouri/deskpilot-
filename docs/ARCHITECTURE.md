# DeskPilot Architecture

This document defines the high-level system boundaries, execution layers, and security invariants of DeskPilot.

---

## 1. System Boundary Diagram

```text
React / TypeScript (Vite + WebView2)
        │
        │ typed Tauri IPC (Domain Intents)
        ▼
Tauri 2 / Rust Core Engine
        │
        ├── domain services (Application Indexer, Search Engine)
        ├── security boundaries (Path Guard, Capability Scopes)
        ├── SQLite (Local Persistence, Rollback Journal)
        └── Windows APIs (Win32 CreateProcessW, Shell Recycle Bin)
```

---

## 2. Layer Responsibilities & Trust Model

### Presentation Layer: React + TypeScript
- **Trust Level:** Untrusted to Semi-Trusted.
- **Responsibilities:** Renders the command palette, virtualized search lists, settings panels, and responsive status indicators.
- **Invariants:**
  - The UI layer is strictly prohibited from holding direct operating system handles, filesystem descriptors, or process execution controls.
  - All communication directed to the operating system must transit through strongly-typed Tauri IPC (`invoke`).
  - Frontend commands must express **domain intents** (e.g., `launch_application(app_id)` or `preview_file_operation(op_id)`) rather than generic operating system primitives (e.g., never `execute_shell_string(cmd)` or `read_arbitrary_path(path)`).

### Privileged Host Layer: Tauri 2 + Rust
- **Trust Level:** High Privilege Security Kernel.
- **Responsibilities:**
  - Validates and sanitizes all incoming IPC request payloads.
  - Interacts directly with Windows native subsystems via Win32 C/C++ bindings (`windows-rs` / `winapi`).
  - Manages the local SQLite database connection pool (WAL mode).
  - Enforces atomic operations, pre-flight checks, and transaction rollback logs.
- **Invariants:**
  - Shell interpreters (`cmd.exe`, `powershell.exe`) are never used to spawn child processes.
  - File deletions are routed through the Windows Shell Recycle Bin (`IFileOperation`) to prevent accidental unrecoverable data destruction.
  - Every path traversal attempt (`..`, relative segments, UNC paths, ADS) is intercepted and rejected before I/O execution.

---

## 3. Technology Boundary Decisions

### Python Layer: Deferred
To maintain a small single-binary distribution footprint (~15-20MB), avoid cross-process IPC synchronization failure modes, and eliminate Python serialization attack surfaces, **Python is excluded from the core architecture**. 
All application indexing, search ranking, persistence, and filesystem operations reside entirely in Rust. If advanced scripting or user-defined plugins are needed in the future, Python may only be evaluated as an isolated, optional, sandboxed sidecar with restricted permissions.

### AI Capabilities: Deferred
Deterministic, rock-solid core functionality takes precedence over artificial intelligence. All search, command filtering, and system automation must function predictably and with sub-millisecond response times offline. AI features will only be explored after the deterministic desktop foundation is completely stabilized and security-audited.
