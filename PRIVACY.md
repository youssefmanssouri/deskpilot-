# DeskPilot Privacy Statement

DeskPilot is engineered as a local-first Windows command center and desktop utility. This document outlines the architectural data-handling boundaries of DeskPilot, distinguishing application-managed operations from underlying platform runtime, installer, and operating system behaviors.

> **Key Operating Principle:**  
> "DeskPilot's core functionality does not require an active internet connection after installation. Windows, WebView2, the installer, and applications launched by the user may independently perform network activity."

---

## 1. DeskPilot Application Core

### In-Memory Application State
- In the audited application codebase (including application discovery, search indexing, launcher execution, process management, system monitoring, and system tray lifecycle), all application runtime state (indexed application catalog entries, active process cache, and system metrics) is held exclusively in volatile host memory (`AppState` guarded by standard concurrency primitives).
- Source inspection of the application codebase confirms that these application subsystems implement no database persistence (such as SQLite) and no disk-based log files. The only files DeskPilot writes are the local launch history and the settings file described below.
- Session indices and in-memory caches are discarded when the application process terminates.

### Local Launch History ("Recent apps")
- After an application launches successfully, DeskPilot records it so the Home view can show recently opened apps.
- Each entry contains only the application's catalog identifier (`app_` followed by 32 hexadecimal characters), its display name, and the launch time. No executable paths, arguments, working directories, or usernames are stored.
- At most 10 entries are kept, newest first, one per application.
- The history is stored in a single file, `launch-history.json`, in DeskPilot's application data folder (on Windows: `%APPDATA%\com.deskpilot.app\`). It never leaves the machine.
- The **Clear** button in the Recent apps section deletes all entries from memory and from the file. Deleting the file manually has the same effect; a missing or damaged file is treated as an empty history.
- History can be turned off in **Settings → Privacy → Remember recent apps**. Turning it off stops recording and deletes the existing list.

### Settings File
- Preferences chosen in **Settings** (theme, glass effect, quick launcher shortcut, and whether recent apps are remembered) are stored in `settings.json` in the same application data folder. It contains no personal data, paths, or usernames.
- A missing or damaged settings file is treated as the default settings. Deleting the file resets DeskPilot's preferences.

### Start with Windows (optional, off by default)
- When **Settings → Startup → Start with Windows** is turned on, DeskPilot adds one value named `DeskPilot` under the current user's startup key (`HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`). It contains the quoted path to DeskPilot plus `--autostart`, so Windows starts it, in the tray, when you sign in. Turning the setting off removes that value.
- This is the only change DeskPilot makes to the Windows registry. It affects the current user only and requires no administrator rights.

### Network Behavior & Telemetry
- The audited application code contains no implemented outbound HTTP or WebSocket client, analytics, telemetry, crash reporting, or automatic update mechanism.
- All communication between the React presentation layer and the Rust backend is strictly confined to local Tauri Inter-Process Communication (IPC) pipes within the local machine.
- All presentation assets (HTML, JavaScript bundles, CSS stylesheets, and icons) are packaged directly into the application binary and served locally without remote resource fetches.

---

## 2. Platform Runtime & Operating System Boundaries

While the audited application code contains no implemented outbound network clients, analytics, or persistent log files, the software operates atop host platforms with independent behaviors:

### Microsoft Edge WebView2 Runtime
- DeskPilot utilizes the Microsoft Edge WebView2 control to render its graphical interface.
- WebView2 maintains an isolated user data directory and cache on disk at:  
  `%LOCALAPPDATA%\com.deskpilot.app\EBWebView\`
- As a Microsoft platform component, the WebView2 runtime may independently communicate with Microsoft platform services according to standard Microsoft Edge and Windows diagnostic policies. DeskPilot neither controls nor intercepts these runtime mechanisms.

### Windows Operating System Footprint
- **Installation Files:** Standard installation via the DeskPilot MSI package places application binaries and assets into `%ProgramFiles%\deskpilot` (per-machine installation requiring elevation) and registers standard Windows Add/Remove Programs registry keys.
- **System Diagnostics:** Operating system security features (including SmartScreen, Smart App Control, and Windows Defender) and Windows Error Reporting operate independently at the OS level under Microsoft's system policies.

### Windows Installer (MSI) & WebView2 Bootstrapper
- The DeskPilot installation package embeds the official Microsoft WebView2 bootstrapper.
- If the Microsoft Edge WebView2 Evergreen Runtime is already installed on the host machine, the installer proceeds locally without requiring external downloads.
- If the WebView2 runtime is absent, the bootstrapper requires an active internet connection to download and install the required Microsoft runtime components. DeskPilot's installer should therefore not be assumed to be unconditionally offline.

---

## 3. Third-Party Applications

- DeskPilot functions as an application launcher and process manager.
- Applications launched by the user through DeskPilot execute in independent process trees and retain their own independent network connectivity, data storage, and telemetry behaviors.
- DeskPilot does not inspect, proxy, filter, or control the network traffic or local file storage of third-party applications launched by the user.

---

## 4. Scope and Limitations

DeskPilot does not guarantee absolute privacy against operating-system-level monitoring, kernel-level logging, host administrative inspection, or compromised execution environments. Users concerned with platform-level data transmission should review Windows privacy settings and Microsoft Edge WebView2 policies.
