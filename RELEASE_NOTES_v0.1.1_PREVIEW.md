# DeskPilot v0.1.1 — Developer Preview Release Notes

**Release Version:** v0.1.1 (Developer Preview)  
**Release Date:** November 2026  
**License:** [MIT License](LICENSE) (100% Free and Open Source Software)  
**Distribution Channel:** Public GitHub Developer Preview (Technical Evaluation, Not General Consumer Availability)

---

## 1. Overview & Preview Scope

DeskPilot v0.1.1 is an open-source, local-first Windows command center engineered natively in **Rust**, **Tauri 2**, and **React 19**. It provides local application discovery, keyboard-first search, direct Win32 process execution, read-only process monitoring, and real-time system intelligence telemetry.

> [!IMPORTANT]
> **Developer Preview Release Status:**
> This release reflects the verified source at commit `e99ccb84b1883a6a844e6576f2141a62ee74518f` (`e99ccb8`), integrating the redesigned UI, native Windows icon extraction, Quick Launcher (`Ctrl+Alt+Space`), focus dismissal repairs, and verified per-machine WiX shortcut packaging. It is packaged as a per-machine WiX MSI installer (`deskpilot_0.1.1_x64_en-US.msi`, SHA-256 `7EF4CBBC6EBCB759406067CE4108D617B0857E84EA07498C488B56BB7FD90CFD`). It is intended for technical evaluation and feedback. It is **not an unrestricted general consumer release**. Binaries are currently unsigned. On Windows 11 Pro 25H2, physical desktop installed-runtime acceptance passed 100% across all 15 functional acceptance dimensions, including an 86+ minute sustained soak observation (Phase 16.38). In isolated **Windows 11 Enterprise Build 26100 Windows Sandbox** testing (Phase 16.29), MSI installation, corrected public shortcuts, and uninstallation succeeded, while execution of the installed binary inside the container was blocked by container WDAC policy `VerifiedAndReputableDesktop` (Event 3077). Separately, GitHub Actions **Windows Server 2022** (`windows-2022`) provides automated installer smoke-test evidence (silent install, 20-second startup check, and uninstall), not desktop Windows compatibility evidence. **Windows 10** has not been tested on physical or virtual desktop hardware (the exact minimum supported Windows 10 build is undetermined). Automated test suites pass 100% (449 passing tests: 244 frontend + 205 backend). Please review the [Windows Security & Code Signing](#6-windows-security-code-signing--application-control-disclosures) and [Known Limitations](#7-known-limitations--operational-advisories) sections before installation.

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
| **Windows 11 Pro 25H2 (Physical Desktop)** | x64 | **Physical Desktop Acceptance PASS (Phase 16.38)** | Empirically validated installed runtime environment. Passed elevated major upgrade, installed executable startup, WebView2 Evergreen runtime initialization, Quick Launcher (`Ctrl+Alt+Space`), application discovery (138 apps), safe Win32 launch execution (`About Java` -> `javaw.exe`), read-only Process Monitor (1,784 entries), System Intelligence (exact host CIM metric parity), system tray with hide-to-tray, single-instance mutex, and an 86+ minute sustained soak observation (~29.4 MB working set, zero runaway heap growth observed). Other Windows 11 builds outside this test host have not been physically tested. |
| **Windows 11 Enterprise Build 26100 (Windows Sandbox)** | x64 | **Installer Lifecycle PASS / Execution Blocked (Phase 16.29, WDAC Event 3077)** | Used for clean container installer lifecycle testing. MSI installation, public shortcuts (`%PUBLIC%\Desktop` and `%PROGRAMDATA%\...`), and uninstallation succeeded, while execution of the unsigned binary inside the container was blocked by container WDAC policy `VerifiedAndReputableDesktop` (Event 3077). |
| **Windows Server 2022 (`windows-2022` CI)** | x64 | **Automated CI Installer Smoke Test PASS** | GitHub Actions workflow `installer-smoke-test.yml` verifies silent MSI install, 20-second startup check, and uninstall. This provides automated installer smoke-test evidence, not desktop Windows compatibility evidence. |
| **Windows 10** | x64 | **Untested (Minimum Build Undetermined)** | Has not been tested on physical or virtual desktop hardware; the exact minimum supported Windows 10 build is undetermined. Windows 10 is not claimed as validated or supported in this preview. |
| **Windows on ARM (ARM64)** | ARM64 | **Not Built / Not Supported** | Targets `x86_64-pc-windows-msvc` only; ARM64 binaries are not compiled, and x64 emulation on ARM64 is untested and unsupported. |
| **Legacy Windows (7 / 8 / 8.1)** | Any | **Outside Intended Support Scope** | Excluded from DeskPilot's intended modern 64-bit Windows scope; DeskPilot relies on modern Windows APIs and the Evergreen WebView2 runtime. |

> [!NOTE]
> **Platform Scope & Validation Boundaries for Preview:**
> DeskPilot targets 64-bit Windows (`x86_64`) as its intended scope. Empirically, full desktop runtime validation was completed only on **Windows 11 Pro 25H2** (Phase 16.38); **Windows 11 Enterprise Build 26100 Windows Sandbox** (Phase 16.29) verified MSI lifecycle operations while blocking unsigned execution under container WDAC policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`, and GitHub Actions **Windows Server 2022** (`windows-2022`) provides automated CI installer smoke-test evidence only. **Windows 10** has not been tested on physical or virtual desktop hardware and has no verified minimum build, while **Windows 7, 8, 8.1, and ARM64** are outside the supported scope. Host environments enforcing strict code integrity policies may block unsigned execution without an inline prompt.

---

## 4. Cryptographic Checksums & Artifact Integrity

Every release candidate binary is identified by its exact SHA-256 cryptographic digest. These checksums reflect the verified preview release candidate compiled from commit `e99ccb84b1883a6a844e6576f2141a62ee74518f` (`e99ccb8`):

| Artifact | File Name | Format | Size (Bytes) | SHA-256 Digest |
| :--- | :--- | :---: | :---: | :--- |
| **Windows MSI Installer** | `deskpilot_0.1.1_x64_en-US.msi` | WiX MSI | 4,538,368 | `7EF4CBBC6EBCB759406067CE4108D617B0857E84EA07498C488B56BB7FD90CFD` |
| **Standalone Executable** | `deskpilot.exe` | Win32 PE | 5,162,496 | `6BC7A9288456FA50144571F0167156ED01638264492B5AC3A9CE90D547AB2CA0` |
| **Extracted MSI Payload** | `deskpilot.exe` | Win32 PE | 5,162,496 | `4C3DD390D1B13693EC613F14D447A003D3587114D338E7C47AA24D5092166D4C` |

> [!NOTE]
> **Artifact Packaging & Availability Scope:**
> These binaries were compiled and verified from commit `e99ccb84b1883a6a844e6576f2141a62ee74518f`. The NSIS installer format (`deskpilot_0.1.1_x64-setup.exe`) was formally discontinued in Phase 16.26; the preview release is strictly MSI-only. These artifacts are published on GitHub Releases as a Developer Preview for technical evaluation and developer testing.

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
> The installer embeds the official Microsoft Edge WebView2 bootstrapper (`MicrosoftEdgeWebview2Setup.exe`). If the Evergreen WebView2 Runtime is already installed on the target machine (present by default on Windows 11), installation completes fully offline. Active internet access is required during installation only if the target system lacks the WebView2 runtime.

### Tested Lifecycle Transitions & Downgrade Distinction
Container testing in Phase 10.9C and **Windows 11 Enterprise Build 26100 Windows Sandbox** testing in Phase 16.29 evaluated specific installer transitions:
- **Verified Successful Operations:** Silent fresh installation (`msiexec /i ... /qn`), public shortcut placement (`%PUBLIC%\Desktop` and `%PROGRAMDATA%\...`), multi-user account provisioning (`TestUser2`), major upgrade from v0.1.0 to v0.1.1 (clean binary replacement, single ARP registration, and existing user data intact), in-place maintenance repair (`msiexec /fa`), and silent uninstallation (`msiexec /x`).
- **Exposed Known Limitation (Legacy Downgrade):** Running the legacy v0.1.0 installer over v0.1.1 resulted in an unguarded downgrade because v0.1.0 was built without downgrade detection guards (see [Known Limitations](#7-known-limitations--operational-advisories)).
- **Testing Scope Distinction:** Installer lifecycle and shortcut directory layout were verified in **Windows 11 Enterprise Build 26100 Windows Sandbox** (Phase 16.29), while runtime execution of the unsigned binary inside that container was blocked by WDAC policy. Separately, GitHub Actions **Windows Server 2022** (`windows-2022`, `installer-smoke-test.yml`) provides automated installer smoke-test evidence (silent install, 20-second startup check, and uninstall), not desktop Windows compatibility evidence. Interactive wizard installation and desktop VM execution remain outside automated verification.

### Major Upgrade from Earlier Versions
- Running `deskpilot_0.1.1_x64_en-US.msi` on a system with DeskPilot v0.1.0 performs a standard Windows Installer major upgrade.
- **Data Scope During Upgrades:** The MSI package does **not** explicitly manage, migrate, or delete persistent JSON files under `%APPDATA%\com.deskpilot.app\` (`settings.json`, `launch-history.json`), the Microsoft Edge WebView2 profile under `%LOCALAPPDATA%\com.deskpilot.app\EBWebView\`, or the optional per-user startup registry value (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\DeskPilot`, created only when *Start with Windows* is enabled). Those items sit outside the MSI's explicit upgrade-removal scope. Existing files were observed intact after the v0.1.0-to-v0.1.1 upgrade tested in Phase 16.38, though only that transition and in-place repair were tested, and schema compatibility or automatic migration across arbitrary future schema changes is not guaranteed.

