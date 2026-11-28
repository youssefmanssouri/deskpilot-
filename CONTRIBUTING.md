# Contributing to DeskPilot

Thank you for your interest in evaluating and contributing to DeskPilot!

DeskPilot is currently in **Developer Preview** (v0.1.1). At this stage, the primary way the community can contribute is through **structured preview testing, environment qualification, and high-quality defect reports**.

---

## 1. Project Status & Supported Environments

- **Current Milestone:** `v0.1.1` Developer Preview (Revision 3, tag [`v0.1.1-preview.3`](https://github.com/youssefmanssouri/deskpilot-/releases/tag/v0.1.1-preview.3)).
- **Intended Scope:** Targets 64-bit Windows (`x86_64`). Windows on ARM64 is not built or supported, and Windows 7, 8, and 8.1 are outside the intended support scope.
- **Validated Desktop Environment:** **Windows 11 Pro 25H2** (physical desktop validation completed in Phase 16.38).
- **Installer Lifecycle Container:** **Windows 11 Enterprise Build 26100 Windows Sandbox** (Phase 16.29 MSI lifecycle tested; execution of the unsigned application was blocked by container WDAC).
- **CI Automated Smoke Test:** **GitHub Actions Windows Server 2022** (`windows-2022`, `installer-smoke-test.yml`) provides automated installer smoke-test evidence (silent install, 20-second startup check, and uninstall), not desktop Windows compatibility evidence.
- **Untested Environment:** **Windows 10 x64** has not been tested on physical or virtual desktop hardware, and the exact minimum supported Windows 10 build is undetermined. Windows 10 is not claimed as validated or supported; evaluation reports from Windows 10 systems are welcome.
- **Code Signing Status:** Release binaries and the MSI package are currently **unsigned** (`NotSigned`). Expect Windows Defender SmartScreen or application control warnings.
- **Security Policy:** Do not bypass, override, or disable security warnings or application control policies. If your system or organizational security policy blocks execution, **stop installation immediately**.

> [!IMPORTANT]
> **Security Boundary Notice:** Never bypass, override, or disable Microsoft Defender SmartScreen, Smart App Control, Windows Defender Application Control (WDAC), or antivirus protections to evaluate DeskPilot. If your corporate or system policy enforces commercial digital signatures or blocks unsigned software, **stop execution immediately**. Building from source is not a workaround to bypass signature or code-integrity enforcement: locally compiled binaries remain unsigned and are subject to the same application-control policies. Contributors and evaluators must only build and execute unsigned binaries in environments where they are explicitly permitted to do so. Otherwise, wait for an appropriately signed release or consult your system administrator.

---

## 2. Obtaining & Verifying the Official Installer

Official releases are distributed exclusively through the [DeskPilot GitHub Releases page](https://github.com/youssefmanssouri/deskpilot-/releases).

Before installing, verify the integrity of the downloaded MSI using PowerShell:

```powershell
Get-FileHash .\deskpilot_0.1.1_x64_en-US.msi -Algorithm SHA256
```

**Official Checksum for v0.1.1 Developer Preview (Revision 3, `v0.1.1-preview.3`):**
- **MSI Installer (`deskpilot_0.1.1_x64_en-US.msi`):** `b6e4001f44a9c05d7577e33ba0fab6ffa99b68e9e099b3a56d56d337e5dfd29f` (Size: `4,472,832` bytes)

If the calculated hash does not match, do not run the installer.

---

## 3. Installation Walkthrough (Windows File Explorer)

1. Double-click `deskpilot_0.1.1_x64_en-US.msi` in File Explorer.
2. **Microsoft Defender SmartScreen and Code Integrity:** Because preview binaries are unsigned (`NotSigned`), Windows Defender SmartScreen or system application control policies may present a warning:

   *"Windows protected your PC — Microsoft Defender SmartScreen prevented an unrecognized app from starting."*

   Do not bypass, override, or disable security warnings or application control policies. If Windows or your organizational security policy blocks execution, **stop installation immediately**. Do not attempt to bypass restrictions by building from source, as locally compiled binaries are also unsigned and will not be exempt from WDAC, Smart App Control, or administrative policies. If your environment requires signed software, wait for an appropriately signed release or consult your system administrator.
3. **User Account Control (UAC):** If permitted by system policy, Windows will display a UAC elevation prompt requesting administrative permission for the Windows Installer. Click **Yes** to complete the per-machine installation into `C:\Program Files\deskpilot\`.
4. Setup installs shortcuts in:
   - Public Desktop: `%PUBLIC%\Desktop\DeskPilot.lnk`
   - Common Start Menu: `%ProgramData%\Microsoft\Windows\Start Menu\Programs\DeskPilot\DeskPilot.lnk`

- **Upgrading:** Normal MSI upgrades replace the application binaries without explicitly managing, migrating, or deleting `%APPDATA%\com.deskpilot.app\settings.json`, `%APPDATA%\com.deskpilot.app\launch-history.json`, the WebView2 profile at `%LOCALAPPDATA%\com.deskpilot.app\EBWebView\`, or the optional startup entry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\DeskPilot`). Schema migration across arbitrary future versions is not guaranteed.
- **Uninstalling:** Before uninstalling, disable **Start with Windows** in DeskPilot Settings so the optional `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\DeskPilot` entry (created only when enabled) is removed cleanly; otherwise it can remain pointing to the removed executable. Then go to **Windows Settings → Apps → Installed apps**, select **DeskPilot**, and click **Uninstall**. Uninstallation removes the binaries (`C:\Program Files\deskpilot`), shortcuts, and the *Installed apps* entry, but does **not** automatically delete `%APPDATA%\com.deskpilot.app\settings.json`, `%APPDATA%\com.deskpilot.app\launch-history.json`, or `%LOCALAPPDATA%\com.deskpilot.app\EBWebView\` (remove those `%APPDATA%` and `%LOCALAPPDATA%` directories manually if you want full profile cleanup).
- **Reinstalling:** Data left outside the MSI removal scope may remain available after reinstalling, though recovery of corrupted or incompatible JSON data is not guaranteed.

---

## 4. Testing Core Features

We encourage community testers to exercise the following core capabilities:

1. **Application Discovery & Launching:**
   - Navigate to the **Apps** tab. Verify that installed Win32 programs appear with native icons extracted from their executables.
   - Search for an application (`Ctrl+K`), select it with arrow keys, and press `Enter` to launch.
   - Verify that the launched program appears in the **Home** view under **Recent apps**.
2. **Global Quick Launcher (`Ctrl+Alt+Space`):**
   - Press `Ctrl+Alt+Space` from any active window.
   - Verify that the palette appears centered and automatically focused.
   - Test typing to search and pressing `Esc` to dismiss.
3. **Process Monitor (Strictly Read-Only):**
   - Navigate to the **Processes** tab. Verify that process list loads and search filtering works smoothly.
   - *Note:* DeskPilot is strictly a monitor; process termination is intentionally not implemented.
4. **System Intelligence:**
   - Navigate to the **System** tab. Compare CPU utilization, physical RAM, and fixed disk volume readings against Windows Task Manager and report any discrepancies.
5. **System Tray Integration:**
   - Closing the main window (`X`) hides DeskPilot to the notification tray area.
   - Right-click the system tray icon to access *Show DeskPilot*, *Hide DeskPilot*, or *Quit DeskPilot*.

---

## 5. Submitting Feedback & Bug Reports

All feedback is tracked via the [GitHub Issue Tracker](https://github.com/youssefmanssouri/deskpilot-/issues).

### Submitting a Bug Report
When opening a bug report using our [Bug Report Template](.github/ISSUE_TEMPLATE/bug_report.md), please include:
- Exact Windows edition, version, and build number (run `winver` in Windows).
- System hardware details (CPU architecture, RAM, multi-monitor configuration).
- Clear, step-by-step reproduction instructions.
- Expected versus actual behavior.
- Relevant error text, dialog messages, or application banners.

### What NOT to Submit (Sensitive Information)
To protect your privacy and security, **never submit**:
- Passwords, credentials, or private keys.
- Personal access tokens, API secrets, or session cookies.
- Personal document paths, customer data, or confidential business files.
- Unsanitized logs containing machine network identities or proprietary directory trees.

### Submitting Feature Proposals
Feature requests may be submitted using our [Feature Request Template](.github/ISSUE_TEMPLATE/feature_request.md). Please explain the underlying use case, anticipated workflow benefits, and any privacy/permission implications. Submitting a proposal or pull request does not imply automatic acceptance or merging.

---

## 6. Development & Source Contributions

To contribute code or build DeskPilot locally from source:
1. Review the [Architecture Documentation](docs/ARCHITECTURE.md).
2. Follow the [Building from Source instructions](README.md#building-from-source).
3. Ensure all automated tests pass before opening a Pull Request:
   ```bash
   npm test
   cd src-tauri && cargo test --locked
   ```
4. Adhere to the core architectural principles defined in [SECURITY.md](SECURITY.md) and [PRIVACY.md](PRIVACY.md).

> [!NOTE]
> Contributors must only build and execute DeskPilot where their local or organizational environment explicitly permits running unsigned binaries. Locally built binaries are unsigned and are not automatically trusted or exempt from Smart App Control, WDAC, or organizational security policies.
