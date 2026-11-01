# DeskPilot Architecture

This document defines the high-level system boundaries, execution layers, security invariants, and error propagation model of DeskPilot.

---

## 1. System Boundary Architecture

```text
┌─────────────────────────────────────────────────────────────────────────┐
│              UNTRUSTED / SEMI-TRUSTED PRESENTATION LAYER                │
│                                                                         │
│  React 19 + TypeScript (Vite / WebView2)                                │
│  - Presentation UI & Virtualized Lists                                  │
│  - Typed IPC API Client (`src/lib/ipc/api.ts`)                          │
│  - Structured Error Handling (`IpcError`, `ErrorCode`)                  │
│  - ZERO direct filesystem or OS process access                          │
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
│  │    Thin controllers, Serde DTO validation, error mapping          │  │
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
│  │    Pure business logic, StatusService, operations engine          │  │
│  └──────────────┬──────────────────────────────────┬─────────────────┘  │
│                 │                                  │                    │
│                 ▼                                  ▼                    │
│  ┌──────────────────────────────┐  ┌─────────────────────────────────┐  │
│  │ 4. State (`src-tauri/state`) │  │ 5. Error Boundary (`error.rs`)  │  │
│  │    Managed runtime state     │  │    DomainError -> IpcErrorDto   │  │
│  │    Thread-safe AppState      │  │    Sanitized user-facing errors │  │
│  └──────────────────────────────┘  └─────────────────────────────────┘  │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
             Windows Native Subsystem / Win32 APIs / Local SQLite
             - Win32 CreateProcessW (Direct execution without shell)
             - IFileOperation / SHFileOperation (Shell Recycle Bin)
             - Local SQLite Database (WAL mode, restricted user ACLs)
```

---

## 2. Layer Responsibilities & Trust Invariants

### 2.1 Frontend Presentation Layer (`src/`)
- **Trust Level:** Untrusted to Semi-Trusted.
- **Contract:** React components must never import or call Tauri's raw `invoke()` directly. All interactions are routed through `src/lib/ipc/api.ts`.
- **Domain Intents Only:** The frontend issues intents (e.g. `getAppStatus()`, `validateCommandIntent(...)`), never operating-system primitives like file paths or shell commands.

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
