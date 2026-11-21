//! Reads an executable's icon as raw RGBA pixels, without running the executable.
//!
//! Uses the shell's icon extraction (the same path Explorer uses), which loads the file's
//! icon resources as data. The caller is responsible for deciding which files may be read
//! (see `core::app_icons::resolve_icon_source`).

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDIBits, GetObjectW, HBITMAP, HDC,
};
use windows_sys::Win32::UI::Shell::SHDefExtractIconW;
use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

/// Extracts the first icon of `path` at `size` x `size` pixels and returns it as straight
/// (non-premultiplied) RGBA, row by row from the top. `None` when the file has no usable icon.
pub fn extract_icon_rgba(path: &Path, size: u32) -> Option<Vec<u8>> {
    if size == 0 || size > 256 {
        return None;
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();

    let mut icon: HICON = std::ptr::null_mut();
    // SAFETY: `wide` is NUL-terminated; `icon` is a valid out-pointer; no small icon requested.
    // The low word of the last argument is the large icon size.
    let hr =
        unsafe { SHDefExtractIconW(wide.as_ptr(), 0, 0, &mut icon, std::ptr::null_mut(), size) };
    if hr != 0 || icon.is_null() {
        return None;
    }
    let pixels = icon_to_rgba(icon, size);
    // SAFETY: `icon` was created by SHDefExtractIconW above and is destroyed exactly once.
    unsafe { DestroyIcon(icon) };
    pixels
}

/// Owns the GDI objects of an ICONINFO and frees them on drop.
struct IconBitmaps {
    color: HBITMAP,
    mask: HBITMAP,
}

impl Drop for IconBitmaps {
    fn drop(&mut self) {
        // SAFETY: both handles come from GetIconInfo, which transfers ownership to the caller.
        unsafe {
            if !self.color.is_null() {
                DeleteObject(self.color);
            }
            if !self.mask.is_null() {
                DeleteObject(self.mask);
            }
        }
    }
}

/// Owns a memory device context and deletes it on drop.
struct MemoryDc(HDC);

impl Drop for MemoryDc {
    fn drop(&mut self) {
        // SAFETY: created by CreateCompatibleDC and deleted exactly once.
        unsafe { DeleteDC(self.0) };
    }
}

fn icon_to_rgba(icon: HICON, size: u32) -> Option<Vec<u8>> {
    let mut info = ICONINFO {
        fIcon: 0,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: std::ptr::null_mut(),
        hbmColor: std::ptr::null_mut(),
    };
    // SAFETY: `icon` is a valid icon handle; `info` is a valid out-pointer.
    if unsafe { GetIconInfo(icon, &mut info) } == 0 {
        return None;
    }
    let bitmaps = IconBitmaps {
        color: info.hbmColor,
        mask: info.hbmMask,
    };
    // Monochrome icons have no colour bitmap; they are rare and not worth supporting.
    if bitmaps.color.is_null() {
        return None;
    }
    if bitmap_size(bitmaps.color)? != (size, size) {
        return None;
    }

    // SAFETY: a null HDC asks for a memory DC compatible with the screen.
    let dc = MemoryDc(unsafe { CreateCompatibleDC(std::ptr::null_mut()) });
    if dc.0.is_null() {
        return None;
    }

    let mut bgra = read_bitmap_32bpp(dc.0, bitmaps.color, size)?;

    // Older icons carry no alpha channel; their transparency is in the mask bitmap, where
    // white means transparent. Read it at 32 bpp so each pixel is black or white.
    if bgra.chunks_exact(4).all(|px| px[3] == 0) {
        let mask = if bitmaps.mask.is_null() {
            None
        } else {
            read_bitmap_32bpp(dc.0, bitmaps.mask, size)
        };
        for (i, px) in bgra.chunks_exact_mut(4).enumerate() {
            let transparent = mask.as_ref().is_some_and(|m| m[i * 4] != 0);
            px[3] = if transparent { 0 } else { 255 };
        }
    }

    // BGRA -> RGBA
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    Some(bgra)
}

fn bitmap_size(bitmap: HBITMAP) -> Option<(u32, u32)> {
    let mut bm = BITMAP {
        bmType: 0,
        bmWidth: 0,
        bmHeight: 0,
        bmWidthBytes: 0,
        bmPlanes: 0,
        bmBitsPixel: 0,
        bmBits: std::ptr::null_mut(),
    };
    // SAFETY: `bm` is a BITMAP and its exact size is passed.
    let written = unsafe {
        GetObjectW(
            bitmap,
            std::mem::size_of::<BITMAP>() as i32,
            (&mut bm as *mut BITMAP).cast(),
        )
    };
    if written == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
        return None;
    }
    Some((bm.bmWidth as u32, bm.bmHeight as u32))
}

/// Copies a `size` x `size` bitmap into a top-down 32 bpp BGRA buffer.
fn read_bitmap_32bpp(dc: HDC, bitmap: HBITMAP, size: u32) -> Option<Vec<u8>> {
    let header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: size as i32,
        biHeight: -(size as i32), // negative = top-down rows
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    // BITMAPINFO = header followed by a colour table, unused for 32 bpp BI_RGB.
    let mut info = BITMAPINFO {
        bmiHeader: header,
        // SAFETY: RGBQUAD is plain data; all-zero is a valid value.
        bmiColors: [unsafe { std::mem::zeroed() }],
    };

    let mut buffer = vec![0u8; (size * size * 4) as usize];
    // SAFETY: `buffer` holds exactly `size` rows of `size` 32-bit pixels, matching `info`.
    let lines = unsafe {
        GetDIBits(
            dc,
            bitmap,
            0,
            size,
            buffer.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    (lines == size as i32).then_some(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_a_real_icon_from_explorer() {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".to_string());
        let explorer = Path::new(&windir).join("explorer.exe");
        let pixels = extract_icon_rgba(&explorer, 64).expect("explorer.exe has an icon");
        assert_eq!(pixels.len(), 64 * 64 * 4);
        // A real icon has both visible and transparent pixels.
        assert!(pixels.chunks_exact(4).any(|px| px[3] > 0));
        assert!(pixels.chunks_exact(4).any(|px| px[3] == 0));
    }

    #[test]
    fn missing_file_and_bad_sizes_return_none() {
        assert!(extract_icon_rgba(Path::new(r"C:\DeskPilotTest\missing.exe"), 64).is_none());
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".to_string());
        let explorer = Path::new(&windir).join("explorer.exe");
        assert!(extract_icon_rgba(&explorer, 0).is_none());
        assert!(extract_icon_rgba(&explorer, 1024).is_none());
    }
}
