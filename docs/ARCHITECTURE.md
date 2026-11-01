# DeskPilot Architecture

This document defines the high-level system boundaries, execution layers, security invariants, and error propagation model of DeskPilot.

---

## 1. System Boundary Architecture

```text
┌─────────────────────────────────────────────────────────────────────────┐
│              UNTRUSTED / SEMI-TRUSTED PRESENTATION LAYER                │
│                                                                         │
│  React 19 + TypeScript (Vite / WebView2)                                │
│  - Presentation UI & Application Catalog View                           │
│  - Typed IPC API Client (`src/lib/ipc/api.ts`)                          │
│  - Structured Error Handling (`IpcError`, `ErrorCode`)                  │
│  - ZERO direct filesystem, process, or registry access                  │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                     Strongly-Typed Tauri 2 IPC
                     `invokeSafe<T>(command, payload)`
                     Domain Intents (No raw paths / shell strings)
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                   PRIVILEGED DESKPILOT RUST KERNEL                      │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 1. Commands Layer (`src-tauri/src/commands/`)                     │  │
│  │    Thin controllers, DTO mapping, IPC error boundaries            │  │
│  └─────────────────────────────────┬─────────────────────────────────┘  │
│                                    │                                    │
│                                    ▼                                    │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 2. Security Layer (`src-tauri/src/security/`)                     │  │
│  │    InputGuard, whitelist checks, path traversal guards            │  │
│  └─────────────────────────────────┬─────────────────────────────────┘  │
│                                    │                                    │
│                                    ▼                                    │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 3. Domain Services (`src-tauri/src/core/`)                        │  │
│  │    ApplicationDiscoveryService, ApplicationIdentity               │  │
│  │    StatusService, pure business logic                             │  │
│  └─────────────────────────────────┬─────────────────────────────────┘  │
│                                    │                                    │
│                                    ▼                                    │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ 4. Platform Engine (`src-tauri/src/platform/windows/`)            │  │
│  │    Registry scanner (HKLM 64/32, HKCU), safe MS-SHLLINK parser    │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
             Windows Native Subsystem / Win32 APIs / Local State
             - Win32 CreateProcessW (Direct execution without shell)
             - IFileOperation / SHFileOperation (Shell Recycle Bin)
             - Winreg registry reading (Read-only, no execution)
             - Future SQLite Database (WAL mode, restricted user ACLs)
```

---

## 2. Layer Responsibilities & Trust Invariants

### 2.1 Frontend Presentation Layer (`src/`)
- **Trust Level:** Untrusted to Semi-Trusted.
- **Contract:** React components must never import or call Tauri's raw `invoke()` directly. All interactions are routed through `src/lib/ipc/api.ts`.
- **Domain Intents Only:** The frontend issues intents (e.g. `getAppStatus()`, `discoverApplications()`), never operating-system primitives like file paths or shell commands.

### 2.2 Tauri IPC Boundary (`invokeSafe`)
- Mediated by `src/lib/ipc/client.ts`.
- Automatically catches IPC rejections and converts them into strongly-typed `IpcError` instances with machine-readable error codes (`INVALID_INPUT`, `PERMISSION_DENIED`, `NOT_FOUND`, etc.).

### 2.3 Rust Command Layer (`src-tauri/src/commands/`)
- Thin controllers with minimal logic.
- Sole responsibilities:
  1. Deserialize and accept input DTOs.
  2. Invoke domain services in `crate::core`.
  3. Map `DomainError` into safe `IpcErrorDto` responses.

