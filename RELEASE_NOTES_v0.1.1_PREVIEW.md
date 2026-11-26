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
> This document records the verified v0.1.1 build from commit `effdb09971edf4746e0388fe714564ce477ee6a5` (GitHub Actions release build `45693205561`), integrating the redesigned UI, native Windows icon extraction, Quick Launcher (`Ctrl+Alt+Space`), focus dismissal repairs, guarded `HKCU` startup registry management, and verified per-machine WiX shortcut packaging. **Publication status:** the latest published GitHub Release is the earlier `v0.1.1-preview.2` tag; this newer `effdb099` build (run `45693205561`) has **not yet been published** as a GitHub Release (workflow artifacts are temporary CI outputs, not permanent public release downloads). It is packaged as a per-machine WiX MSI installer (`deskpilot_0.1.1_x64_en-US.msi`, SHA-256 `b6e4001f44a9c05d7577e33ba0fab6ffa99b68e9e099b3a56d56d337e5dfd29f`). It is intended for technical evaluation and feedback, **not an unrestricted general consumer release**. Binaries are **unsigned** (`NotSigned`); users may encounter *Unknown publisher* or SmartScreen warnings, and warning-free installation is not promised. On **Windows 11 Pro 25H2** (build `26200.9974`, x64), physical desktop acceptance (Phase 18.4) passed 15/15 permitted functional checks, including 137 apps discovered, guarded startup states (`Disabled`, `Enabled`, `Stale`, `OtherInstall`, `Unrecognized`), upgrade data preservation, and a 31-minute runtime observation without crashes or unexpected resource growth. In isolated **Windows 11 Enterprise Build 26100 Windows Sandbox** testing (Phase 16.29), MSI installation, corrected public shortcuts, and uninstallation succeeded, while execution of the unsigned binary inside the container was blocked by container WDAC policy `VerifiedAndReputableDesktop` (Event 3077). Separately, GitHub Actions **Windows Server 2022** (`windows-2022`, run `45692834602`) provides automated installer smoke-test evidence (silent install, 20-second startup check, and uninstall), not desktop Windows compatibility evidence. **Windows 10** has not been tested on physical or virtual desktop hardware (the exact minimum supported Windows 10 build is undetermined). Automated test suites pass 100% (511 total: 260 Rust + 251 frontend, headless unit/integration tests). Please review the [Windows Security & Code Signing](#6-windows-security-code-signing--application-control-disclosures) and [Known Limitations](#7-known-limitations--operational-advisories) sections before installation.

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
| **Windows 11 Pro 25H2 (Physical Desktop)** | x64 | **Physical Desktop Acceptance PASS (Phase 18.4, Build `26200.9974`)** | Empirically validated installed runtime environment (earlier preview validated in Phase 16.38). Passed 15/15 permitted functional acceptance checks: elevated in-place MSI upgrade with preserved settings/history, 137 apps discovered, safe Win32 launch (`About Java` -> `javaw.exe`), read-only Process Monitor (253 processes), System Intelligence (CPU, RAM, drives, uptime), `--autostart` tray launch, single-instance mutex, guarded startup registry states (`Disabled`, `Enabled`, `Stale`, `OtherInstall`, `Unrecognized`), and a 31-minute runtime observation (`26.24 -> 23.48 MB` working set, zero crashes or unexpected resource growth). Other Windows 11 builds outside this test host have not been physically tested. |
| **Windows 11 Enterprise Build 26100 (Windows Sandbox)** | x64 | **Installer Lifecycle PASS / Execution Blocked (Phase 16.29, WDAC Event 3077)** | Used for clean container installer lifecycle testing. MSI installation, public shortcuts (`%PUBLIC%\Desktop` and `%PROGRAMDATA%\...`), and uninstallation succeeded, while execution of the unsigned binary inside the container was blocked by container WDAC policy `VerifiedAndReputableDesktop` (Event 3077). |
| **Windows Server 2022 (`windows-2022` CI)** | x64 | **Automated CI Installer Smoke Test PASS (Run `45692834602`)** | GitHub Actions workflow `installer-smoke-test.yml` verifies silent MSI install, 20-second startup check, and uninstall. This provides automated installer smoke-test evidence, not desktop Windows compatibility evidence. |
| **Windows 10** | x64 | **Untested (Minimum Build Undetermined)** | Has not been tested on physical or virtual desktop hardware; the exact minimum supported Windows 10 build is undetermined. Windows 10 is not claimed as validated or supported in this preview. |
| **Windows on ARM (ARM64)** | ARM64 | **Not Built / Not Supported** | Targets `x86_64-pc-windows-msvc` only; ARM64 binaries are not compiled, and x64 emulation on ARM64 is untested and unsupported. |
| **Legacy Windows (7 / 8 / 8.1)** | Any | **Outside Intended Support Scope** | Excluded from DeskPilot's intended modern 64-bit Windows scope; DeskPilot relies on modern Windows APIs and the Evergreen WebView2 runtime. |

> [!NOTE]
> **Platform Scope & Validation Boundaries for Preview:**
> DeskPilot targets 64-bit Windows (`x86_64`) as its intended scope. Empirically, full desktop runtime validation was completed only on **Windows 11 Pro 25H2** (build `26200.9974` in Phase 18.4; earlier preview in Phase 16.38); **Windows 11 Enterprise Build 26100 Windows Sandbox** (Phase 16.29) verified MSI lifecycle operations while blocking unsigned execution under container WDAC policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`, and GitHub Actions **Windows Server 2022** (`windows-2022`, run `45692834602`) provides automated CI installer smoke-test evidence only. **Windows 10** has not been tested on physical or virtual desktop hardware and has no verified minimum build, while **Windows 7, 8, 8.1, and ARM64** are outside the supported scope. Host environments enforcing strict code integrity policies may block unsigned execution without an inline prompt.

---

## 4. Cryptographic Checksums & Artifact Integrity

Every release candidate binary is identified by its exact SHA-256 cryptographic digest. These checksums reflect the verified preview release candidate compiled from commit `effdb09971edf4746e0388fe714564ce477ee6a5` (GitHub Actions run `45693205561`, `build-manifest.json` & `SHA256SUMS.txt`):

| Artifact | File Name | Format | Size (Bytes) | SHA-256 Digest |
| :--- | :--- | :---: | :---: | :--- |
| **Windows MSI Installer** | `deskpilot_0.1.1_x64_en-US.msi` | WiX MSI | 4,472,832 | `b6e4001f44a9c05d7577e33ba0fab6ffa99b68e9e099b3a56d56d337e5dfd29f` |
| **Standalone Executable** | `deskpilot.exe` | Win32 PE | 5,169,152 | `c1a15adfb3e9dc4be64182a96331a5691ab63ada8b93e410de1492a018ea5782` |
| **MSI Payload / Installed Executable** | `deskpilot.exe` | Win32 PE | 5,169,152 | `a517c48dbdcc1f3fadad8bce8dbff2ca073361edcbfa5e15cb4b057f0a5c0501` |

> [!NOTE]
> **Artifact Packaging & Availability Scope:**
> These binaries were compiled and verified from commit `effdb09971edf4746e0388fe714564ce477ee6a5` (run `45693205561`). The NSIS installer format (`deskpilot_0.1.1_x64-setup.exe`) was formally discontinued in Phase 16.26; the preview release is strictly MSI-only. The embedded Microsoft WebView2 bootstrapper (`MicrosoftEdgeWebview2Setup.exe`, 1,826,528 bytes, SHA-256 `fc2972da71d43e0c57b8d85a5cb9b200ed221f723cf43052d08ca722eb0bb22e`) was verified by Microsoft Authenticode signature (`CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US`, Code Signing EKU `1.3.6.1.5.5.7.3.3`) during the build workflow.

### PowerShell Verification Command
```powershell
Get-FileHash -Algorithm SHA256 .\deskpilot_0.1.1_x64_en-US.msi
# Expected output must match:
# b6e4001f44a9c05d7577e33ba0fab6ffa99b68e9e099b3a56d56d337e5dfd29f
```

### Executable Hash Provenance & Installed Payload Reconciliation

When verifying the installed binary at `C:\Program Files\deskpilot\deskpilot.exe` (or when extracting the MSI payload via administrative extraction `msiexec /a`), the resulting executable matches the MSI payload byte-for-byte and produces a distinct hash from the standalone CI binary:
- **MSI-Embedded Payload / Installed `deskpilot.exe` SHA-256:** `a517c48dbdcc1f3fadad8bce8dbff2ca073361edcbfa5e15cb4b057f0a5c0501`
- **Standalone Release Binary SHA-256:** `c1a15adfb3e9dc4be64182a96331a5691ab63ada8b93e410de1492a018ea5782`

This hash variance is an expected property of the Tauri packaging pipeline:
1. **Compilation Origin:** Both executables originate from the exact same compilation invocation.
2. **The 3-Byte Bundler Tag:** Exactly **3 bytes** differ at file offsets 4,873,967–4,873,969. The Tauri CLI bundler (`tauri-bundler`) automatically stamps a duplicate of the binary with package-type marker `__TAURI_BUNDLE_TYPE_VAR_MSI` (`4D 53 49`) prior to WiX packaging, whereas the standalone binary output in `target/release` retains the default `__TAURI_BUNDLE_TYPE_VAR_UNK` (`55 4E 4B`) marker.
3. **Machine Code Identity:** All machine instructions and assets are **100% byte-for-byte identical**, and the installed executable matches the MSI payload byte-for-byte (verified in Phase 18.4.1).

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
- The v0.1.1 preview binaries are unsigned (`NotSigned`). No Authenticode signing certificate is configured or planned for this preview release.
- This release is intended strictly for controlled developer evaluation and technical feedback in non-production environments, not for unrestricted enterprise deployment.
- Unsigned binaries lack cryptographic publisher attribution and encounter standard Windows operating system defense prompts (such as *Unknown publisher* or Microsoft Defender SmartScreen) and Application Control policies; warning-free installation is not promised.

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
> Never disable Microsoft Defender SmartScreen, Smart App Control, or Windows Application Control policies to run untrusted software. Do not instruct users to bypass enterprise security controls or blindly override Windows warnings. If your organizational or personal security policy requires signed binaries or prohibits running unsigned software, do not install this unsigned preview.

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
- On hosts where Windows Application Control or Smart App Control is enforced, unsigned Cargo build-scripts (`build.rs`) generated during local `cargo test` execution may be blocked. Full automated test suites pass 100% locally and in CI: 511 total (260 Rust + 251 frontend, headless unit/integration tests).

### 6. Installed Runtime Lifecycle Validated on Physical Windows 11 Pro 25H2
- In Phase 18.4 (commit `effdb09971edf4746e0388fe714564ce477ee6a5`), elevated major upgrade, installed binary execution, and 15/15 permitted functional acceptance checks were verified on a physical Windows 11 Pro 25H2 desktop (build `26200.9974`, x64), including 137 apps discovered, guarded startup registry states (`Disabled`, `Enabled`, `Stale`, `OtherInstall`, `Unrecognized`), preserved user settings/launch history, and a 31-minute runtime observation without crashes or unexpected resource growth (`26.24 -> 23.48 MB` working set; not claimed as formal memory-leak proof). Earlier preview validation (Phase 16.38) additionally observed an 86+ minute soak. Application JSON state under `%APPDATA%\com.deskpilot.app\` (`settings.json`, `launch-history.json`) and WebView2 profile state under `%LOCALAPPDATA%\com.deskpilot.app\EBWebView\` (outside the MSI's explicit upgrade-removal scope) remained intact across the tested upgrade. Windows 10 desktop validation (minimum build undetermined) and second-user interactive GUI login testing remain pending.

### 7. Windows 11 Sandbox Runtime Verification Scope & Protocol Boundaries (Phases 16.27 & 16.29)
- **Installer Lifecycle Validated:** In Phase 16.29, silent per-machine installation (`msiexec /i ... /qn`), public desktop shortcut placement (`%PUBLIC%\Desktop\DeskPilot.lnk`), Common Start Menu shortcut placement (`%PROGRAMDATA%\...\DeskPilot\DeskPilot.lnk`), secondary user account provisioning (`TestUser2`), and silent uninstallation (`msiexec /x`) were verified 100% pass in Windows Sandbox. Erroneous legacy paths (`C:\Desktop`, `C:\deskpilot`) were completely eliminated.
- **Kernel-Level WDAC Interception:** Windows Sandbox base images (Windows 11 Enterprise Build 26100) enforce hypervisor Code Integrity policy `{0283ac0f-fff1-49ae-ada1-8a933130cad6}` (`VerifiedAndReputableDesktop`). When attempting to launch `deskpilot.exe` inside the container, process creation was blocked with Win32 Error `4551` and Code Integrity Events `3077`, `3089`, and `3099`.
- **Silent Installation Protocol:** The MSI installer was validated using silent execution (`msiexec /i ... /qn`). Interactive graphical installer wizard dialog behavior was not tested.
- **Historical Note (Phase 13.9):** Earlier test iterations in Phase 13.9 observed interactive launch under different container policy conditions, but current Windows 11 Enterprise container environments actively enforce Code Integrity against unsigned binaries.
- **Second-User Interactive Login Scope:** Multi-user directory placement was verified with `TestUser2`, but second-user interactive GUI login and application launch were not tested due to Windows Sandbox's single-session (`WDAGUtilityAccount`) architecture.

### 8. Windows 10 Empirical Verification Pending (Untested)
- Windows 10 has **not** been tested on physical or virtual desktop hardware, and the exact minimum supported Windows 10 build is **undetermined**. While DeskPilot uses modern 64-bit Windows APIs and Evergreen WebView2, Windows 10 is not claimed as validated or supported in this preview prior to empirical desktop verification.

### 9. Release Artifact Publication Status
- The binaries, installer, and checksums documented in these release notes were generated and validated from commit `effdb09971edf4746e0388fe714564ce477ee6a5` (GitHub Actions run `45693205561`) as a temporary CI workflow artifact and have **not yet been published** as a GitHub Release. The latest published GitHub Release remains the earlier `v0.1.1-preview.2` release.

---

## 8. Development Verification Summary

- **Frontend Test Suite Execution Evidence:** **251 passing tests** across 22 Vitest suites in `src/components/__tests__`, `src/launcher/__tests__`, and `src/lib/ipc/__tests__`.
- **Backend Test Suite Execution Evidence:** **260 passing tests** in Rust (`195` unit tests in `src-tauri/src` and `65` integration tests across `src-tauri/tests`). Total automated tests passing: **511 passed** (0 failed, 100% pass rate).
- **TypeScript Compilation:** Zero compilation errors (`tsc --noEmit`).
- **Testing Boundary Distinction:** These automated test suites execute in headless environments (Node.js/JSDOM for Vitest; console test harness for Cargo). They empirically validate logical correctness, state transitions, binary parsing, and IPC contracts; they do **not** constitute native GUI surface verification, live WebView2 window rendering, or runtime acceptance by host security policies.
- **Consolidated Release Verification Matrix:**
  - **Automated Tests & CI (`effdb09971edf4746e0388fe714564ce477ee6a5`):** 511 passing headless tests (260 Rust + 251 frontend); `CI` (`45692834705`), `Installer smoke test` (`45692834602`), `CodeQL` (`45692835764`), and `Build Release Windows MSI` (`45693205561`) all passed.
  - **Installer Architecture:** Per-machine WiX MSI packaging (`deskpilot_0.1.1_x64_en-US.msi`, 4,472,832 bytes) with Microsoft Authenticode-verified embedded WebView2 bootstrapper (`build-manifest.json`), public shortcuts at `%PUBLIC%\Desktop\DeskPilot.lnk` and `%PROGRAMDATA%\Microsoft\Windows\Start Menu\Programs\DeskPilot\DeskPilot.lnk`, and byte-for-byte payload match to installed `deskpilot.exe` (Phase 18.4.1).
  - **Physical Desktop Runtime Acceptance (Phase 18.4, Windows 11 Pro 25H2 Build `26200.9974`):** 15/15 permitted functional acceptance checks passed, including elevated upgrade with preserved settings/history, Quick Launcher, app discovery (137 apps), safe launch (`About Java`), Process Monitor (253 processes), System Intelligence, `--autostart` tray launch, single-instance mutex, guarded startup registry states (`Disabled`, `Enabled`, `Stale`, `OtherInstall`, `Unrecognized`), and a 31-minute runtime observation (`26.24 -> 23.48 MB` working set, zero crashes or unexpected resource growth).
  - **Automated CI smoke test (GitHub Actions Windows Server 2022, `windows-2022`, run `45692834602`):** `installer-smoke-test.yml` passes silent install, 20-second startup check, and uninstall (automated CI smoke-test evidence only; not desktop compatibility evidence).
  - **1 documented known limitation:** legacy v0.1.0 downgrade ingress (older v0.1.0 installer lacks downgrade guards).
  - **1 blocked criterion in sandbox (Windows 11 Enterprise Build 26100 Windows Sandbox):** container runtime execution of the unsigned binary (blocked by container hypervisor WDAC policy `{0283ac0f...}` Event 3077).
  - **1 pending verification:** empirical runtime verification on Windows 10 (physical or virtual desktop hardware; minimum supported Windows 10 build is undetermined).
  - **Unsigned Preview Policy:** Binaries are intentionally distributed as an unsigned Developer Preview; Authenticode signing is not configured.
  - **Packaging format:** MSI-only (NSIS discontinued in Phase 16.26).

---

## 9. Feedback & Contributing

DeskPilot is developed transparently on GitHub. Bug reports, technical feedback, and code contributions are welcome:

- **Repository:** `https://github.com/youssefmanssouri/deskpilot-`
- **Issue Tracker:** Submit reproduction steps and system information via GitHub Issues.
- **Architecture & Security:** See `README.md` and `PRIVACY.md` for architectural invariants.

