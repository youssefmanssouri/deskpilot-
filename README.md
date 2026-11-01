# DeskPilot

> A serious, local-first Windows desktop command center.

DeskPilot is designed as an open-source, technically disciplined, and security-conscious command center for Windows. It provides fast keyboard-driven navigation, application indexing, safe local file operations, and productivity utilities without mandatory cloud dependencies or telemetry.

---

## Mission & Philosophy

- **Local-First & Offline:** All persistent data, indexing, and operational state remain on the local machine in SQLite. No cloud account is required.
- **Deterministic Core First:** Essential system operations and utilities are engineered for reliability and determinism before introducing any optional AI features.
- **Zero Telemetry by Default:** DeskPilot does not collect, track, or transmit usage metrics, telemetry, or system information.
- **Strict Security Boundaries:** Destructive actions (such as file modifications and deletions) incorporate pre-flight validation, preview mechanisms, Windows Recycle Bin integration, and rollback ledgers. The frontend interface never receives unfettered shell or filesystem access.

---

## Architecture Overview

DeskPilot enforces strict separation of concerns across a multi-tier boundary:

```text
React 19 / TypeScript (Vite)
        │
        │ Typed Tauri 2 IPC (Domain Intents)
        ▼
Tauri 2 / Rust Core Engine
        │
        ├── Domain Services (Application Indexer, Search Engine)
        ├── Security Kernel (Path Traversal Guard, Least-Privilege Policies)
        ├── SQLite Persistence (Local State & Operation History)
        └── Native Windows Subsystem (Direct Win32 APIs, Recycle Bin)
```

- **Frontend (Presentation):** React + TypeScript running in WebView2. Responsible solely for user interaction and UI state rendering. It has no direct access to the Windows filesystem, shell, or processes.
- **Backend (Privileged Kernel):** Rust inside Tauri 2. Enforces security capabilities, manages local SQLite databases, executes Win32 APIs directly (`CreateProcessW`, `IFileOperation`), and validates all IPC payloads.
- **Python / AI:** Deferred. Python is excluded from the core architecture to maintain a lightweight, single-binary distribution and eliminate IPC serialization attack surfaces. Any future scripting or AI capabilities will operate as isolated optional extensions.

---

## Current Status

**Phase 1: Environment & Toolchain Foundation (In Progress)**
- Official MSVC Rust toolchain (`rustc`/`cargo` v1.99.1) configured with LLVM linker (`rust-lld`) and Windows SDK / CRT libraries.
- Tauri 2 + React + TypeScript scaffolding established.
- Comprehensive `.gitignore` and security baseline implemented.
- Quality gate checks for frontend (`tsc && vite build`) and baseline tests established.

*Application features (app launcher, file manager, notes, system metrics) will be implemented in subsequent phases.*

---

## Development Prerequisites

- **Operating System:** Windows 10/11 (64-bit)
- **Node.js:** Node.js v20+ (tested on Node v24.15.0 with npm 11.12.1)
- **Rust Toolchain:** Stable MSVC (`x86_64-pc-windows-msvc`), version 1.80+ (v1.99.1 recommended)
- **Windows Build Tools:** MSVC CRT & Windows SDK libraries (managed via `~/.xwin` or Visual Studio C++ Build Tools)
- **WebView2:** Microsoft Edge WebView2 Runtime (pre-installed on Windows 10/11)

---

## Basic Development Commands

All npm commands should be invoked using `npm.cmd` on Windows if PowerShell execution policies restrict `.ps1` scripts:

```powershell
# Install frontend dependencies
npm.cmd install

# Type-check TypeScript and build frontend bundle
npm.cmd run build

# Start the Vite development server (UI only)
npm.cmd run dev

# Run Rust tests in the backend
cd src-tauri
cargo test
cd ..

# Launch the full Tauri 2 desktop application in development mode
npm.cmd run tauri dev
```

---

## License

This project is open-source software licensed under the [MIT License](LICENSE).
