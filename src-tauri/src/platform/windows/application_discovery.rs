use crate::models::application::DiscoverySource;
use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Expected CLSID for Windows Shell Links (.lnk): 00021401-0000-0000-C000-000000000046
const SHELL_LINK_CLSID: [u8; 16] = [
    0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];

/// Raw discovered entry extracted directly from Windows registry or Start Menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDiscoveredEntry {
    pub raw_display_name: Option<String>,
    pub raw_publisher: Option<String>,
    pub raw_version: Option<String>,
    pub raw_install_location: Option<String>,
    pub raw_display_icon: Option<String>,
    pub raw_target_path: Option<String>,
    pub source: DiscoverySource,
}

/// Windows native application discovery implementation.
pub struct WindowsApplicationScanner;

impl WindowsApplicationScanner {
    /// Sanitizes registry strings by stripping null bytes, truncating to 2048 characters, and trimming whitespace.
    pub fn sanitize_registry_string(raw: String) -> Option<String> {
        let stripped: String = raw.chars().filter(|&c| c != '\0').take(2048).collect();
        let trimmed = stripped.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn format_registry_error(err: &std::io::Error) -> String {
        if err.kind() == std::io::ErrorKind::PermissionDenied {
            "Access is denied.".to_string()
        } else if err.kind() == std::io::ErrorKind::NotFound {
            "Registry key not found.".to_string()
        } else {
            "Registry key inaccessible.".to_string()
        }
    }

    /// Scans Windows Uninstall registry hives for installed application metadata.
    ///
    /// Applies view flags (`KEY_WOW64_64KEY` or `KEY_WOW64_32KEY`), limits enumeration to 5000 keys,
    /// and sanitizes all incoming strings.
    pub fn scan_registry_hive(
        root: winreg::HKEY,
        subkey_path: &str,
        access_flags: u32,
        source: DiscoverySource,
    ) -> Result<Vec<RawDiscoveredEntry>, String> {
        use winreg::RegKey;

        let root_key = RegKey::predef(root);
        let uninstall_key = root_key
            .open_subkey_with_flags(subkey_path, access_flags)
            .map_err(|e| Self::format_registry_error(&e))?;

        let mut entries = Vec::new();

        // Enumerate application subkeys, bounded to 5000 to prevent DoS
        for key_name in uninstall_key.enum_keys().filter_map(|r| r.ok()).take(5000) {
            if let Ok(app_key) = uninstall_key.open_subkey_with_flags(&key_name, access_flags) {
                // Ignore system components and internal updates
                if let Ok(sys_comp) = app_key.get_value::<u32, _>("SystemComponent") {
                    if sys_comp == 1 {
                        continue;
                    }
                }
                if let Ok(parent_key) = app_key.get_value::<String, _>("ParentKeyName") {
                    if !parent_key.trim().is_empty() {
                        continue;
                    }
                }

                // Read and sanitize DisplayName - if missing or empty, skip entry
                let display_name: Option<String> = app_key
                    .get_value::<String, _>("DisplayName")
                    .ok()
                    .and_then(Self::sanitize_registry_string);

                if display_name.is_none() {
                    continue;
                }

                let publisher: Option<String> = app_key
                    .get_value::<String, _>("Publisher")
                    .ok()
                    .and_then(Self::sanitize_registry_string);

                let version: Option<String> = app_key
                    .get_value::<String, _>("DisplayVersion")
                    .ok()
                    .and_then(Self::sanitize_registry_string);

                let install_location: Option<String> = app_key
                    .get_value::<String, _>("InstallLocation")
                    .ok()
                    .and_then(Self::sanitize_registry_string);

                let display_icon: Option<String> = app_key
                    .get_value::<String, _>("DisplayIcon")
                    .ok()
                    .and_then(Self::sanitize_registry_string);

                // Derive potential target path from DisplayIcon without executing
                let mut target_path = None;
                if let Some(icon) = &display_icon {
                    let clean_icon = icon
                        .split(',')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_matches('"');
                    if clean_icon.to_lowercase().ends_with(".exe") && Path::new(clean_icon).exists()
                    {
                        target_path = Some(clean_icon.to_string());
                    }
                }

                entries.push(RawDiscoveredEntry {
                    raw_display_name: display_name,
                    raw_publisher: publisher,
                    raw_version: version,
                    raw_install_location: install_location,
                    raw_display_icon: display_icon,
                    raw_target_path: target_path,
                    source: source.clone(),
                });
            }
        }

        Ok(entries)
    }

    /// Scans a Windows Start Menu directory for `.lnk` shortcuts.
    pub fn scan_start_menu_directory(
        dir_path: &Path,
        source: DiscoverySource,
        max_depth: usize,
    ) -> Result<Vec<RawDiscoveredEntry>, String> {
        if !dir_path.exists() {
            return Err("Directory does not exist or is inaccessible.".to_string());
        }

        let mut entries = Vec::new();
        Self::traverse_start_menu(dir_path, dir_path, 0, max_depth, &source, &mut entries);
        Ok(entries)
    }

    fn traverse_start_menu(
        _root: &Path,
        current: &Path,
        current_depth: usize,
        max_depth: usize,
        source: &DiscoverySource,
        entries: &mut Vec<RawDiscoveredEntry>,
    ) {
        if current_depth > max_depth {
            return;
        }

        let read_dir = match std::fs::read_dir(current) {
            Ok(rd) => rd,
            Err(_) => return, // Inaccessible folder - skip without crashing
        };

        for entry in read_dir.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                Self::traverse_start_menu(
                    _root,
                    &path,
                    current_depth + 1,
                    max_depth,
                    source,
                    entries,
                );
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("lnk") {
                        if let Some(discovered) = Self::inspect_shortcut(&path, source) {
                            entries.push(discovered);
                        }
                    }
                }
            }
        }
    }

    /// Safely inspects a Windows `.lnk` file metadata without executing it.
    pub fn inspect_shortcut(path: &Path, source: &DiscoverySource) -> Option<RawDiscoveredEntry> {
        let file_stem = path.file_stem()?.to_str()?.trim();

        // Filter out uninstaller shortcuts, help documentation, and web links
        let stem_lower = file_stem.to_lowercase();
        if stem_lower.starts_with("uninstall")
            || stem_lower.contains("help")
            || stem_lower.contains("documentation")
            || stem_lower.contains("readme")
        {
            return None;
        }

        let target_path = Self::parse_lnk_target(path);

        Some(RawDiscoveredEntry {
            raw_display_name: Some(file_stem.to_string()),
            raw_publisher: None,
            raw_version: None,
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: target_path,
            source: source.clone(),
        })
    }

    /// Safe binary parser for Windows Shell Link (MS-SHLLINK) to read local base target path.
    ///
    /// Never executes the link or loads COM libraries.
    pub fn parse_lnk_target(path: &Path) -> Option<String> {
        let file = File::open(path).ok()?;
        let mut buffer = Vec::new();
        // Bounded read up to 64KB protects memory against corrupted or malicious files
        file.take(65536).read_to_end(&mut buffer).ok()?;

        Self::parse_lnk_target_from_bytes(&buffer, true)
    }

    /// Pure parser for Shell Link binary data.
    ///
    /// Implements strict MS-SHLLINK validation, checked arithmetic for offsets,
    /// and supports both UTF-16LE Unicode paths and ANSI paths.
    pub fn parse_lnk_target_from_bytes(buffer: &[u8], verify_existence: bool) -> Option<String> {
        // MS-SHLLINK header is exactly 76 bytes
        if buffer.len() < 76 {
            return None;
        }

        // HeaderSize must be 0x0000004C (76)
        let header_size = u32::from_le_bytes(buffer.get(0..4)?.try_into().ok()?);
        if header_size != 76 {
            return None;
        }

        // Validate LinkCLSID: 00021401-0000-0000-C000-000000000046
        if buffer.get(4..20)? != SHELL_LINK_CLSID {
            return None;
        }

        // LinkFlags at byte offset 20
        let flags = u32::from_le_bytes(buffer.get(20..24)?.try_into().ok()?);
        let has_target_id_list = (flags & 0x01) != 0;
        let has_link_info = (flags & 0x02) != 0;

        if !has_link_info {
            return None;
        }

        let mut offset = 76usize;

        // Skip LinkTargetIDList if present
        if has_target_id_list {
            let next_offset = offset.checked_add(2)?;
            if buffer.len() < next_offset {
                return None;
            }
            let id_list_size =
                u16::from_le_bytes(buffer.get(offset..next_offset)?.try_into().ok()?) as usize;
            offset = next_offset.checked_add(id_list_size)?;
        }

        // Minimum LinkInfo size is 28 bytes
        let link_info_min_header = offset.checked_add(28)?;
        if buffer.len() < link_info_min_header {
            return None;
        }

        // LinkInfo structure
        let link_info_size =
            u32::from_le_bytes(buffer.get(offset..offset + 4)?.try_into().ok()?) as usize;
        let link_info_header_size =
            u32::from_le_bytes(buffer.get(offset + 4..offset + 8)?.try_into().ok()?) as usize;
        let link_info_flags =
            u32::from_le_bytes(buffer.get(offset + 8..offset + 12)?.try_into().ok()?);

        if link_info_size < 28 || link_info_header_size < 28 {
            return None;
        }

        let link_info_end = offset.checked_add(link_info_size)?.min(buffer.len());

        // Bit 0 = VolumeIDAndLocalBasePath
        if (link_info_flags & 0x01) == 0 {
            return None;
        }

        // 1. Try reading LocalBasePathOffsetUnicode if header size >= 36 (0x24)
        if link_info_header_size >= 36 {
            if let Some(unicode_offset_slice) = buffer.get(offset + 28..offset + 32) {
                let unicode_offset =
                    u32::from_le_bytes(unicode_offset_slice.try_into().ok()?) as usize;
                if unicode_offset > 0 {
                    if let Some(unicode_start) = offset.checked_add(unicode_offset) {
                        if unicode_start < link_info_end {
                            let mut u16_chars = Vec::new();
                            let mut cur = unicode_start;
                            while let Some(next_cur) = cur.checked_add(2) {
                                if next_cur > link_info_end {
                                    break;
                                }
                                if let Some(pair) = buffer.get(cur..next_cur) {
                                    let code_unit = u16::from_le_bytes([pair[0], pair[1]]);
                                    if code_unit == 0 {
                                        break;
                                    }
                                    u16_chars.push(code_unit);
                                    cur = next_cur;
                                } else {
                                    break;
                                }
                            }

                            if !u16_chars.is_empty() {
                                if let Ok(unicode_path) = String::from_utf16(&u16_chars) {
                                    let clean = unicode_path.trim().to_string();
                                    if !clean.is_empty() {
                                        if !verify_existence || PathBuf::from(&clean).exists() {
                                            return Some(clean);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Fall back to ANSI LocalBasePathOffset at offset + 16
        let local_base_path_offset =
            u32::from_le_bytes(buffer.get(offset + 16..offset + 20)?.try_into().ok()?) as usize;

        let path_start = offset.checked_add(local_base_path_offset)?;
        if path_start < link_info_end {
            let mut path_bytes = Vec::new();
            if let Some(slice) = buffer.get(path_start..link_info_end) {
                for &b in slice {
                    if b == 0 {
                        break;
                    }
                    path_bytes.push(b);
                }
            }

            if !path_bytes.is_empty() {
                let path_str = String::from_utf8_lossy(&path_bytes).into_owned();
                let clean = path_str.trim().to_string();
                if !clean.is_empty() {
                    if !verify_existence || PathBuf::from(&clean).exists() {
                        return Some(clean);
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_shortcut_filters_uninstallers() {
        let fake_uninstall = Path::new("C:\\ProgramData\\Start\\Uninstall App.lnk");
        let result = WindowsApplicationScanner::inspect_shortcut(
            fake_uninstall,
            &DiscoverySource::StartMenuCommon,
        );
        assert!(result.is_none());

        let fake_readme = Path::new("C:\\ProgramData\\Start\\App Readme.lnk");
        let result = WindowsApplicationScanner::inspect_shortcut(
            fake_readme,
            &DiscoverySource::StartMenuCommon,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_inspect_shortcut_valid_app() {
        let fake_app = Path::new("C:\\ProgramData\\Start\\Visual Studio Code.lnk");
        let result = WindowsApplicationScanner::inspect_shortcut(
            fake_app,
            &DiscoverySource::StartMenuCommon,
        );
        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(
            entry.raw_display_name,
            Some("Visual Studio Code".to_string())
        );
        assert_eq!(entry.source, DiscoverySource::StartMenuCommon);
    }

    #[test]
    fn test_sanitize_registry_string() {
        let dirty = "Google Chrome\0with hidden payload\0".to_string();
        let cleaned = WindowsApplicationScanner::sanitize_registry_string(dirty);
        assert_eq!(
            cleaned,
            Some("Google Chromewith hidden payload".to_string())
        );

        let whitespace = "   \0 \t  ".to_string();
        assert_eq!(
            WindowsApplicationScanner::sanitize_registry_string(whitespace),
            None
        );

        let huge = "A".repeat(3000);
        let cleaned_huge = WindowsApplicationScanner::sanitize_registry_string(huge).unwrap();
        assert_eq!(cleaned_huge.len(), 2048);
    }

    #[test]
    fn test_parse_lnk_target_from_bytes_ansi() {
        // Construct a valid MS-SHLLINK byte buffer with ANSI path
        let mut buf = vec![0u8; 256];
        // HeaderSize = 76
        buf[0..4].copy_from_slice(&76u32.to_le_bytes());
        // CLSID
        buf[4..20].copy_from_slice(&SHELL_LINK_CLSID);
        // LinkFlags: HasLinkInfo (0x02)
        buf[20..24].copy_from_slice(&2u32.to_le_bytes());

        // LinkInfo at offset 76:
        // Size = 80 (plenty of room for 28-byte header + target string)
        buf[76..80].copy_from_slice(&80u32.to_le_bytes());
        // HeaderSize = 28
        buf[80..84].copy_from_slice(&28u32.to_le_bytes());
        // Flags = 1 (VolumeIDAndLocalBasePath)
        buf[84..88].copy_from_slice(&1u32.to_le_bytes());
        // LocalBasePathOffset = 28 (relative to offset 76, so target path is at 76 + 28 = 104)
        buf[92..96].copy_from_slice(&28u32.to_le_bytes());

        let target = b"C:\\Program Files\\App\\app.exe\0";
        buf[104..104 + target.len()].copy_from_slice(target);

        let parsed = WindowsApplicationScanner::parse_lnk_target_from_bytes(&buf, false);
        assert_eq!(parsed, Some("C:\\Program Files\\App\\app.exe".to_string()));
    }

    #[test]
    fn test_parse_lnk_target_from_bytes_unicode() {
        // Construct a valid MS-SHLLINK byte buffer with Unicode (UTF-16LE) path
        let mut buf = vec![0u8; 256];
        // HeaderSize = 76
        buf[0..4].copy_from_slice(&76u32.to_le_bytes());
        // CLSID
        buf[4..20].copy_from_slice(&SHELL_LINK_CLSID);
        // LinkFlags: HasLinkInfo (0x02)
        buf[20..24].copy_from_slice(&2u32.to_le_bytes());

        // LinkInfo at offset 76:
        // Size = 150
        buf[76..80].copy_from_slice(&150u32.to_le_bytes());
        // HeaderSize = 36 (indicates Unicode offsets present)
        buf[80..84].copy_from_slice(&36u32.to_le_bytes());
        // Flags = 1 (VolumeIDAndLocalBasePath)
        buf[84..88].copy_from_slice(&1u32.to_le_bytes());
        // LocalBasePathOffset (ANSI dummy) = 36
        buf[92..96].copy_from_slice(&36u32.to_le_bytes());
        // LocalBasePathOffsetUnicode = 40 (relative to 76, so at 116)
        buf[104..108].copy_from_slice(&40u32.to_le_bytes());

        let utf16_str: Vec<u16> = "C:\\Programme\\App\\ÜberTool.exe\0"
            .encode_utf16()
            .collect();
        let mut pos = 116;
        for u in utf16_str {
            buf[pos..pos + 2].copy_from_slice(&u.to_le_bytes());
            pos += 2;
        }

        let parsed = WindowsApplicationScanner::parse_lnk_target_from_bytes(&buf, false);
        assert_eq!(parsed, Some("C:\\Programme\\App\\ÜberTool.exe".to_string()));
    }

    #[test]
    fn test_parse_lnk_rejects_corrupted_headers() {
        // Truncated buffer (< 76 bytes)
        let short_buf = vec![0u8; 50];
        assert_eq!(
            WindowsApplicationScanner::parse_lnk_target_from_bytes(&short_buf, false),
            None
        );

        // HeaderSize not 76
        let mut bad_size_buf = vec![0u8; 100];
        bad_size_buf[0..4].copy_from_slice(&75u32.to_le_bytes());
        assert_eq!(
            WindowsApplicationScanner::parse_lnk_target_from_bytes(&bad_size_buf, false),
            None
        );

        // Invalid CLSID
        let mut bad_clsid_buf = vec![0u8; 100];
        bad_clsid_buf[0..4].copy_from_slice(&76u32.to_le_bytes());
        bad_clsid_buf[4..20].copy_from_slice(&[0xFF; 16]);
        assert_eq!(
            WindowsApplicationScanner::parse_lnk_target_from_bytes(&bad_clsid_buf, false),
            None
        );

        // Missing HasLinkInfo flag
        let mut no_info_buf = vec![0u8; 100];
        no_info_buf[0..4].copy_from_slice(&76u32.to_le_bytes());
        no_info_buf[4..20].copy_from_slice(&SHELL_LINK_CLSID);
        no_info_buf[20..24].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(
            WindowsApplicationScanner::parse_lnk_target_from_bytes(&no_info_buf, false),
            None
        );
    }
}
