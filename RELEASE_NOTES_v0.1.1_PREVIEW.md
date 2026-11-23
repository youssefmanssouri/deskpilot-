# DeskPilot v0.1.1 — Developer Preview Release Notes

**Release Version:** v0.1.1 (Developer Preview)  
**Release Date:** November 2026  
**License:** [MIT License](LICENSE) (100% Free and Open Source Software)  
**Distribution Channel:** Controlled Preview / Developer Testing (Not General Consumer Availability)

---

## 1. Overview & Preview Scope

DeskPilot v0.1.1 is an open-source, local-first Windows command center engineered natively in **Rust**, **Tauri 2**, and **React 19**. It provides local application discovery, keyboard-first search, direct Win32 process execution, read-only process monitoring, and real-time system intelligence telemetry.

> [!IMPORTANT]
> **Controlled Preview Release Candidate Status:**
> This release candidate reflects the verified source at commit `1231b1cdebca6bc930c411af319ccb19178ee83b` (`1231b1c`), integrating the redesigned UI, native Windows icon extraction, Quick Launcher (`Ctrl+Alt+Space`), and focus dismissal repairs. It is packaged as a per-machine WiX MSI installer (`deskpilot_0.1.1_x64_en-US.msi`, SHA-256 `7EF4CBBC6EBCB759406067CE4108D617B0857E84EA07498C488B56BB7FD90CFD`). It is intended for technical evaluation and feedback. It is **not an unrestricted general consumer release**. Binaries are currently unsigned. Physical desktop acceptance of the uninstalled debug executable passed 100% on Windows 11 (Phase 16.21). In isolated Windows Sandbox testing (Phase 16.29), MSI installation, corrected public shortcuts, and uninstallation succeeded, but execution of the installed binary was blocked by container WDAC policy `VerifiedAndReputableDesktop` (Event 3077). Clean-machine installed-application runtime acceptance and Windows 10 validation remain pending. Automated test suites pass 100% (449 passing tests: 244 frontend + 205 backend). Please review the [Windows Security & Code Signing](#6-windows-security-code-signing--smartscreen-disclosures) and [Known Limitations](#7-known-limitations--operational-advisories) sections before installation.

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
| **Windows 11 (22H2 / 23H2 / 24H2)** | x64 | **Host Uninstalled Execution PASS / Sandbox Installer Lifecycle PASS / Sandbox Execution Blocked (WDAC)** | Structurally compatible. Uninstalled debug executable passed physical desktop acceptance (Phase 16.21). In Windows Sandbox (Phase 16.29), MSI installation, public shortcuts (`%PUBLIC%\Desktop` and `%PROGRAMDATA%\...\DeskPilot`), and uninstallation succeeded. However, execution of the installed unsigned binary was blocked by container WDAC policy `VerifiedAndReputableDesktop` (Event 3077). Installed-build runtime acceptance on a clean VM remains pending. |
| **Windows 10 (21H2 / 22H2)** | x64 | **Architecture Compatible / Runtime Verification Pending** | Win32 APIs and Evergreen WebView2 are structurally compatible. Windows 10 lacks Smart App Control, but genuine Windows 10 empirical runtime testing on physical or virtual hardware is still pending. |
| **Windows on ARM (ARM64)** | ARM64 | **Outside Tested Scope** | Compiles natively for `x86_64-pc-windows-msvc`; native ARM64 binaries are not built, and x64 emulation is unverified. |
| **Legacy Windows (7 / 8 / 8.1)** | Any | **Outside Target Scope** | DeskPilot relies on Windows 10+ Win32 APIs and the Evergreen WebView2 runtime. |

> [!NOTE]
> **Platform Scope & Validation Boundaries for Preview:**  
> While underlying Win32 APIs and WebView2 are standard across Windows 10 and 11, compatibility on Windows 10 has not yet been verified through empirical runtime execution on physical or virtual Windows 10 hardware. On Windows 11, the uninstalled debug executable ran cleanly on the physical desktop (Phase 16.21), but Windows Sandbox enforces WDAC policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`, blocking unsigned execution. Installed runtime validation on an isolated machine/VM without WDAC whitelisting remains pending.

---

## 4. Cryptographic Checksums & Artifact Integrity

Every release candidate binary is identified by its exact SHA-256 cryptographic digest. These checksums reflect the locally verified preview release candidate compiled from commit `1231b1cdebca6bc930c411af319ccb19178ee83b`:

| Artifact | File Name | Format | Size (Bytes) | SHA-256 Digest |
| :--- | :--- | :---: | :---: | :--- |
| **Windows MSI Installer** | `deskpilot_0.1.1_x64_en-US.msi` | WiX MSI | 4,538,368 | `7EF4CBBC6EBCB759406067CE4108D617B0857E84EA07498C488B56BB7FD90CFD` |
| **Standalone Executable** | `deskpilot.exe` | Win32 PE | 5,162,496 | `6BC7A9288456FA50144571F0167156ED01638264492B5AC3A9CE90D547AB2CA0` |
| **Extracted MSI Payload** | `deskpilot.exe` | Win32 PE | 5,162,496 | `4C3DD390D1B13693EC613F14D447A003D3587114D338E7C47AA24D5092166D4C` |

> [!NOTE]
> **Controlled Artifact Availability & Packaging Scope:**
> These binaries were compiled and verified locally from commit `1231b1cdebca6bc930c411af319ccb19178ee83b`. The NSIS installer format (`deskpilot_0.1.1_x64-setup.exe`) was formally discontinued in Phase 16.26; the preview candidate is strictly MSI-only. These artifacts are part of a private, controlled Developer Preview Release Candidate and are not published for general consumer download.

### PowerShell Verification Command
```powershell
Get-FileHash -Algorithm SHA256 .\deskpilot_0.1.1_x64_en-US.msi
# Expected output must match:
# 7EF4CBBC6EBCB759406067CE4108D617B0857E84EA07498C488B56BB7FD90CFD
```

### Executable Hash Provenance & Installed Payload Reconciliation

When verifying the installed binary at `C:\Program Files\deskpilot\deskpilot.exe` (or when extracting the MSI payload via administrative extraction `msiexec /a`), the resulting executable produces a different cryptographic hash:
- **MSI-Embedded Payload SHA-256:** `4C3DD390D1B13693EC613F14D447A003D3587114D338E7C47AA24D5092166D4C`
- **Standalone Release Binary SHA-256:** `6BC7A9288456FA50144571F0167156ED01638264492B5AC3A9CE90D547AB2CA0`

This hash variance is an expected property of the Tauri packaging pipeline:
1. **Compilation Origin:** Both executables originate from the exact same compilation invocation.
2. **The 3-Byte Bundler Tag:** Exactly **3 bytes** differ at file offset 4,864,919 in the `.rdata` section. The Tauri CLI bundler (`crates/tauri-bundler/src/bundle.rs`) automatically stamps a duplicate of the binary with package-type marker `__TAURI_BUNDLE_TYPE_VAR_MSI` (`4D 53 49`) prior to WiX packaging, whereas the standalone binary output in `target/release` retains the default `__TAURI_BUNDLE_TYPE_VAR_UNK` (`55 4E 4B`) marker.
3. **Machine Code Identity:** All machine instructions and assets are **100% byte-for-byte identical**. There is zero divergence in executable logic, security guards, or embedded frontend assets.

---

## 5. Installation, Upgrade & Uninstallation Guide

### Clean Installation
1. Download `deskpilot_0.1.1_x64_en-US.msi`.
2. Run the installer:
   ```cmd
   msiexec /i deskpilot_0.1.1_x64_en-US.msi
   ```
3. The installer installs the application per-machine (`ALLUSERS=1`) to `C:\Program Files\deskpilot` (requiring administrative elevation), places shortcuts in the Common Start Menu (`%ProgramData%\Microsoft\Windows\Start Menu\Programs\DeskPilot\DeskPilot.lnk`), Public Desktop (`%PUBLIC%\Desktop\DeskPilot.lnk`), registers an uninstallation shortcut in the Start Menu program folder, and registers a single entry in Windows Installed Apps (Add/Remove Programs). Erroneous legacy paths (`C:\Desktop`, `C:\deskpilot`) are completely eliminated.

> [!NOTE]
> **WebView2 Runtime Requirement:**  
> The installer embeds the official Microsoft Edge WebView2 bootstrapper (`MicrosoftEdgeWebview2Setup.exe`). If the Evergreen WebView2 Runtime is already installed on the target machine (standard on Windows 11 and modern updated Windows 10), installation completes fully offline. Active internet access is required during installation only if the target system lacks the WebView2 runtime.

### Tested Lifecycle Transitions & Downgrade Distinction
Container testing in Phase 10.9C and Phase 16.29 evaluated specific installer transitions:
- **Verified Successful Operations:** Silent fresh installation (`msiexec /i ... /qn`), public shortcut placement (`%PUBLIC%\Desktop` and `%PROGRAMDATA%\...`), multi-user account provisioning (`TestUser2`), major upgrade from v0.1.0 to v0.1.1 (clean binary replacement, single ARP registration, and user data survival), in-place maintenance repair (`msiexec /fa`), and silent uninstallation (`msiexec /x`).
- **Exposed Known Limitation (Legacy Downgrade):** Running the legacy v0.1.0 installer over v0.1.1 resulted in an unguarded downgrade because v0.1.0 was built without downgrade detection guards (see [Known Limitations](#7-known-limitations--operational-advisories)).
- **Testing Scope Distinction:** Installer lifecycle and shortcut directory layout were fully verified in isolated container testing, but runtime execution of the installed binary was blocked by container WDAC policy. Clean-machine installation lifecycle and interactive execution validation on a dedicated VM remain pending.

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
  In historical Phase 10.9C sandbox lifecycle testing (where the application was executed and WebView2 initialized), this directory measured 192 files totaling approximately 7.93 MB; these figures represent test-environment measurements and will vary based on user activity. In Phase 16.29 container testing, this directory was not created because application execution was blocked by WDAC prior to WebView2 initialization. The installer package does not automatically purge this directory during uninstallation; complete removal requires manually deleting this directory.

---

## 6. Windows Security, Code Signing & Application Control Disclosures

DeskPilot v0.1.1 release binaries are currently **unsigned** (`Get-AuthenticodeSignature` reports `NotSigned`):

### 1. Controlled Developer Evaluation & Unsigned Status
- The v0.1.1 preview binaries are unsigned. Code signing has not been completed, and no signing date has been scheduled.
- This release is intended strictly for controlled developer evaluation and technical feedback in non-production environments, not for unrestricted enterprise deployment.
- Unsigned binaries lack cryptographic publisher attribution and encounter standard Windows operating system defense prompts and Application Control policies.

### 2. Microsoft Defender SmartScreen Reputation & Warnings
- When downloading and running the unsigned MSI or standalone executable, Windows Defender SmartScreen may display an alert:
  *"Windows protected your PC — Microsoft Defender SmartScreen prevented an unrecognized app from starting."*
- SmartScreen assesses applications algorithmically based on download volume, telemetry, and established publisher reputation.
- Even if an Authenticode certificate (standard OV or hardware-token EV) is applied, warning-free execution is not guaranteed immediately. Reputation must accrue over time across multiple downloads before SmartScreen warnings subside.
- **Integrity Verification Guidance:** Users should obtain binaries exclusively from the official project repository release (`youssefmanssouri/deskpilot-`) and validate published SHA-256 checksums before running any file. Never download binaries from untrusted third-party mirrors.

### 3. Windows Application Control & Error 4551 Disclosures
- **Windows Application Control Execution Interception in Sandbox:** In isolated Windows 11 Enterprise Sandbox testing (Phases 16.27 & 16.29), process creation was intercepted and blocked by hypervisor Code Integrity enforcing policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}` (`VerifiedAndReputableDesktop`), generating Win32 Error `4551` (`0x11C7`, `ERROR_UNKNOWN_REVISION`) and Code Integrity Operational Events `3077`, `3089`, and `3099`. In contrast, on the physical developer host (Phase 16.21), the uninstalled debug executable executed successfully because user-mode code integrity policies were not actively enforcing signed-only whitelists.
- **Policy-Dependent Behavior:** Execution behavior depends entirely on the active security policy of the host or container environment (such as Smart App Control, App Control for Business / WDAC, or container hypervisor policies). When an application control policy enforces strict code integrity, unsigned binaries are blocked at the kernel loader without an inline user override prompt.
- **Code Signing Expectations:** Code signing with a trusted Authenticode certificate establishes verifiable publisher identity and allows eligibility for reputation evaluation. However, digital signing alone does not guarantee that every Application Control restriction will be automatically bypassed if the active policy mandates specific enterprise roots or whitelisted signers.

### 4. Benefits of Authenticode Signing
- Digital signing provides cryptographic verification that binaries have not been tampered with in transit, confirms publisher identity, and enables reputation accumulation across releases.

> [!CAUTION]
> **Security Advice:**  
> Never disable Microsoft Defender SmartScreen, Smart App Control, or Windows Application Control policies to run untrusted software. Do not instruct users to bypass enterprise security controls or blindly override Windows warnings. If your organizational or personal security policy requires signed binaries or prohibits running unsigned software, please defer evaluating DeskPilot until officially signed releases become available.

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
- On hosts where Windows Application Control or Smart App Control is enforced, unsigned Cargo build-scripts (`build.rs`) generated during local `cargo test` execution may be blocked. Full automated test suites pass 100% locally and in CI: 449 passing tests (244 frontend + 205 backend).

### 6. Clean-Machine Installation Lifecycle Pending
- While installer lifecycle transitions (install, repair, upgrade, uninstall, public shortcuts) were evaluated in isolated container environments, empirical validation of installed binary execution and second-user interactive login on a clean, production-configured Windows 11 machine or dedicated VM remains pending. (In Phase 16.31, dedicated Hyper-V VM provisioning was halted due to host RAM constraints and missing ISO media).

### 7. Windows 11 Sandbox Runtime Verification Scope & Protocol Boundaries (Phases 16.27 & 16.29)
- **Installer Lifecycle Validated:** In Phase 16.29, silent per-machine installation (`msiexec /i ... /qn`), public desktop shortcut placement (`%PUBLIC%\Desktop\DeskPilot.lnk`), Common Start Menu shortcut placement (`%PROGRAMDATA%\...\DeskPilot\DeskPilot.lnk`), secondary user account provisioning (`TestUser2`), and silent uninstallation (`msiexec /x`) were verified 100% pass in Windows Sandbox. Erroneous legacy paths (`C:\Desktop`, `C:\deskpilot`) were completely eliminated.
- **Kernel-Level WDAC Interception:** Windows Sandbox base images (Windows 11 Enterprise Build 26100) enforce hypervisor Code Integrity policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}` (`VerifiedAndReputableDesktop`). When attempting to launch `deskpilot.exe` inside the container, process creation was blocked with Win32 Error `4551` and Code Integrity Events `3077`, `3089`, and `3099`.
- **Silent Installation Protocol:** The MSI installer was validated using silent execution (`msiexec /i ... /qn`). Interactive graphical installer wizard dialog behavior was not tested.
- **Historical Note (Phase 13.9):** Earlier test iterations in Phase 13.9 observed interactive launch under different container policy conditions, but current Windows 11 Enterprise container environments actively enforce Code Integrity against unsigned binaries.
- **Second-User Interactive Login Scope:** Multi-user directory placement was verified with `TestUser2`, but second-user interactive GUI login and application launch were not tested due to Windows Sandbox's single-session (`WDAGUtilityAccount`) architecture.

### 8. Windows 10 Empirical Verification Pending
- DeskPilot is structurally compatible with Windows 10 (1809+) via standard Win32 APIs and Evergreen WebView2. However, empirical runtime execution and testing on physical or virtual Windows 10 hardware has not yet been conducted.

### 9. Release Artifact Publication Status
- The binaries, installer, and checksums documented in these release notes were generated and validated locally from commit `1231b1cdebca6bc930c411af319ccb19178ee83b` (`1231b1c`). These artifacts are preserved locally as a restricted developer preview release candidate and are not published for general public download.

---

## 8. Development Verification Summary

- **Frontend Test Suite Execution Evidence:** **244 passing tests** across 14 Vitest suites in `src/components/__tests__`, `src/launcher/__tests__`, and `src/lib/ipc/__tests__`.
- **Backend Test Suite Execution Evidence:** **205 passing tests** in Rust source (166 unit tests in `src-tauri/src` and 39 integration tests in `tests/cli.rs`, `tests/core.rs`, and `tests/security.rs`). Total automated tests passing: **449 passed** (0 failed, 100% pass rate).
- **TypeScript Compilation:** Zero compilation errors (`tsc --noEmit`).
- **Testing Boundary Distinction:** These automated test suites execute in headless environments (Node.js/JSDOM for Vitest; console test harness for Cargo). They empirically validate logical correctness, state transitions, binary parsing, and IPC contracts; they do **not** constitute native GUI surface verification, live WebView2 window rendering, or runtime acceptance by host security policies.
- **Consolidated Release Verification Matrix (Phase 16.33 Reconciliation):**
  - **Automated Tests:** 449 passing tests (244 frontend + 205 backend).
  - **Installer Architecture:** Per-machine WiX MSI packaging verified with 0 ICE errors and 0 ICE suppressions in WiX v3.14 (ICE38, ICE43, ICE57 satisfied with HKCU KeyPaths). Public shortcuts verified at `%PUBLIC%\Desktop\DeskPilot.lnk` and `%PROGRAMDATA%\Microsoft\Windows\Start Menu\Programs\DeskPilot\DeskPilot.lnk`. Erroneous `C:\Desktop` folder eliminated.
  - **1 verified PASS on physical desktop:** uninstalled debug executable focus restoration, 5-cycle launcher activation, and native 64×64 icon extraction (Phase 16.21).
  - **1 documented known limitation:** legacy v0.1.0 downgrade ingress (older v0.1.0 installer lacks downgrade guards).
  - **1 blocked criterion in sandbox:** Windows Sandbox container runtime execution (blocked by container hypervisor WDAC policy `{0283ac0f...}` Event 3077).
  - **1 pending verification:** clean-machine installed-application runtime acceptance and second-user interactive login in dedicated VM.
  - **1 pending verification:** empirical runtime verification on genuine Windows 10 hardware.
  - **1 unsatisfied gate:** Authenticode digital signing for unrestricted general distribution.
  - **Packaging format:** MSI-only (NSIS discontinued in Phase 16.26).

---

## 9. Feedback & Contributing

DeskPilot is developed transparently on GitHub. Bug reports, technical feedback, and code contributions are welcome:

- **Repository:** `https://github.com/youssefmanssouri/deskpilot-`
- **Issue Tracker:** Submit reproduction steps and system information via GitHub Issues.
- **Architecture & Security:** See `README.md` and `PRIVACY.md` for architectural invariants.

