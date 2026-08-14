//! 螢幕範圍截圖：把使用者框選的畫面區域讀成 BGRA8 像素，交給 OCR 辨識。

use crate::CaptureError;

/// OCR 引擎可接受的最長邊。
const MAX_OCR_EDGE: u32 = 10_000;
/// 放大後希望達到的短邊像素，太小的字級 OCR 容易漏字。
const PREFERRED_OCR_EDGE: u32 = 1_100;
/// 小於這個邊長的框選視為誤觸。
pub const MIN_REGION_EDGE: u32 = 8;

/// 以實體像素表示的螢幕矩形。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScreenRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl ScreenRegion {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn is_usable(&self) -> bool {
        self.width >= MIN_REGION_EDGE && self.height >= MIN_REGION_EDGE
    }
}

/// 截取到的畫面，像素順序為 BGRA8。
#[derive(Clone, Debug)]
pub struct CapturedImage {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

/// 把截圖編成 PNG 再轉 base64，供多模態模型辨識用。
///
/// 兩件事一定要在這裡處理，否則模型收到的是一張看不見內容的圖：
///
/// 1. **通道順序**：擷取到的是 BGRA，PNG 要 RGB，B 和 R 得對調。
/// 2. **alpha 一律丟掉**。GDI 的 `BitBlt` 不會寫 alpha 通道，抓下來整片都是 0。
///    照原樣編成帶 alpha 的 PNG，出來會是一張**全透明**的圖——模型回「看不到文字」，
///    看起來像辨識能力不好，實際上是我們送了一張空白過去。螢幕內容本來就不透明，
///    直接輸出 RGB 最保險，順帶少掉四分之一的體積。
pub fn png_base64(image: &CapturedImage) -> Result<String, CaptureError> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let mut rgb = Vec::with_capacity(image.bgra.len() / 4 * 3);
    for pixel in image.bgra.chunks_exact(4) {
        rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
    }

    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, image.width, image.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| CaptureError::new(format!("圖片編碼失敗：{error}")))?;
        writer
            .write_image_data(&rgb)
            .map_err(|error| CaptureError::new(format!("圖片編碼失敗：{error}")))?;
    }

    Ok(STANDARD.encode(&encoded))
}

/// 決定截圖時要放大幾倍。小範圍放大能提高辨識率，但不得超過引擎上限。
pub fn upscale_factor(width: u32, height: u32) -> u32 {
    let shortest = width.min(height).max(1);
    let longest = width.max(height).max(1);
    let wanted = (PREFERRED_OCR_EDGE / shortest).clamp(1, 4);
    let allowed = (MAX_OCR_EDGE / longest).max(1);
    wanted.min(allowed)
}

/// 目前所有螢幕合起來的桌面範圍，用來鋪滿框選用的覆蓋視窗。
#[cfg(target_os = "windows")]
pub fn virtual_screen_bounds() -> Option<ScreenRegion> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    if width <= 0 || height <= 0 {
        return None;
    }

    Some(ScreenRegion::new(
        unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
        unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
        width as u32,
        height as u32,
    ))
}

#[cfg(target_os = "windows")]
pub fn capture_region(region: ScreenRegion) -> Result<CapturedImage, CaptureError> {
    use std::ffi::c_void;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection,
        DIB_RGB_COLORS, DeleteDC, DeleteObject, GdiFlush, GetDC, HALFTONE, HBITMAP, HDC, HGDIOBJ,
        ROP_CODE, ReleaseDC, SRCCOPY, SelectObject, SetBrushOrgEx, SetStretchBltMode, StretchBlt,
    };

    struct ScreenDc(HDC);
    impl Drop for ScreenDc {
        fn drop(&mut self) {
            unsafe { ReleaseDC(Some(HWND::default()), self.0) };
        }
    }

    struct MemoryDc(HDC);
    impl Drop for MemoryDc {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }

    struct Dib(HBITMAP);
    impl Drop for Dib {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(self.0.0));
            }
        }
    }

    if !region.is_usable() {
        return Err(CaptureError::new("選取範圍太小，請重新框選"));
    }

    let scale = upscale_factor(region.width, region.height);
    let width = region.width * scale;
    let height = region.height * scale;

    let screen = ScreenDc(unsafe { GetDC(Some(HWND::default())) });
    if screen.0.is_invalid() {
        return Err(CaptureError::new("無法讀取螢幕內容"));
    }

    let memory = MemoryDc(unsafe { CreateCompatibleDC(Some(screen.0)) });
    if memory.0.is_invalid() {
        return Err(CaptureError::new("無法建立截圖緩衝區"));
    }

    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            // 負高度代表由上而下排列，省去翻轉列的成本。
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    let bitmap = Dib(
        unsafe { CreateDIBSection(Some(memory.0), &info, DIB_RGB_COLORS, &mut bits, None, 0) }
            .map_err(|error| CaptureError::new(error.to_string()))?,
    );
    if bits.is_null() {
        return Err(CaptureError::new("無法配置截圖緩衝區"));
    }

    let previous = unsafe { SelectObject(memory.0, HGDIOBJ(bitmap.0.0)) };
    unsafe { SetStretchBltMode(memory.0, HALFTONE) };
    let _ = unsafe { SetBrushOrgEx(memory.0, 0, 0, None) };
    // CAPTUREBLT 讓分層視窗（例如選單、提示泡泡）也會出現在截圖裡。
    let copied = unsafe {
        StretchBlt(
            memory.0,
            0,
            0,
            width as i32,
            height as i32,
            Some(screen.0),
            region.x,
            region.y,
            region.width as i32,
            region.height as i32,
            ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0),
        )
    };
    let _ = unsafe { GdiFlush() };

    let pixel_bytes = (width as usize) * (height as usize) * 4;
    let mut bgra = vec![0u8; pixel_bytes];
    if copied.as_bool() {
        bgra.copy_from_slice(unsafe { std::slice::from_raw_parts(bits as *const u8, pixel_bytes) });
    }
    unsafe { SelectObject(memory.0, previous) };

    if !copied.as_bool() {
        return Err(CaptureError::new("無法複製螢幕內容"));
    }

    // 桌面像素的 alpha 通道多半是 0，補成不透明才不會被辨識器當成空白。
    for pixel in bgra.chunks_exact_mut(4) {
        pixel[3] = 0xFF;
    }

    Ok(CapturedImage {
        width,
        height,
        bgra,
    })
}

