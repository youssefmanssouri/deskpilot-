# Security Policy

DeskPilot is designed from the ground up as a security-conscious desktop command center. Because the application interacts with operating system facilities, file manipulation, and process launching, security boundaries are enforced as architectural guarantees rather than developer conventions.

---

## Core Security Invariant

> **Frontend requests are never treated as authorization to perform arbitrary operating-system actions.**

The user interface layer is an untrusted presentation layer. Every request crossing the Tauri IPC boundary must represent a discrete, validated high-level domain intent. The backend Rust kernel alone owns authorization, path canonicalization, pre-flight safety checks, and execution boundaries.

---

## Core Security Principles

### 1. Principle of Least Privilege
The frontend user interface (React running inside Windows WebView2) is treated as an untrusted or semi-trusted environment. It is never granted wildcard or blanket OS privileges. Generic Tauri filesystem, process, and shell access plugins are disabled by default.

### 2. Zero Shell Invocation
DeskPilot never invokes command interpreters (`cmd.exe /c`, `powershell.exe -Command`, or bash) to launch applications or execute system actions. All process executions are dispatched directly through native Win32 APIs (`CreateProcessW` or `ShellExecuteExW`) with discrete argument arrays to prevent shell injection, argument injection, and command chaining vulnerabilities.

### 3. Strongly Typed Domain IPC Boundaries
Communication between the frontend and Rust core is strictly mediated by strongly-typed Tauri IPC commands. The frontend transmits high-level *domain intents* (e.g., `execute_application(app_id: Uuid)`), never raw executable paths or arbitrary system commands. Every payload is validated against strict Serde schemas in Rust and Zod/TypeScript schemas in the frontend before execution.

### 4. Path Traversal & Filesystem Guards
All filesystem paths received by the backend must undergo canonicalization (`std::fs::canonicalize`) and pass through a dedicated security guard (`security::InputGuard` / `fs_guard`) to prevent directory traversal (`..`), Alternate Data Streams (`:stream`), null bytes (`\0`), and unauthorized namespace access.

### 5. Safe Destructive Operations & Rollback Safeguards
Destructive file operations (such as file deletion or bulk renaming) require explicit safety mechanisms:
- File deletions default to the Windows Recycle Bin (`IFileOperation` / `SHFileOperationW` with `FOF_ALLOWUNDO`) rather than permanent disk unlinking.
- Mutating operations require two-phase verification: dry-run inspection followed by transactional execution with a logged rollback journal in SQLite.

### 6. Local-First & Zero Telemetry
DeskPilot operates entirely locally. No analytics, tracking tokens, crash dumps, or user telemetry are transmitted over the network. All operational state resides in local SQLite storage restricted to the active user's local profile (`%LOCALAPPDATA%`).

### 7. Dependency & Supply Chain Integrity
All npm and Cargo dependencies are audited regularly. Lockfiles (`package-lock.json` and `Cargo.lock`) are committed to version control to guarantee deterministic, reproducible builds.

---

## Reporting a Vulnerability

If you discover a security vulnerability in DeskPilot, please report it responsibly:

1. **Do not disclose the issue publicly** via GitHub Issues, discussions, or social media.
2. Email a detailed description, reproduction steps, and proof-of-concept to the maintainers or submit a private security advisory on GitHub.
3. Maintainers will review the report within 48 hours, coordinate a remediation patch, and credit the reporter in the advisory release notes.
