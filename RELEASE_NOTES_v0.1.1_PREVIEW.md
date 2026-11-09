# DeskPilot v0.1.1 — Developer Preview Release Notes

**Release Version:** v0.1.1 (Developer Preview)  
**Release Date:** November 2026  
**License:** [MIT License](LICENSE) (100% Free and Open Source Software)  
**Distribution Channel:** Controlled Preview / Developer Testing (Not General Consumer Availability)

---

## 1. Overview & Preview Scope

DeskPilot v0.1.1 is an open-source, local-first Windows command center engineered natively in **Rust**, **Tauri 2**, and **React 19**. It provides local application discovery, keyboard-first search, direct Win32 process execution, read-only process monitoring, and real-time system intelligence telemetry.

> [!IMPORTANT]
> **Controlled Preview Status:**  
> This release is a **controlled developer and early-adopter preview**, intended for technical evaluation and feedback. It is **not an unrestricted general consumer release**. Binaries are currently unsigned, and runtime verification has been performed exclusively on Windows 11. Please review the [Windows Security & Code Signing](#6-windows-security-code-signing--smartscreen-disclosures) and [Known Limitations](#7-known-limitations--operational-advisories) sections before installation.

DeskPilot is and will remain **100% free and open-source software**. There are no paid tiers, no commercial subscriptions, no feature gates, and no account requirements.

---

## 2. Core Capabilities

DeskPilot provides five core native capabilities built on least-privilege architectural principles:

### 🔍 Deterministic Application Discovery & Indexing
- **Multi-Hive Registry Scanning:** Exhaustively enumerates 64-bit (`KEY_WOW64_64KEY`) and 32-bit (`KEY_WOW64_32KEY`) uninstall keys across both `HKEY_LOCAL_MACHINE` and `HKEY_CURRENT_USER`.
- **Start Menu Traversal:** Scans both Common (`C:\ProgramData\Microsoft\Windows\Start Menu\Programs`) and Per-User Start Menu directories.
- **Memory-Safe Shortcut Parsing:** Handcrafted binary parser complying with the Microsoft Shell Link (`[MS-SHLLINK]`) specification with bounded file reads (64 KB ceiling) and CLSID validation.
- **128-bit Collision-Resistant Application Identity:** Generates deterministic `app_<32-hex>` identifiers from normalized metadata to prevent duplicate merge collisions.
- **Availability State Tracking:** Continuously categorizes entries as `Available`, `Unverified`, `IncompleteMetadata`, or `MissingTarget`.

### ⚡ Keyboard-First Real-Time Search & Ranking
- Four-tier ranking hierarchy prioritizing exact title matches, prefix matches, substring matches, and publisher metadata.
- Interactive keyboard navigation (arrow keys, Enter to launch, Escape to clear search query).
- Empty-state rendering when queries yield no matching applications.

### 🛡️ Win32 Native Application Launching
- Direct process spawning via Windows `CreateProcessW` without intermediate shell interpreters (`cmd.exe` or `powershell.exe`), eliminating shell command injection risks.
- Resolves working directories to target executable parent directories or system roots, scoping execution context and avoiding reliance on ambient working directories.
- Launches processes detached from parent handle inheritance.

### 🧵 Read-Only Process Intelligence
- Non-blocking active process enumeration via Win32 Toolhelp32 snapshots (`CreateToolhelp32Snapshot`) offloaded to background threads.
- Formats process identifiers (PID, parent PID), executable names, working-set memory footprint, and CPU execution times.
- Read-only safety: process termination, suspension, and modification are deliberately unimplemented.
- Handles elevated/system processes gracefully via status classification (`full`, `partial`, `handleUnavailable`, `exited`).

### 📊 Real-Time System Intelligence Telemetry
- Non-blocking CPU utilization tracking across all processor cores via Win32 `GetSystemTimes`.
- Physical RAM capacity and memory pressure reporting via `GlobalMemoryStatusEx`.
- Fixed storage volume telemetry via `GetLogicalDriveStringsW` and `GetDiskFreeSpaceExW`, excluding volatile or remote network shares.
- Monotonic system uptime tracking via `GetTickCount64` and application session duration measurement.

### 🪟 Desktop Lifecycle & Notification Area Integration
- Named single-instance mutex enforcement: secondary launches forward focus to the active window without duplicating processes.
- Native Windows system tray icon with context menu (`Show DeskPilot`, `Hide DeskPilot`, `Quit DeskPilot`).
- Predictable hide-to-tray lifecycle: closing the window (`X`) hides it to the notification area while preserving state.

---

## 3. Platform Compatibility Status

| Operating System | Architecture | Verification Status | Operational Notes |
| :--- | :---: | :---: | :--- |
| **Windows 11 (22H2 / 23H2 / 24H2)** | x64 | **Runtime-Tested** | Supported scope for this preview. Validated in clean Windows 11 Enterprise (Build 26100) and Pro (Build 26200) environments. |
| **Windows 10 (21H2 / 22H2)** | x64 | **Architecture Compatible / Runtime Verification Pending** | Win32 APIs and evergreen WebView2 are structurally compatible, but genuine Windows 10 runtime testing is pending. This is not an unconditional blocker for preview users on Windows 11. |
| **Windows on ARM (ARM64)** | ARM64 | **Outside Tested Scope** | Compiles natively for `x86_64-pc-windows-msvc`; native ARM64 binaries are not built, and x64 emulation is unverified. |
| **Legacy Windows (7 / 8 / 8.1)** | Any | **Outside Target Scope** | DeskPilot relies on Windows 10+ Win32 APIs and the Evergreen WebView2 runtime. |

> [!NOTE]
> **Platform Scope for Preview:**  
> The verified platform scope for this developer preview is **Windows 11 (x64)**. While underlying Win32 APIs and WebView2 are standard across Windows 10 and 11, compatibility on Windows 10 has not yet been verified through runtime execution on physical or virtual Windows 10 hardware.

---

## 4. Cryptographic Checksums & Artifact Integrity

Every release binary is identified by its exact SHA-256 cryptographic digest. Verify your download before execution:

| Artifact | File Name | Format | SHA-256 Digest |
| :--- | :--- | :---: | :--- |
| **Windows Installer** | `deskpilot_0.1.1_x64_en-US.msi` | WiX MSI | `d6b919a499b1901bb6e2706b92205ddbba0335db8aaa504c7383c415201764b1` |
| **Standalone Executable** | `deskpilot.exe` | Win32 PE | `8f1beec208cf460bfc74a4b1d907c182ae6b5b5ee114ee722ea9835c93c03493` |

### PowerShell Verification Command
```powershell
Get-FileHash -Algorithm SHA256 .\deskpilot_0.1.1_x64_en-US.msi
# Expected output must match:
# D6B919A499B1901BB6E2706B92205DDBBA0335DB8AAA504C7383C415201764B1
```

---

## 5. Installation, Upgrade & Uninstallation Guide

### Clean Installation
1. Download `deskpilot_0.1.1_x64_en-US.msi`.
2. Run the installer:
   ```cmd
   msiexec /i deskpilot_0.1.1_x64_en-US.msi
   ```
3. The installer installs the application to `C:\Program Files\deskpilot`, places shortcuts in the Start Menu and Desktop, and registers a single entry in Windows Installed Apps (Add/Remove Programs).

### Tested Lifecycle Transitions & Downgrade Distinction
Sandbox testing in Phase 10.9C evaluated specific installer transitions:
- **Verified Successful Operations:** Silent fresh installation (`msiexec /i`), major upgrade from v0.1.0 to v0.1.1 (clean binary replacement, single ARP registration, and user data survival), in-place maintenance repair (`msiexec /fa`), and silent uninstallation (`msiexec /x`).
- **Exposed Known Limitation (Legacy Downgrade):** Running the legacy v0.1.0 installer over v0.1.1 resulted in an unguarded downgrade because v0.1.0 was built without downgrade detection guards (see [Known Limitations](#7-known-limitations--operational-advisories)).

### Major Upgrade from Earlier Versions
- Running `deskpilot_0.1.1_x64_en-US.msi` on a system with DeskPilot v0.1.0 performs a standard Windows Installer major upgrade.
- Verified in testing: existing binaries are cleanly replaced, shortcuts and ARP registrations are maintained without duplicates, and user configuration in `%LOCALAPPDATA%\com.deskpilot.app` is preserved. (Note: Only this specific v0.1.0-to-v0.1.1 path and in-place repair were tested; not every theoretical upgrade permutation has been evaluated).

### In-Place Repair
- If installed files become corrupted, run the repair command:
  ```cmd
  msiexec /fa deskpilot_0.1.1_x64_en-US.msi
  ```

### Uninstallation & Retained Data Notice
- Uninstall via **Windows Settings > Apps > Installed apps**, or via command line:
  ```cmd
  msiexec /x deskpilot_0.1.1_x64_en-US.msi
  ```
- **Application Removal:** The uninstallation test demonstrated clean removal of application binaries (`C:\Program Files\deskpilot`), Start Menu and Desktop shortcuts, and Add/Remove Programs registry entries.
- **User Profile Retention:** The uninstallation test also demonstrated that user profile and runtime data created by the embedded Microsoft WebView2 runtime remained on disk under:
  ```text
  %LOCALAPPDATA%\com.deskpilot.app\EBWebView
  ```
  In empirical sandbox testing, this directory measured 192 files totaling approximately 7.93 MB; these figures represent test-environment measurements and will vary based on user activity. The installer package does not automatically purge this directory during uninstallation; complete removal requires manually deleting this directory.

---

## 6. Windows Security, Code Signing & SmartScreen Disclosures

DeskPilot v0.1.1 release binaries are currently **unsigned** (`Get-AuthenticodeSignature` reports `NotSigned`):

### 1. Unsigned Publisher Status
- An Authenticode code signing certificate is not strictly necessary to publish, download, or evaluate an open-source preview release.
- However, unsigned binaries lack cryptographic publisher attribution and will encounter standard Windows operating system defense prompts and Smart App Control scrutiny.

### 2. Microsoft Defender SmartScreen Reputation & Warnings
- When downloading and running the unsigned MSI or standalone executable, Windows Defender SmartScreen may display an alert:
  *"Windows protected your PC — Microsoft Defender SmartScreen prevented an unrecognized app from starting."*
- SmartScreen assesses applications algorithmically based on download volume, telemetry, and established publisher reputation.
- Even if an Authenticode certificate (standard OV or hardware-token EV) is applied, warning-free execution is not guaranteed immediately. Reputation must accrue over time across multiple downloads before SmartScreen warnings subside.

### 3. Microsoft Smart App Control (SAC) Enforcement
- On Windows 11 systems where **Smart App Control** is enabled in *Enforcement Mode*, execution and enforcement depend on Microsoft's cloud-based security evaluation and policy. Binaries that are unsigned or lack sufficient cloud reputation may be blocked by policy.
- **Smart App Control is distinct from SmartScreen:** SmartScreen presents an informational prompt with an override option, whereas Smart App Control enforces security decisions based on cloud evaluation without an inline user override.
- Code signing with an approved certificate establishes eligibility for SAC evaluation, but signing alone does not guarantee SAC clearance if Microsoft's cloud intelligence has not yet vetted the binary.

### 4. Benefits of Authenticode Signing
- Digital signing provides cryptographic verification that binaries have not been tampered with in transit, confirms publisher identity, and enables reputation accumulation across releases.

> [!CAUTION]
> **Security Advice:**  
> Never disable Microsoft Defender SmartScreen or Smart App Control to run untrusted software. If your organizational or personal security policy prohibits running unsigned software, please defer evaluating DeskPilot until officially signed releases become available.

---

## 7. Known Limitations & Operational Advisories

### 1. Legacy Installer Downgrade Vulnerability
- **Limitation:** DeskPilot v0.1.1 contains internal downgrade protection logic (`WIX_DOWNGRADE_DETECTED` with `LaunchCondition`) that prevents future releases from being downgraded by v0.1.1. However, the earlier legacy v0.1.0 installer was built prior to commit `9169fcf` and lacks downgrade detection guards.
- **Advisory:** **Do not execute the legacy v0.1.0 MSI on a system where v0.1.1 is installed.** Doing so will trigger an unguarded downgrade back to v0.1.0. (Only specific lifecycle transitions were evaluated during testing; do not assume all historical installer combinations are protected).

### 2. Network Isolation Scope
- **Implementation:** DeskPilot application code (Rust backend and React frontend) contains **zero outbound HTTP/WebSocket clients, analytics libraries, telemetry loggers, or background update pollers**.
- **Runtime Environment:** The embedded Microsoft WebView2 control is an independent system component managed by Microsoft. While short-duration testing observed zero active TCP/UDP sockets, this does not establish that background WebView2 runtime tasks (such as component updates or crashpad reporting) will never initiate network calls under all operating system conditions.

### 3. Process Launching Scope
- DeskPilot invokes Windows `CreateProcessW` directly without intermediate shell parsing (`cmd.exe`/`powershell.exe`) and binds working directories to target executable paths or system roots.
- Validates application identifiers and launch targets through backend security guards before invocation.
- This mitigates shell argument injection and ambient working directory issues. However, it does not constitute a generic guarantee against all forms of binary hijacking or operating-system-level vulnerabilities.

### 4. Security & Stability Scope
- **Evaluation Evidence:** The security model has been validated through architectural code review, boundary input fuzzing, and short-duration interactive sessions (4 to 11 minutes of continuous operation in disposable sandbox environments).
- **Proportional Claims:** DeskPilot has **not undergone third-party penetration testing or binary exploit analysis**, and has not been subjected to multi-week stress testing, system sleep/wake recovery cycles, or severe memory pressure validation.

### 5. Smart App Control Impact on Developer Builds
- On hosts with Smart App Control enabled, unsigned Cargo build-scripts (`build.rs`) generated during local `cargo test` execution may be blocked. Automated testing for this release was conducted via frontend Vitest suites (113 tests passed) and clean disposable Windows Sandbox environments.

---

## 8. Development Verification Summary

- **Frontend Test Suite:** **113 passing tests** across 11 Vitest suites (`ApplicationCatalog`, `ProcessMonitor`, `SystemMonitor`, `LaunchButton`, `interaction`, `discovery`, `processes`, `launcher`, `system`, `ipc`, `ApplicationCard`).
- **Backend Structural Test Definitions:** **180 test definitions identified** in Rust source (143 unit test definitions in `src/`, 37 integration test definitions in `tests/`). Execution of these backend test definitions was not verified on the local host baseline because Windows Smart App Control (SAC) blocked unsigned debug build-scripts. Total test definitions identified: 293 (113 frontend passing + 180 backend identified).
- **Consolidated Release Verification Matrix (Phase 10.10):** Across comprehensive empirical QA testing, 27 distinct criteria were evaluated with **100% evidence coverage** (every criterion was investigated with empirical evidence, not that every criterion passed):
  - **24 criteria passed** (100% of executable functional, UI, and installer lifecycle tests in clean Windows 11 Sandbox environments).
  - **1 documented known limitation** (legacy v0.1.0 downgrade ingress).
  - **1 blocked criterion** (runtime verification on genuine Windows 10, pending a dedicated test harness; not an unconditional blocker for the Windows 11 preview).
  - **1 unsatisfied gate** (Authenticode digital signing for unrestricted general distribution).

---

## 9. Feedback & Contributing

DeskPilot is developed transparently on GitHub. Bug reports, technical feedback, and code contributions are welcome:

- **Repository:** `https://github.com/youssefmanssouri/deskpilot-`
- **Issue Tracker:** Submit reproduction steps and system information via GitHub Issues.
- **Architecture & Security:** See `README.md` and `PRIVACY.md` for architectural invariants.