### In-Place Repair
- If installed files become corrupted, run the repair command:
  ```cmd
  msiexec /fa deskpilot_0.1.1_x64_en-US.msi
  ```

### Uninstallation & Retained Data Notice
- If you enabled **Start with Windows**, disable it inside DeskPilot Settings **before** uninstalling so the optional startup registry entry is removed cleanly. Uninstall via **Windows Settings > Apps > Installed apps**, or via command line:
  ```cmd
  msiexec /x deskpilot_0.1.1_x64_en-US.msi
  ```
- **Application Removal:** The uninstallation test demonstrated clean removal of application binaries (`C:\Program Files\deskpilot`), Start Menu and Desktop shortcuts, and Add/Remove Programs registry entries.
- **Data & Registry Retention Outside MSI Removal Scope:** Uninstallation does **not** automatically delete:
  - **Persistent JSON application data:** `%APPDATA%\com.deskpilot.app\settings.json` and `%APPDATA%\com.deskpilot.app\launch-history.json`
  - **Microsoft Edge WebView2 profile data:**
    ```text
    %LOCALAPPDATA%\com.deskpilot.app\EBWebView
    ```
    In historical Phase 10.9C sandbox lifecycle testing (where the application was executed and WebView2 initialized), this directory measured 192 files totaling approximately 7.93 MB; these figures represent test-environment measurements and will vary based on user activity. In Phase 16.29 container testing, this directory was not created because application execution was blocked by WDAC prior to WebView2 initialization.
  - **Optional startup registry value:** `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\DeskPilot` (created only when the user explicitly enables *Start with Windows*), which can remain pointing to the removed executable if *Start with Windows* was not disabled in DeskPilot Settings prior to uninstallation.
