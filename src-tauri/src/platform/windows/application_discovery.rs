use crate::models::application::DiscoverySource;
use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

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
    /// Scans Windows Uninstall registry hives for installed application metadata.
    pub fn scan_registry_hive(
        root: winreg::enums::HKEY,
        subkey_path: &str,
        source: DiscoverySource,
    ) -> Result<Vec<RawDiscoveredEntry>, String> {
        use winreg::RegKey;
        use winreg::enums::*;

        let root_key = RegKey::predef(root);
        let uninstall_key = root_key
            .open_subkey_with_flags(subkey_path, KEY_READ)
            .map_err(|e| format!("Inaccessible registry key {}: {}", subkey_path, e))?;

        let mut entries = Vec::new();

        // Enumerate all application subkeys
        for key_name in uninstall_key.enum_keys().filter_map(|r| r.ok()) {
            if let Ok(app_key) = uninstall_key.open_subkey_with_flags(&key_name, KEY_READ) {
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

                // Read DisplayName - if missing or empty, skip entry
                let display_name: Option<String> = app_key
                    .get_value("DisplayName")
                    .ok()
                    .filter(|s: &String| !s.trim().is_empty());

                if display_name.is_none() {
                    continue;
                }

                let publisher: Option<String> = app_key
                    .get_value("Publisher")
                    .ok()
                    .filter(|s: &String| !s.trim().is_empty());

                let version: Option<String> = app_key
                    .get_value("DisplayVersion")
                    .ok()
                    .filter(|s: &String| !s.trim().is_empty());

                let install_location: Option<String> = app_key
                    .get_value("InstallLocation")
                    .ok()
                    .filter(|s: &String| !s.trim().is_empty());

                let display_icon: Option<String> = app_key
                    .get_value("DisplayIcon")
                    .ok()
                    .filter(|s: &String| !s.trim().is_empty());

                // Derive potential target path from DisplayIcon or InstallLocation without executing
                let mut target_path = None;
                if let Some(ref icon) = display_icon {
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
            return Err(format!(
                "Start Menu directory does not exist: {:?}",
                dir_path
            ));
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
        let mut file = File::open(path).ok()?;
        let mut buffer = Vec::new();
        // Read at most 4KB to protect against huge or corrupted files
        file.take(4096).read_to_end(&mut buffer).ok()?;

        if buffer.len() < 76 {
            return None;
        }

        // HeaderSize must be 0x4C (76)
        let header_size = u32::from_le_bytes(buffer[0..4].try_into().ok()?);
        if header_size != 76 {
            return None;
        }

        // LinkFlags at byte offset 20
        let flags = u32::from_le_bytes(buffer[20..24].try_into().ok()?);
        let has_target_id_list = (flags & 0x01) != 0;
        let has_link_info = (flags & 0x02) != 0;

        if !has_link_info {
            return None;
        }

        let mut offset = 76;

        if has_target_id_list {
            if buffer.len() < offset + 2 {
                return None;
            }
            let id_list_size =
                u16::from_le_bytes(buffer[offset..offset + 2].try_into().ok()?) as usize;
            offset += 2 + id_list_size;
        }

        if buffer.len() < offset + 28 {
            return None;
        }

        // LinkInfo structure
        let link_info_size =
            u32::from_le_bytes(buffer[offset..offset + 4].try_into().ok()?) as usize;
        let link_info_header_size =
            u32::from_le_bytes(buffer[offset + 4..offset + 8].try_into().ok()?) as usize;
        let link_info_flags = u32::from_le_bytes(buffer[offset + 8..offset + 12].try_into().ok()?);

        // Bit 0 = VolumeIDAndLocalBasePath
        if (link_info_flags & 0x01) != 0 && link_info_header_size >= 28 {
            let local_base_path_offset =
                u32::from_le_bytes(buffer[offset + 16..offset + 20].try_into().ok()?) as usize;

            let path_start = offset + local_base_path_offset;
            if path_start < buffer.len() && path_start < offset + link_info_size {
                let mut path_bytes = Vec::new();
                for &b in &buffer[path_start..] {
                    if b == 0 {
                        break;
                    }
                    path_bytes.push(b);
                }

                if let Ok(path_str) = String::from_utf8(path_bytes) {
                    let clean = path_str.trim().to_string();
                    if !clean.is_empty() && PathBuf::from(&clean).exists() {
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
}
