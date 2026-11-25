---
name: Bug Report
about: Report a reproducible defect or issue in DeskPilot
title: "[BUG] "
labels: ["bug", "preview-v0.1.1"]
assignees: ''
---

> [!WARNING]
> **Privacy Notice:** Do NOT include sensitive personal information, passwords, API tokens, access keys, or confidential files in this report or in attached logs. All GitHub issues are public.

### Descriptive Title
<!-- Provide a clear, short summary of the issue in the title above -->

### Environment Information
- **DeskPilot Version:** v0.1.1 Developer Preview (Revision 2, tag `v0.1.1-preview.2`)
- **Windows Edition & Version:** <!-- e.g., Windows 11 Pro 25H2 -->
- **Windows OS Build:** <!-- Run 'winver' or '[System.Environment]::OSVersion' to find build number, e.g., 26100.1742 -->
- **System Architecture:** <!-- e.g., x64 / AMD64 -->
- **WebView2 Runtime Version:** <!-- Run 'Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"' or check Edge settings -->
- **Installation Method:**
  - [ ] Standard GUI setup (`deskpilot_0.1.1_x64_en-US.msi` double-click in File Explorer)
  - [ ] Command-line silent install (`msiexec /i ... /qn`)
  - [ ] Built from source (`cargo tauri build`)

### Steps to Reproduce
1. Start DeskPilot from Start Menu or Desktop.
2. Go to '...'
3. Click or press '...'
4. Observe behavior.

### Expected Behavior
<!-- A concise description of what you expected to happen -->

### Actual Behavior
<!-- A concise description of what actually happened -->

### Reproducibility
- [ ] Happens every time (100%)
- [ ] Happens intermittently (~50%)
- [ ] Happened once

### Relevant Error Messages / UI Banners
<!-- If DeskPilot displayed an error dialog or banner (e.g. elevation warning, launch error), quote the exact text here -->

### Diagnostic Logs & Screenshots (Sanitized)
<!--
If an installer issue occurred, paste sanitized excerpts from the MSI verbose log (`msiexec /i ... /l*v install.log`).
If an application crash occurred, paste Windows Event Viewer details (Application Log, Event ID 1000/1001).
Ensure all usernames or personal directories are redacted.
-->
```text

```