- **Manual Cleanup & Reinstallation Behavior:** The installer package does not automatically purge `%APPDATA%\com.deskpilot.app\` or `%LOCALAPPDATA%\com.deskpilot.app\`; complete local file removal requires manually deleting those directories after uninstalling. Data left outside the MSI removal scope may remain available after reinstalling DeskPilot, though recovery of incompatible or corrupted application data is not guaranteed.

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

### 6. Installed Runtime Lifecycle Validated on Physical Windows 11 Pro 25H2
- In Phase 16.38, elevated major upgrade, installed binary execution, and full 15-point functional acceptance were verified on a physical Windows 11 Pro 25H2 desktop across an 86+ minute soak observation (~29.4 MB working set, zero runaway heap growth observed). Application JSON state under `%APPDATA%\com.deskpilot.app\` (`settings.json`, `launch-history.json`) and WebView2 profile state under `%LOCALAPPDATA%\com.deskpilot.app\EBWebView\` (outside the MSI's explicit upgrade-removal scope) remained intact across the tested upgrade. Windows 10 desktop validation (minimum build undetermined) and second-user interactive GUI login testing remain pending.

### 7. Windows 11 Sandbox Runtime Verification Scope & Protocol Boundaries (Phases 16.27 & 16.29)
- **Installer Lifecycle Validated:** In Phase 16.29, silent per-machine installation (`msiexec /i ... /qn`), public desktop shortcut placement (`%PUBLIC%\Desktop\DeskPilot.lnk`), Common Start Menu shortcut placement (`%PROGRAMDATA%\...\DeskPilot\DeskPilot.lnk`), secondary user account provisioning (`TestUser2`), and silent uninstallation (`msiexec /x`) were verified 100% pass in Windows Sandbox. Erroneous legacy paths (`C:\Desktop`, `C:\deskpilot`) were completely eliminated.
- **Kernel-Level WDAC Interception:** Windows Sandbox base images (Windows 11 Enterprise Build 26100) enforce hypervisor Code Integrity policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}` (`VerifiedAndReputableDesktop`). When attempting to launch `deskpilot.exe` inside the container, process creation was blocked with Win32 Error `4551` and Code Integrity Events `3077`, `3089`, and `3099`.
- **Silent Installation Protocol:** The MSI installer was validated using silent execution (`msiexec /i ... /qn`). Interactive graphical installer wizard dialog behavior was not tested.
- **Historical Note (Phase 13.9):** Earlier test iterations in Phase 13.9 observed interactive launch under different container policy conditions, but current Windows 11 Enterprise container environments actively enforce Code Integrity against unsigned binaries.
- **Second-User Interactive Login Scope:** Multi-user directory placement was verified with `TestUser2`, but second-user interactive GUI login and application launch were not tested due to Windows Sandbox's single-session (`WDAGUtilityAccount`) architecture.

