# DeskPilot (Not Done Yet...)

<div align="center">

![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011%20(x64)-0078D6?style=for-the-badge&logo=windows&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-v2.0-FFC131?style=for-the-badge&logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-1.80%2B-DEA584?style=for-the-badge&logo=rust&logoColor=white)
![React](https://img.shields.io/badge/React-19.0-61DAFB?style=for-the-badge&logo=react&logoColor=black)
![TypeScript](https://img.shields.io/badge/TypeScript-5.6-3178C6?style=for-the-badge&logo=typescript&logoColor=white)
![Frontend Tests](https://img.shields.io/badge/Frontend%20Tests-113%20Passing-brightgreen?style=for-the-badge&logo=vitest&logoColor=white)
![Test Definitions](https://img.shields.io/badge/Test%20Definitions-293%20Automated-blue?style=for-the-badge)
![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)

<p align="center">
  <strong>A high-performance, local-first Windows command center built for privacy, determinism, and speed.</strong>
</p>

</div>

---

## Overview

**DeskPilot** is an open-source, technically disciplined command center and launcher engineered natively for Windows. It provides lightning-fast application indexing, sub-millisecond search, safe Win32 execution, and system monitoring with no implemented outbound network client, analytics, telemetry, or cloud dependencies (see [PRIVACY.md](PRIVACY.md)).

Built on **Tauri 2**, **Rust**, and **React 19**, DeskPilot pairs native Win32 performance with modern UI ergonomics while enforcing defense-in-depth isolation boundaries between the web presentation layer and host operating system privileges.

---

## Preview

### Application Discovery Catalog
Discovers applications across Windows Registry hives and Start Menu hierarchies with provenance tracking, operational availability badges, and secure launch controls.

![DeskPilot Application Catalog](docs/screenshots/deskpilot_overview.png)

### Real-Time Deterministic Search & Ranking
Sub-millisecond filtering utilizing a four-tier ranking hierarchy (exact match, prefix match, substring match, and publisher metadata) with keyboard-first navigation.

![DeskPilot Real-Time Search](docs/screenshots/deskpilot_search.png)

---

## Core Capabilities

### 🔍 Deterministic Windows Application Discovery
- **Multi-Hive Registry Enumeration:** Exhaustively indexes 64-bit (`KEY_WOW64_64KEY`) and 32-bit (`KEY_WOW64_32KEY`) system software keys across `HKEY_LOCAL_MACHINE` and `HKEY_CURRENT_USER`.
- **Start Menu Hierarchy Scanner:** Traversing both Common (`C:\ProgramData\Microsoft\Windows\Start Menu\Programs`) and Per-User Start Menu directories.
- **128-bit Collision-Resistant Application Identity:** Deterministic SHA-256 identity generation (`app_<32-hex>`) incorporating normalized publishers, application titles, and target paths to eliminate duplicate merge collisions.
- **Availability State Engine:** Continuously tracks target executables and categorizes entries into `Available`, `Unverified`, `IncompleteMetadata`, and `MissingTarget`.

### 🛡️ Memory-Safe MS-SHLLINK Shortcut Parser
- Handcrafted, memory-safe binary parser complying with Microsoft Shell Link specification (`[MS-SHLLINK]`).
- Strict Shell Link Header CLSID verification (`00021401-0000-0000-C000-000000000046`).
- Checked offset arithmetic and bounded file reads (64 KB ceiling) protecting against malicious link-bomb attacks.
- Robust decoding supporting both legacy Windows-1252 ANSI strings and modern UTF-16LE Unicode paths.

### ⚡ Secure Win32 Native Launcher
- **Direct Win32 API Invocation:** Spawns processes directly via Windows `CreateProcessW` without intermediate shell interpreters (`cmd.exe` or `powershell.exe`), eliminating command injection vulnerabilities.
- **Safe Working Directory Binding:** Resolves working directories to target executable parent directories or system roots, preventing DLL hijacking in shared folders.
- **Process Isolation:** Detaches launched applications into independent process trees without inheriting privileged handles.

### 📊 Native Win32 System Intelligence Monitoring
- **Hardware & CPU Utilization:** Periodic non-blocking retrieval of total processor utilization across system cores via Windows `GetSystemTimes`.
- **Physical Memory Accounting:** Queries system RAM configuration through `GlobalMemoryStatusEx`, presenting total physical capacity, available RAM, and real-time memory pressure.
- **Fixed Storage Partition Telemetry:** Enumerates mounted fixed storage drives using `GetLogicalDriveStringsW` and `GetDiskFreeSpaceExW`, calculating total volume size, free capacity, and percentage used for local partitions while excluding volatile optical or remote network media.
- **Monotonic System & App Uptime:** Tracks host system uptime via `GetTickCount64` and measures continuous DeskPilot application session duration.

### 🔌 Strongly Typed, Least-Privilege IPC Boundary
- **Opaque ID Addressing:** The frontend presentation layer only holds opaque application IDs (`app_<hex>`) and never handles raw filesystem paths, executable arguments, or environment variables.
- **Input Guard Validation:** Rigorous backend validation rejecting malformed identifiers, invalid UTF-8 sequences, and forbidden payload structures.
- **Safe Serialization:** Structured error envelopes ensuring system path leaks do not reach frontend error toasts.

### 🪟 Desktop Lifecycle & System Tray Integration
- **Single-Instance Enforcement:** Automatically detects secondary launches, forwards focus signals, and summons the primary instance without spawning duplicate processes.
- **Native System Tray:** Lives quietly in the Windows notification area with a native context menu (`Show DeskPilot`, `Hide DeskPilot`, `Quit DeskPilot`).
- **Predictable Hide-to-Tray:** Window close events (`X`) gracefully hide the window to the tray while preserving warm state and catalog indices; explicit quit terminates cleanly.
- **Ergonomic Window Constraints:** Configured with a default dimension of 960×680 and strict minimum bounds of 520×450 to guarantee consistent UI layout.

---

## Architecture

```text
┌────────────────────────────────────────────────────────┐
│               Frontend Presentation Layer              │
│               React 19 • TypeScript • Vite             │
│   • ApplicationCatalog        • Search & 4-Tier Rank   │
│   • ApplicationCard           • Keyboard Navigation    │
│   • LaunchButton (Feedback)   • Real-Time Status Badges│
└───────────────────────────┬────────────────────────────┘
                            │
               Typed Tauri 2 IPC Boundary
         (Domain Intents & Opaque Application IDs)
                            │
┌───────────────────────────▼────────────────────────────┐
│                Rust Backend Core (Kernel)              │
│                                                        │
│  ┌───────────────────────┐   ┌──────────────────────┐  │
│  │   Security Kernel     │   │  Application Catalog │  │
│  │ • Input Guard         │   │ • Generational Cache │  │
│  │ • Path Bounds Checker │   │ • RwLock Concurrency │  │
│  └───────────────────────┘   └──────────────────────┘  │
│  ┌───────────────────────┐   ┌──────────────────────┐  │
│  │   Discovery Engine    │   │   System Tray & Win  │  │
│  │ • Registry Scanners   │   │ • Single-Instance    │  │
│  │ • MS-SHLLINK Parser   │   │ • Intercepted Close  │  │
│  └───────────────────────┘   └──────────────────────┘  │
└───────────────────────────┬────────────────────────────┘
                            │ Direct Win32 APIs
┌───────────────────────────▼────────────────────────────┐
│                 Windows 10 / 11 Native                 │
│  • Advapi32 (Registry)       • Shell32 (Shortcuts)     │
│  • Kernel32 (CreateProcessW) • User32 (Tray & Window)  │
└────────────────────────────────────────────────────────┘
```

---

## Security Model & Invariants

DeskPilot is designed around strict defense-in-depth principles:

1. **Least-Privilege Web View:** WebView2 operates with restricted permissions. Direct filesystem APIs, shell execution, and Node.js primitives are completely disabled.
2. **Path Traversal Guard:** Any filesystem access strictly sanitizes relative segments (`..`, `.\`, `%TEMP%`) and normalizes paths before verification.
3. **No Shell Intermediate:** Launch commands are never formatted as string shell scripts. Process creation bypasses shell parsing entirely.
4. **Local Data Sovereignty:** The audited application code contains no implemented outbound HTTP or WebSocket client, analytics, telemetry, or automatic update mechanism. DeskPilot's core functionality does not require an active internet connection after installation. (Note: Windows, WebView2, the installer, and applications launched by the user may independently perform network activity; see [`PRIVACY.md`](PRIVACY.md)).

---

## Development & Build

### Prerequisites

| Component | Minimum Version | Recommended |
|---|---|---|
| **OS** | Windows 10 (64-bit) | Windows 11 Build 22631+ |
| **Node.js** | Node.js v20+ | Node.js v22+ LTS |
| **Rust** | Rust 1.80+ | Rust MSVC Toolchain (`x86_64-pc-windows-msvc`) |
| **C++ Build Tools** | Visual Studio Build Tools 2022 | MSVC v143 / Windows 10/11 SDK |
| **WebView2** | Evergreen Runtime | Pre-installed on Windows 10/11 |

### Setup & Commands

All commands can be run via PowerShell or Command Prompt. Use `npm.cmd` on Windows if PowerShell execution policies restrict script execution:

```powershell
# Clone the repository
git clone https://github.com/youssefmanssouri/deskpilot-.git
cd deskpilot-

# Install frontend dependencies
npm.cmd install

# Type-check TypeScript and build the production web bundle
npm.cmd run build

# Run frontend test suite (113 tests across 11 suites)
npm.cmd test -- --run

# Launch full desktop application in development mode (with hot reload)
npm.cmd run tauri dev
```

---

## Automated Verification Suite

DeskPilot maintains a total of **293 automated test definitions identified** across both frontend presentation and native backend layers:

- **Frontend Execution Evidence:** **113 tests passed** at the audited baseline (`npm test -- --run`), executed across 11 Vitest suites.
- **Rust Backend Structural Inventory:** **180 test definitions identified** in source (143 unit test definitions in `src/`, 37 integration test definitions in `tests/`). Execution is not verified at the current host baseline due to Windows Smart App Control (SAC) blocking unsigned debug build-scripts.

### Frontend Test Breakdown (Vitest — 113 Passing)
- `ProcessMonitor.test.tsx` (16 tests): Process list rendering, state sorting, CPU/memory formatting, interactive termination confirmations, and error envelopes.
- `ApplicationCatalog.test.tsx` (15 tests): Discovery loading, error states, keyboard navigation, empty state handling, and refresh lifecycles.
- `SystemMonitor.test.tsx` (13 tests): CPU gauge rendering, memory pressure indicators, storage partition cards, and uptime formatting.
- `interaction.test.tsx` (22 tests): Component interaction, keyboard-first arrow navigation, quick-select, focus traps, and modal controls.
- `LaunchButton.test.tsx` (12 tests): Launch invocation, pending state, success confirmation reset, error recovery, and timer unmount hygiene.
- `discovery.test.ts` (8 tests): Typed IPC contract validation for application discovery and payload integrity.
- `processes.test.ts` (7 tests): Process enumeration, kill command IPC contract, and error deserialization.
- `launcher.test.ts` (6 tests): Launch IPC contract validation and error mapping.
- `ApplicationCard.test.tsx` (6 tests): Card rendering, availability badge states, publisher provenance display.
- `system.test.ts` (4 tests): Typed IPC contracts for system times, memory metrics, and drive partitions.
- `ipc.test.ts` (4 tests): Typed error serialization and boundary protection.

### Backend Test Inventory (Cargo / Rust — 180 Definitions Identified)
- **Unit Test Definitions (`src-tauri/src/`):** **143 unit test definitions** identified across 19 source modules covering application discovery and registry scanning, application identity hashing, safe process launching and path traversal checks, process service and provider integration, system intelligence metrics (CPU, RAM, fixed disks, uptime), in-memory `AppState` concurrency, system tray lifecycle, error sanitization, and input guard validation.
- **Integration Test Definitions (`src-tauri/tests/`):** **37 integration test definitions** identified across 3 integration suites:
  - `catalog_launcher_integration_tests.rs` (13 test definitions): End-to-end catalog indexing, state transition, and launch validation.
  - `discovery_unit_tests.rs` (12 test definitions): Multi-source discovery merging, shortcut inspection, and uninstaller filtering.
  - `process_integration_tests.rs` (12 test definitions): Process snapshot queries, system provider metrics, and termination contracts.
- **Execution Note:** Execution of the backend test definitions is not verified at the current host baseline due to Windows Smart App Control (SAC) blocking unsigned debug build-scripts.

---

## Project Roadmap

- [x] **Phase 1:** Architectural Foundation & Security Baseline
- [x] **Phase 2:** Hardened Discovery Engine & Registry Scanner
- [x] **Phase 3:** MS-SHLLINK Binary Parser & Application Identity
- [x] **Phase 4:** Win32 Secure Launcher, Typed IPC & Catalog Integration
- [x] **Phase 5.1:** Window Lifecycle Hardening, Single-Instance & 4-Tier Search Ranking
- [x] **Phase 5.2:** Native System Tray & Summon Integration
- [ ] **Phase 6:** Safe Local File Operations (Recycle Bin Integration & Rollback Ledgers)
- [ ] **Phase 7:** SQLite Persistence & Operation Audit Trail
- [ ] **Phase 8:** System Resource Monitoring & Health Metrics

---

## Documentation & Privacy

- **Privacy Architecture:** Technical details regarding local data sovereignty, in-memory state, and host platform boundaries are documented in [`PRIVACY.md`](PRIVACY.md).

---

## License

This project is licensed under the [MIT License](LICENSE).
