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

### 5. Bounded State Mutation & Isolation Safeguards
DeskPilot avoids unconstrained filesystem modifications. The v0.1.1 preview application does not perform destructive file manipulations (such as arbitrary file deletion, renaming, or task killing). Persistent application data is stored in discrete, schema-bounded local JSON files (`settings.json` and `launch-history.json`) under the user's application profile (`%APPDATA%\com.deskpilot.app\`), with defensive write routines that prevent corruption. In addition, an optional per-user startup registry value at `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` is created only when the user explicitly enables startup behavior (it does not exist by default).

### 6. Local-First Architecture & Network Behavior Disclosures
DeskPilot is architected as an offline, local-first utility. Persistent application data is confined to local schema-bounded JSON files (`settings.json` and `launch-history.json`) in the active user's profile directory (`%APPDATA%\com.deskpilot.app\`) and, only if explicitly enabled by the user, the optional per-user startup registry value at `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. Network disclosures are categorized as follows:

- **Source-Code Inspection:** Static inspection of DeskPilot's application codebase (Rust core and React web assets) identified no application-level telemetry engines, analytics trackers, crash reporters, HTTP/WebSocket clients, or socket implementations in the inspected application code. This is a source-inspection finding, not proof of runtime behavior or a universal guarantee across all execution conditions or platform dependencies.
- **Empirical Runtime Observations:** During documented physical Windows 11 testing workloads and soak monitoring (Phase 16.38), no active TCP/UDP listening sockets or remote socket connections were observed from the DeskPilot desktop process. These findings represent empirical observations from specific tested workloads and configurations, not an exhaustive guarantee across all execution conditions, Windows environments, or network configurations.
- **Independent Platform Components:** Certain system components operate independently outside DeskPilot's own application code:
  - The Microsoft Edge WebView2 runtime is an independently managed platform component under Windows and Microsoft Edge. It may initiate its own network connections under Microsoft's policies, and stores profile data locally in `%LOCALAPPDATA%\com.deskpilot.app\EBWebView`.
  - The MSI installer embeds Microsoft's WebView2 bootstrapper and, per the installer configuration, invokes it only when an existing WebView2 runtime installation is not detected; runtime installation and any associated network activity are managed by that bootstrapper.
  - Third-party applications launched through DeskPilot run as independent operating-system processes with their own permissions and network behavior.

### 7. Dependency & Supply Chain Integrity
All npm and Cargo dependencies are audited regularly. Lockfiles (`package-lock.json` and `Cargo.lock`) are committed to version control to guarantee deterministic, reproducible builds.

### 8. Application Discovery & Metadata Trust Boundaries
Application discovery interacts with external Windows state (registry hives and Start Menu filesystem hierarchies). All discovered data is treated as untrusted:
- **Zero Execution Policy:** Discovered binaries, registry strings (`UninstallString`, `DisplayIcon`), and `.lnk` shortcuts are NEVER executed or passed to the shell during discovery.
- **Safe Binary Parsing:** Shell link files (`.lnk`) are parsed using an isolated binary parser that reads raw LinkInfo headers without instantiating COM or triggering Windows shell handlers.
- **Path and Machine Data Omission:** Internal executable paths, install directories, and user profile paths are retained only within the privileged Rust domain; they are never transmitted to the frontend in `ApplicationDto`.
- **Resilience Against Malformed Data:** Registry entries with corrupt types, missing fields, or oversized buffers are handled gracefully without aborting the discovery scan.
- **Partial Failure Transparency:** Inaccessible registry subkeys or filesystem folders are reported as non-fatal warnings in `partialFailures`, preventing silent failure states.

---

## Reporting a Vulnerability

If you discover a security vulnerability in DeskPilot, please report it responsibly:

1. **Do not disclose the issue publicly** via GitHub Issues, discussions, or social media.
2. Email a detailed description, reproduction steps, and proof-of-concept to the maintainers or submit a private security advisory on GitHub.
3. Maintainers will review the report within 48 hours, coordinate a remediation patch, and credit the reporter in the advisory release notes.