### 8. Windows 10 Empirical Verification Pending (Untested)
- Windows 10 has **not** been tested on physical or virtual desktop hardware, and the exact minimum supported Windows 10 build is **undetermined**. While DeskPilot uses modern 64-bit Windows APIs and Evergreen WebView2, Windows 10 is not claimed as validated or supported in this preview prior to empirical desktop verification.

### 9. Release Artifact Publication Status
- The binaries, installer, and checksums documented in these release notes were generated and validated from commit `e99ccb84b1883a6a844e6576f2141a62ee74518f` (`e99ccb8`). These artifacts are published on GitHub Releases as a Developer Preview for technical evaluation and developer testing.

---

## 8. Development Verification Summary

- **Frontend Test Suite Execution Evidence:** **244 passing tests** across 14 Vitest suites in `src/components/__tests__`, `src/launcher/__tests__`, and `src/lib/ipc/__tests__`.
- **Backend Test Suite Execution Evidence:** **205 passing tests** in Rust source (166 unit tests in `src-tauri/src` and 39 integration tests in `tests/cli.rs`, `tests/core.rs`, and `tests/security.rs`). Total automated tests passing: **449 passed** (0 failed, 100% pass rate).
- **TypeScript Compilation:** Zero compilation errors (`tsc --noEmit`).
- **Testing Boundary Distinction:** These automated test suites execute in headless environments (Node.js/JSDOM for Vitest; console test harness for Cargo). They empirically validate logical correctness, state transitions, binary parsing, and IPC contracts; they do **not** constitute native GUI surface verification, live WebView2 window rendering, or runtime acceptance by host security policies.
- **Consolidated Release Verification Matrix (Phase 16.40 Release Preparation):**
  - **Automated Tests:** 449 passing tests (244 frontend + 205 backend).
  - **Installer Architecture:** Per-machine WiX MSI packaging verified with 0 ICE errors and 0 ICE suppressions in WiX v3.14 (committed at `e99ccb8`). Public shortcuts verified at `%PUBLIC%\Desktop\DeskPilot.lnk` and `%PROGRAMDATA%\Microsoft\Windows\Start Menu\Programs\DeskPilot\DeskPilot.lnk`. Erroneous `C:\Desktop` folder eliminated.
  - **1 verified PASS on physical desktop installed runtime (Phase 16.38, Windows 11 Pro 25H2):** 15/15 functional dimensions passed 100%, including elevated upgrade, startup, WebView2 Evergreen runtime initialization, Quick Launcher, app discovery (138 apps), safe launch (`About Java`), Process Monitor (1,784 entries), System Intelligence (exact host CIM metric parity), system tray with hide-to-tray, single-instance mutex, 86+ minute sustained soak observation (~29.4 MB working set, zero runaway heap growth observed), and retention of existing user files across the tested upgrade.
  - **Automated CI smoke test (GitHub Actions Windows Server 2022, `windows-2022`):** `installer-smoke-test.yml` passes silent install, 20-second startup check, and uninstall (automated CI smoke-test evidence only; not desktop compatibility evidence).
  - **1 documented known limitation:** legacy v0.1.0 downgrade ingress (older v0.1.0 installer lacks downgrade guards).
  - **1 blocked criterion in sandbox (Windows 11 Enterprise Build 26100 Windows Sandbox):** container runtime execution of the unsigned binary (blocked by container hypervisor WDAC policy `{0283ac0f...}` Event 3077).
  - **1 pending verification:** empirical runtime verification on Windows 10 (physical or virtual desktop hardware; minimum supported Windows 10 build is undetermined).
  - **1 unsatisfied gate for General Availability:** Authenticode digital signing for unrestricted general distribution.
  - **Packaging format:** MSI-only (NSIS discontinued in Phase 16.26).

---

## 9. Feedback & Contributing

DeskPilot is developed transparently on GitHub. Bug reports, technical feedback, and code contributions are welcome:

- **Repository:** `https://github.com/youssefmanssouri/deskpilot-`
- **Issue Tracker:** Submit reproduction steps and system information via GitHub Issues.
- **Architecture & Security:** See `README.md` and `PRIVACY.md` for architectural invariants.