### 2.4 Security Layer (`src-tauri/src/security/`)
- Implements `InputGuard`:
  - Enforces length bounds on identifiers to prevent memory exhaustion.
  - Rejects null bytes (`\0`), directory traversal sequences (`..`), separators (`/`, `\`), and shell command chaining symbols.
  - Verifies requested action types against a strict whitelist.

### 2.5 Domain Services (`src-tauri/src/core/`)
- Pure business logic decoupled from Tauri transport.
- Functions operate on domain models and return `Result<T, DomainError>`.

### 2.6 State Management (`src-tauri/src/state/`)
- Encapsulates `AppState` registered with Tauri runtime via `.manage()`.
- Thread-safe tracking of application metadata, uptime (`Instant`), and health flags.

### 2.7 Error Boundary (`src-tauri/src/error.rs`)
- Structured `DomainError` enum:
  - `InvalidInput`
  - `NotFound`
  - `PermissionDenied`
  - `Unavailable`
  - `OperationFailed`
  - `InternalError`
- **Sanitization Guarantee:** Internal details, absolute file paths, panic strings, and OS error codes are never exposed to the frontend. `InternalError` maps to a safe generic advisory: *"An internal system error occurred. Please consult backend logs."*

---

## 3. Technology Boundary Decisions

### Python Layer: Deferred
Python is excluded from the core runtime to ensure a lightweight, secure single-binary distribution. All application indexing, persistence, and filesystem operations reside entirely in Rust. If advanced scripting or user-defined plugins are needed later, Python will only be evaluated as an isolated, optional, sandboxed sidecar.

### AI Capabilities: Deferred
Deterministic, offline core functionality takes precedence over artificial intelligence. All search, command filtering, and system automation must function predictably and with sub-millisecond response times offline before any AI features are explored.

---

## 4. Windows Application Discovery Architecture

### 4.1 Discovery Sources
DeskPilot inspects controlled, documented Windows discovery locations without recursively scanning arbitrary filesystem roots:
1. **Registry HKLM (64-bit):** `Software\Microsoft\Windows\CurrentVersion\Uninstall` (Machine-wide 64-bit applications).
2. **Registry HKLM (32-bit):** `Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall` (Machine-wide 32-bit applications on 64-bit Windows).
3. **Registry HKCU (User):** `Software\Microsoft\Windows\CurrentVersion\Uninstall` (Per-user installations, e.g. VS Code User, Discord).
4. **Common Start Menu:** `%ProgramData%\Microsoft\Windows\Start Menu\Programs` (System-wide `.lnk` shortcuts).
5. **User Start Menu:** `%APPDATA%\Microsoft\Windows\Start Menu\Programs` (User-specific `.lnk` shortcuts).

### 4.2 Deterministic Application Identity Algorithm
DeskPilot does not rely on transient UUIDs or bare display names for identity. Application IDs are generated using a deterministic, collision-resistant algorithm:

$$\text{seed} = \begin{cases} 
\text{"pub:"} + \text{norm}(publisher) + \text{":app:"} + \text{norm}(name) & \text{if publisher exists} \\
\text{"exe:"} + \text{basename}(target) + \text{":app:"} + \text{norm}(name) & \text{if executable basename exists} \\
\text{"bare:app:"} + \text{norm}(name) & \text{otherwise}
\end{cases}$$

$$\text{ID} = \text{"app\_"} + \text{Hex}(\text{SHA-256}(\text{seed}))[0..16]$$

- **Normalization Rules:**
  - `norm(name)`: Strips leading/trailing whitespace, converts to lowercase, collapses consecutive spaces.
  - `norm(publisher)`: Strips corporate legal suffixes (`, inc.`, `, llc.`, ` corp.`, ` corporation`, ` gmbh`, ` ltd.`) so cross-source variants resolve identically.
  - `basename(target)`: Extracts lowercase executable filename (e.g. `chrome.exe`).
- **Guarantees:**
  - Stable across runs for identical applications.
  - Differentiates different applications that happen to share identical display names (e.g. "Launcher" from different publishers).
  - Unifies observations of the same application discovered across both Registry and Start Menu.

### 4.3 Deduplication & Conflict Resolution
When an application is discovered from multiple sources:
- **Source Provenance:** Every contributing source is accumulated in the application's `sources` list (e.g. `["Registry (HKLM 32-bit)", "Start Menu (Common)"]`).
- **Display Name Precedence:** Registry `DisplayName` is preferred over shortcut filename for accurate casing and branding.
- **Publisher & Version Precedence:** Non-empty Registry values take precedence over unassigned shortcut metadata.
- **Availability Precedence:** Evaluated using deterministic precedence:
  $$\text{Available} \succ \text{Unverified} \succ \text{IncompleteMetadata} \succ \text{MissingTarget}$$

### 4.4 Availability States
- **`Available`:** Target executable is verified to exist on disk.
- **`MissingTarget`:** An explicit target path was specified, but is absent from disk.
- **`Unverified`:** An installation folder exists or shortcut was found, but target binary was not resolved directly.
- **`IncompleteMetadata`:** Entry has no executable or install directory attached.

### 4.5 Metadata Trust Boundaries
- **Untrusted Inputs:** All registry values and shortcut files are treated as untrusted data. Corrupted registry types or malformed link files do not crash the service.
- **Zero Execution:** No shortcuts, executables, or registry strings (`UninstallString`, `DisplayIcon`) are executed during discovery.
- **Path Redaction:** Raw filesystem paths, user profile folders, and registry keys are retained strictly within the privileged Rust domain; only sanitized DTOs are sent to React.
- **Partial Failure Transparency:** If an individual hive or directory is inaccessible (e.g. permission restriction), it is recorded in `partialFailures` rather than failing open or silently swallowing errors.

### 4.6 Known Limitations & Next Steps
- Packaged MSIX / modern UWP apps from the Microsoft Store are not enumerated in this phase (desktop Win32 installed apps and Start Menu shortcuts are supported).
- Application launching and persistence in SQLite are deferred to subsequent phases.
