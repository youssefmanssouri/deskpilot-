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
> This release is a **controlled developer and early-adopter preview**, intended for technical evaluation and feedback. It is **not an unrestricted general consumer release**. Binaries are currently unsigned. On the tested Windows 11 machine, native process execution was intercepted by Windows Application Control (Win32 Error 4551), and empirical runtime verification on Windows 10 remains pending. Automated test suites pass 100% (383 passing tests: 175 frontend + 208 backend). Please review the [Windows Security & Code Signing](#6-windows-security-code-signing--smartscreen-disclosures) and [Known Limitations](#7-known-limitations--operational-advisories) sections before installation.

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
- **Manual Catalog Refresh (Phase 12.8):** Dedicated manual refresh button with real-time spin animation, `aria-busy` accessibility attributes, and concurrency guards preventing overlapping discovery scans.

### ⚡ Keyboard-First Real-Time Search & Ranking
- Four-tier ranking hierarchy prioritizing exact title matches, prefix matches, substring matches, and publisher metadata.
- Interactive keyboard navigation (arrow keys, Enter to launch, Escape to clear search query).
- Empty-state rendering when queries yield no matching applications.

### 🛡️ Win32 Native Application Launching
- Direct process spawning via Windows `CreateProcessW` without intermediate shell interpreters (`cmd.exe` or `powershell.exe`), eliminating shell command injection risks.
- Resolves working directories to target executable parent directories or system roots, scoping execution context and avoiding reliance on ambient working directories.
- Launches processes detached from parent handle inheritance.
- Detects when an application requires administrative elevation (Win32 Error 740, `ERROR_ELEVATION_REQUIRED`) and provides a structured permission-denied notification, explaining that DeskPilot launches applications with standard user privileges.

### 🔒 Security Hardening & Sidebar Functional Honesty
- **Elimination of Non-Functional Placeholders (Phase 12.8):** Removed decorative, non-operational sidebar controls (Settings, Terminal Workspace, Window Layout) to ensure complete functional honesty across the UI surface.
- **Unused Dependency Removal:** Completely eliminated unused `tauri-plugin-opener` plugin and its capability permissions from backend crates, builder initialization, and capability definitions.
- **Content Security Policy (CSP):** Enforced strict Content Security Policy (`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self'; object-src 'none'; base-uri 'self';`) restricting script execution, style loading, and media sources to trusted origins.
- **Clean Installer Hierarchy:** WiX package configuration explicitly maps the uninstall shortcut to `ApplicationProgramsFolder` (`%ProgramData%\Microsoft\Windows\Start Menu\Programs\deskpilot\`).
- **Standardized Application Title:** Window title explicitly set to `DeskPilot`.

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
| **Windows 11 (22H2 / 23H2 / 24H2)** | x64 | **Architecture Compatible / Native Runtime Blocked** | Structurally compatible. Passed automated suites and disposable Sandbox evaluation. However, on host machines enforcing Windows Application Control, native execution of the unsigned binary is currently blocked (Win32 Error 4551). |
| **Windows 10 (21H2 / 22H2)** | x64 | **Architecture Compatible / Runtime Verification Pending** | Win32 APIs and Evergreen WebView2 are structurally compatible. Windows 10 lacks Smart App Control, but genuine Windows 10 empirical runtime testing on physical or virtual hardware is still pending. |
| **Windows on ARM (ARM64)** | ARM64 | **Outside Tested Scope** | Compiles natively for `x86_64-pc-windows-msvc`; native ARM64 binaries are not built, and x64 emulation is unverified. |
| **Legacy Windows (7 / 8 / 8.1)** | Any | **Outside Target Scope** | DeskPilot relies on Windows 10+ Win32 APIs and the Evergreen WebView2 runtime. |

> [!NOTE]
> **Platform Scope for Preview:**  
> While underlying Win32 APIs and WebView2 are standard across Windows 10 and 11, compatibility on Windows 10 has not yet been verified through empirical runtime execution on physical or virtual Windows 10 hardware. On Windows 11, native process execution is subject to active Windows Application Control policies.

---

## 4. Cryptographic Checksums & Artifact Integrity

Every release binary is identified by its exact SHA-256 cryptographic digest. These checksums reflect the official build produced by GitHub Actions CI run `43790285367` from binary source commit `0772d4661589f2e87a42bc2527b165f3c92cc3ee`:

| Artifact | File Name | Format | Size (Bytes) | SHA-256 Digest |
| :--- | :--- | :---: | :---: | :--- |
| **Windows Installer** | `deskpilot_0.1.1_x64_en-US.msi` | WiX MSI | 4,149,248 | `8d05ed157bbc6660f97f48bf1b00f5c44d0c096f1aecffd7f085449df7e9578e` |
| **Standalone Executable** | `deskpilot.exe` | Win32 PE | 4,613,632 | `12be660a18f48f1451531dd0723b31839e7b625ee5835bcfe77ad1a27a65e26f` |
| **Build Manifest** | `build-manifest.json` | JSON | 3,391 | `df61b0863da6907441337e76924f96b2870bd64ed4b67a83c4bc3b7c04c8675a` |
| **Checksum Manifest** | `SHA256SUMS.txt` | Text | 176 | `14d613dba2464523506ab43c13e020c14ceb6f330306be3f91d2d8611de93d82` |

> [!NOTE]
> **Controlled Artifact Availability:**
> These binaries were compiled and verified within hosted GitHub Actions CI run `43790285367`. The preview tag `v0.1.1-preview` points to binary source commit `0772d4661589f2e87a42bc2527b165f3c92cc3ee`. These artifacts are part of a private, controlled Developer Preview and are not published for general consumer download.

### PowerShell Verification Command
```powershell
Get-FileHash -Algorithm SHA256 .\deskpilot_0.1.1_x64_en-US.msi
# Expected output must match:
# 8D05ED157BBC6660F97F48BF1B00F5C44D0C096F1AECFFD7F085449DF7E9578E
```

---

## 5. Installation, Upgrade & Uninstallation Guide

### Clean Installation
1. Download `deskpilot_0.1.1_x64_en-US.msi`.
2. Run the installer:
   ```cmd
   msiexec /i deskpilot_0.1.1_x64_en-US.msi
   ```
3. The installer installs the application per-machine to `C:\Program Files\deskpilot` (requiring administrative elevation), places shortcuts in the Start Menu (`ApplicationProgramsFolder`), Desktop (`CommonDesktopFolder`), registers an uninstallation shortcut in the Start Menu program folder, and registers a single entry in Windows Installed Apps (Add/Remove Programs).

> [!NOTE]
> **WebView2 Runtime Requirement:**  
> The installer embeds the official Microsoft Edge WebView2 bootstrapper (`MicrosoftEdgeWebview2Setup.exe`). If the Evergreen WebView2 Runtime is already installed on the target machine (standard on Windows 11 and modern updated Windows 10), installation completes fully offline. Active internet access is required during installation only if the target system lacks the WebView2 runtime.

### Tested Lifecycle Transitions & Downgrade Distinction
Sandbox testing in Phase 10.9C evaluated specific installer transitions:
- **Verified Successful Operations:** Silent fresh installation (`msiexec /i`), major upgrade from v0.1.0 to v0.1.1 (clean binary replacement, single ARP registration, and user data survival), in-place maintenance repair (`msiexec /fa`), and silent uninstallation (`msiexec /x`).
- **Exposed Known Limitation (Legacy Downgrade):** Running the legacy v0.1.0 installer over v0.1.1 resulted in an unguarded downgrade because v0.1.0 was built without downgrade detection guards (see [Known Limitations](#7-known-limitations--operational-advisories)).
- **Testing Scope Distinction:** Only these specific lifecycle transitions were evaluated in isolated sandbox environments; clean-machine installation lifecycle validation on dedicated production systems remains pending.

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

## 6. Windows Security, Code Signing & Application Control Disclosures

DeskPilot v0.1.1 release binaries are currently **unsigned** (`Get-AuthenticodeSignature` reports `NotSigned`):

### 1. Unsigned Publisher Status
- An Authenticode code signing certificate is not strictly necessary to publish, download, or evaluate an open-source preview release in an unconstrained environment.
- However, unsigned binaries lack cryptographic publisher attribution and encounter standard Windows operating system defense prompts and Application Control policies.

### 2. Microsoft Defender SmartScreen Reputation & Warnings
- When downloading and running the unsigned MSI or standalone executable, Windows Defender SmartScreen may display an alert:
  *"Windows protected your PC — Microsoft Defender SmartScreen prevented an unrecognized app from starting."*
- SmartScreen assesses applications algorithmically based on download volume, telemetry, and established publisher reputation.
- Even if an Authenticode certificate (standard OV or hardware-token EV) is applied, warning-free execution is not guaranteed immediately. Reputation must accrue over time across multiple downloads before SmartScreen warnings subside.

### 3. Windows Application Control & Error 4551 Disclosures
- **Windows Application Control Execution Interception:** Windows Application Control blocked execution of the unsigned application on the tested Windows 11 machine (Win32 Error `4551` / `0x11C7`, Code Integrity Operational Events `3033` and `3077` under Policy ID `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`).
- **Policy-Dependent Behavior:** Execution behavior depends entirely on the active security policy of the host machine (such as Smart App Control or App Control for Business / WDAC). When an application control policy enforces strict code integrity, unsigned binaries are blocked at the kernel loader without an inline user override prompt.
- **Code Signing Expectations:** Code signing with a trusted Authenticode certificate establishes verifiable publisher identity and allows eligibility for reputation evaluation. However, digital signing alone does not guarantee that every Application Control restriction will be automatically bypassed if the active policy mandates specific enterprise roots or whitelisted signers.

### 4. Benefits of Authenticode Signing
- Digital signing provides cryptographic verification that binaries have not been tampered with in transit, confirms publisher identity, and enables reputation accumulation across releases.

> [!CAUTION]
> **Security Advice:**  
> Never disable Microsoft Defender SmartScreen, Smart App Control, or Windows Application Control policies to run untrusted software. If your organizational or personal security policy prohibits running unsigned software, please defer evaluating DeskPilot until officially signed releases become available.

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

### 5. Windows Application Control Impact on Developer Builds
- On hosts where Windows Application Control or Smart App Control is enforced, unsigned Cargo build-scripts (`build.rs`) generated during local `cargo test` execution may be blocked. Full backend test execution for this release was validated via the hosted CI pipeline (208 tests passed in hosted CI run `43790285367`), and frontend tests were validated via Vitest (175 tests passed).

### 6. Clean-Machine Installation Lifecycle Pending
- While installer lifecycle transitions (install, repair, upgrade, uninstall) were evaluated in isolated sandbox environments, empirical validation on a clean, production-configured Windows 11 machine without developer tooling remains pending.

### 7. Release Artifact Publication Status
- The binaries, installer, and checksums documented in these release notes were generated and validated by hosted GitHub Actions CI run `43790285367` from commit `0772d4661589f2e87a42bc2527b165f3c92cc3ee`. The GitHub Draft Release `438757492` and preview tag `v0.1.1-preview` are synchronized with this commit; these artifacts remain in a private draft release and are not published for general public download.

---

## 8. Development Verification Summary

- **Frontend Test Suite Execution Evidence:** **175 passing tests** across 13 Vitest suites (`ApplicationCatalog` [30], `ProcessMonitor` [24], `SystemMonitor` [27], `LaunchButton` [12], `interaction` [32], `discovery` [8], `processes` [7], `launcher` [6], `system` [4], `ipc` [4], `ApplicationCard` [6], `ApplicationInspector` [8], `ProcessInspector` [7]).
- **Backend Test Suite Execution Evidence:** **208 passing tests** in Rust source across six targets (144 unit tests in `src/lib.rs` including elevation handling test `test_spawn_error_elevation_required`, 14 in `tests/cli.rs`, 38 in `tests/core.rs`, 12 in `tests/security.rs`) verified in hosted CI run `43790285367` from source commit `0772d4661589f2e87a42bc2527b165f3c92cc3ee`. Total automated tests executed and passing: **383 passed** (0 failed, 100% pass rate).
- **TypeScript Compilation:** Zero compilation errors (`tsc --noEmit`).
- **Testing Boundary Distinction:** These automated test suites execute in headless environments (Node.js/JSDOM for Vitest; console test harness for Cargo). They empirically validate logical correctness, state transitions, binary parsing, and IPC contracts; they do **not** constitute native GUI surface verification, live WebView2 window rendering, or runtime acceptance by host security policies.
- **Consolidated Release Verification Matrix (Phase 12.11):**
  - **Automated Tests:** 383 passing tests (175 frontend + 208 backend).
  - **Installer Architecture:** Per-machine WiX MSI packaging verified with 0 ICE errors in CI.
  - **1 documented known limitation:** legacy v0.1.0 downgrade ingress.
  - **1 blocked criterion:** native Windows 11 runtime execution (blocked by Windows Application Control with Error 4551 on enforced hosts).
  - **1 pending verification:** empirical runtime verification on genuine Windows 10 hardware/VMs.
  - **1 pending verification:** clean-machine installation lifecycle validation on dedicated Windows 11 hardware.
  - **1 unsatisfied gate:** Authenticode digital signing for unrestricted general distribution.
  - **Sidebar Functional Honesty:** Non-functional Settings, Terminal, and Window Layout controls were removed from the navigation sidebar in Phase 12.8, and explicit manual refresh action with loading feedback and concurrency protection was added for Application Discovery.

---

## 9. Feedback & Contributing

DeskPilot is developed transparently on GitHub. Bug reports, technical feedback, and code contributions are welcome:

- **Repository:** `https://github.com/youssefmanssouri/deskpilot-`
- **Issue Tracker:** Submit reproduction steps and system information via GitHub Issues.
- **Architecture & Security:** See `README.md` and `PRIVACY.md` for architectural invariants.

