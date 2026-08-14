//! 畫面文字辨識：用 Windows 內建的 OCR 引擎讀出截圖裡的文字，全程留在本機。

use crate::CaptureError;

#[cfg(target_os = "windows")]
use crate::screen::CapturedImage;

/// 找不到使用者慣用語言的辨識器時，依序嘗試的語言。
#[cfg(target_os = "windows")]
const FALLBACK_LANGUAGES: [&str; 4] = ["zh-Hant", "zh-Hans", "en", "ja"];

#[cfg(target_os = "windows")]
pub fn recognize_text(image: &CapturedImage) -> Result<String, CaptureError> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Storage::Streams::DataWriter;
    use windows::Win32::System::Com::CoIncrementMTAUsage;

    // WinRT 呼叫需要已初始化的 COM 執行緒；cookie 刻意不釋放，讓 MTA 存活到程式結束。
    let _ = unsafe { CoIncrementMTAUsage() }
        .map_err(|error| CaptureError::new(format!("無法初始化文字辨識：{error}")))?;

    let writer = DataWriter::new().map_err(winrt_error)?;
    writer.WriteBytes(&image.bgra).map_err(winrt_error)?;
    let buffer = writer.DetachBuffer().map_err(winrt_error)?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        image.width as i32,
        image.height as i32,
    )
    .map_err(winrt_error)?;

    let engine = recognizer()?;
    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(winrt_error)?
        .get()
        .map_err(winrt_error)?;

    let lines = result.Lines().map_err(winrt_error)?;
    let mut recognized = Vec::with_capacity(lines.Size().unwrap_or(0) as usize);
    for line in &lines {
        recognized.push(line.Text().map_err(winrt_error)?.to_string());
    }

    Ok(tidy_ocr_text(&recognized.join("\n")))
}

#[cfg(target_os = "windows")]
fn recognizer() -> Result<windows::Media::Ocr::OcrEngine, CaptureError> {
    use windows::Globalization::Language;
    use windows::Media::Ocr::OcrEngine;
    use windows::core::HSTRING;

    if let Ok(engine) = OcrEngine::TryCreateFromUserProfileLanguages() {
        return Ok(engine);
    }

    for tag in FALLBACK_LANGUAGES {
        let Ok(language) = Language::CreateLanguage(&HSTRING::from(tag)) else {
            continue;
        };
        if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language) {
            return Ok(engine);
        }
    }

    if let Ok(available) = OcrEngine::AvailableRecognizerLanguages() {
        for language in &available {
            if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language) {
                return Ok(engine);
            }
        }
    }

    Err(CaptureError::new(
        "這台電腦沒有可用的 OCR 語言套件，請到 Windows 設定的「語言與地區」安裝中文或英文的選用文字辨識功能",
    ))
}

#[cfg(target_os = "windows")]
fn winrt_error(error: windows::core::Error) -> CaptureError {
    CaptureError::new(format!("文字辨識失敗：{error}"))
}

/// 整理辨識結果：Windows OCR 會把中日韓文字逐字斷詞，要把多餘的空白接回來。
pub fn tidy_ocr_text(text: &str) -> String {
    text.lines()
        .map(|line| join_wide_words(line.trim()))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn join_wide_words(line: &str) -> String {
    let characters = line.chars().collect::<Vec<_>>();
    let mut joined = String::with_capacity(line.len());
    let mut index = 0;

    while index < characters.len() {
        if characters[index] != ' ' {
            joined.push(characters[index]);
            index += 1;
            continue;
        }

        let mut end = index;
        while end < characters.len() && characters[end] == ' ' {
            end += 1;
        }
        let before = joined.chars().next_back();
        let after = characters.get(end).copied();
        if !(before.is_some_and(is_wide) && after.is_some_and(is_wide)) {
            joined.extend(&characters[index..end]);
        }
        index = end;
    }

    joined
}

/// 中日韓文字與全形標點，這些字之間的空白是斷詞產物而不是原文。
fn is_wide(character: char) -> bool {
    matches!(
        character as u32,
        0x1100..=0x11FF
            | 0x2E80..=0x303F
            | 0x3040..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7FF
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE1F
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x20000..=0x3FFFD
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_word_breaks_are_joined_back_together() {
        assert_eq!(tidy_ocr_text("這是 一段 中文 字幕"), "這是一段中文字幕");
    }

    #[test]
    fn latin_words_keep_their_spacing() {
        assert_eq!(
            tidy_ocr_text("Save the current document"),
            "Save the current document"
        );
    }

    #[test]
    fn mixed_lines_only_lose_the_spaces_between_wide_characters() {
        assert_eq!(
            tidy_ocr_text("按下 Save 按鈕 即可 儲存"),
            "按下 Save 按鈕即可儲存"
        );
    }

    #[test]
    fn line_structure_survives_but_surrounding_blanks_do_not() {
        assert_eq!(
            tidy_ocr_text("  第一 行  \n  second line  \n"),
            "第一行\nsecond line"
        );
    }
}