/// 讀出剪貼簿裡的圖片，轉成 OCR 要的 BGRA8。剪貼簿沒有圖片時回 `Ok(None)`。
///
/// 讓使用者可以改用自己慣用的截圖工具（Win＋Shift＋S、Snipping Tool、ShareX……）：
/// 那些工具都會把結果放進剪貼簿的 CF_BITMAP／CF_DIB。
///
/// 走 CF_BITMAP 而不是自己解析 CF_DIB：DIB 有 1/4/8/16/24/32 位元色深、調色盤、
/// BI_BITFIELDS 壓縮與由下而上的列順序等變體，逐一處理既長又容易漏。改用 GDI
/// 把來源畫進我們自己的 32 位元由上而下 DIB，格式轉換就交給系統。
#[cfg(target_os = "windows")]
pub fn clipboard_image() -> Result<Option<CapturedImage>, CaptureError> {
    use std::ffi::c_void;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection,
        DIB_RGB_COLORS, DeleteDC, DeleteObject, GdiFlush, GetDC, GetObjectW, HALFTONE, HBITMAP,
        HDC, HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject, SetBrushOrgEx, SetStretchBltMode,
        StretchBlt,
    };
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };

    const CF_BITMAP: u32 = 2;

    struct ClipboardGuard;
    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseClipboard();
            }
        }
    }

    struct ScreenDc(HDC);
    impl Drop for ScreenDc {
        fn drop(&mut self) {
            unsafe { ReleaseDC(Some(HWND::default()), self.0) };
        }
    }

    struct MemoryDc(HDC);
    impl Drop for MemoryDc {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }

    struct Dib(HBITMAP);
    impl Drop for Dib {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(self.0.0));
            }
        }
    }

    if unsafe { IsClipboardFormatAvailable(CF_BITMAP) }.is_err() {
        return Ok(None);
    }

    // 剪貼簿常被其他程式短暫佔用，重試幾次再放棄。
    let _clipboard = {
        let mut opened = None;
        for _ in 0..5 {
            if unsafe { OpenClipboard(None) }.is_ok() {
                opened = Some(ClipboardGuard);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        opened.ok_or_else(|| CaptureError::new("剪貼簿目前被其他程式佔用，請稍後再試"))?
    };

    // 這個控制代碼屬於剪貼簿，只能讀、不能刪。
    let Ok(handle) = (unsafe { GetClipboardData(CF_BITMAP) }) else {
        return Ok(None);
    };
    let source = HBITMAP(handle.0);
    if source.is_invalid() {
        return Ok(None);
    }

    let mut header = BITMAP::default();
    let read = unsafe {
        GetObjectW(
            HGDIOBJ(source.0),
            size_of::<BITMAP>() as i32,
            Some(&mut header as *mut BITMAP as *mut c_void),
        )
    };
    if read == 0 || header.bmWidth <= 0 || header.bmHeight == 0 {
        return Err(CaptureError::new("剪貼簿裡的圖片無法讀取"));
    }

    let source_width = header.bmWidth as u32;
    // 高度為負代表由上而下，取絕對值當像素高度。
    let source_height = header.bmHeight.unsigned_abs();

    let scale = upscale_factor(source_width, source_height);
    let width = source_width * scale;
    let height = source_height * scale;

    let screen = ScreenDc(unsafe { GetDC(Some(HWND::default())) });
    if screen.0.is_invalid() {
        return Err(CaptureError::new("無法建立繪圖環境"));
    }

    let source_dc = MemoryDc(unsafe { CreateCompatibleDC(Some(screen.0)) });
    let target_dc = MemoryDc(unsafe { CreateCompatibleDC(Some(screen.0)) });
    if source_dc.0.is_invalid() || target_dc.0.is_invalid() {
        return Err(CaptureError::new("無法建立圖片緩衝區"));
    }

    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            // 負高度＝由上而下，和 capture_region 一致，省去翻轉列的成本。
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    let target = Dib(
        unsafe { CreateDIBSection(Some(target_dc.0), &info, DIB_RGB_COLORS, &mut bits, None, 0) }
            .map_err(|error| CaptureError::new(error.to_string()))?,
    );
    if bits.is_null() {
        return Err(CaptureError::new("無法配置圖片緩衝區"));
    }

    let previous_source = unsafe { SelectObject(source_dc.0, HGDIOBJ(source.0)) };
    let previous_target = unsafe { SelectObject(target_dc.0, HGDIOBJ(target.0.0)) };
    unsafe { SetStretchBltMode(target_dc.0, HALFTONE) };
    let _ = unsafe { SetBrushOrgEx(target_dc.0, 0, 0, None) };
    let copied = unsafe {
        StretchBlt(
            target_dc.0,
            0,
            0,
            width as i32,
            height as i32,
            Some(source_dc.0),
            0,
            0,
            source_width as i32,
            source_height as i32,
            SRCCOPY,
        )
    };
    let _ = unsafe { GdiFlush() };

    let pixel_bytes = (width as usize) * (height as usize) * 4;
    let mut bgra = vec![0u8; pixel_bytes];
    if copied.as_bool() {
        bgra.copy_from_slice(unsafe { std::slice::from_raw_parts(bits as *const u8, pixel_bytes) });
    }
    // 一定要在來源點陣圖被釋放前解除選取，否則剪貼簿的物件會留在我們的 DC 上。
    unsafe { SelectObject(source_dc.0, previous_source) };
    unsafe { SelectObject(target_dc.0, previous_target) };

    if !copied.as_bool() {
        return Err(CaptureError::new("無法讀取剪貼簿裡的圖片"));
    }

    // 截圖工具常留下 alpha=0 的像素，補成不透明才不會被辨識器當成空白。
    for pixel in bgra.chunks_exact_mut(4) {
        pixel[3] = 0xFF;
    }

    Ok(Some(CapturedImage {
        width,
        height,
        bgra,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 這個測試守的是「送出去的圖到底看不看得見」。
    /// 通道對調弄反、或把 GDI 留下的 alpha=0 一起編進去，
    /// 症狀都是模型回「沒有文字」，從外面完全看不出是我們送錯了圖。
    #[test]
    fn png_encoding_drops_the_unset_alpha_and_puts_channels_back_in_order() {
        use base64::{Engine, engine::general_purpose::STANDARD};

        // BGRA，alpha 全是 0——GDI 擷取後的實際樣子
        let image = CapturedImage {
            width: 2,
            height: 1,
            bgra: vec![
                0x10, 0x20, 0x30, 0x00, // 第一個像素：B=0x10 G=0x20 R=0x30
                0xC0, 0xB0, 0xA0, 0x00, // 第二個像素：B=0xC0 G=0xB0 R=0xA0
            ],
        };

        let encoded = STANDARD.decode(png_base64(&image).unwrap()).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(&encoded))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();

        assert_eq!(info.color_type, png::ColorType::Rgb, "不該帶 alpha 通道");
        assert_eq!(
            &pixels[..info.buffer_size()],
            // RGB：B 與 R 已對調
            &[0x30, 0x20, 0x10, 0xA0, 0xB0, 0xC0]
        );
    }

    #[test]
    fn small_regions_are_upscaled_for_recognition() {
        assert_eq!(upscale_factor(320, 180), 4);
        assert_eq!(upscale_factor(1200, 550), 2);
        assert_eq!(upscale_factor(1920, 1200), 1);
    }

    #[test]
    fn upscaling_never_exceeds_the_recognizer_limit() {
        assert_eq!(upscale_factor(6000, 400), 1);
        assert_eq!(upscale_factor(3000, 300), 3);
    }

    #[test]
    fn accidental_clicks_are_not_usable_regions() {
        assert!(!ScreenRegion::new(10, 10, 4, 90).is_usable());
        assert!(ScreenRegion::new(10, 10, 120, 40).is_usable());
    }
}
