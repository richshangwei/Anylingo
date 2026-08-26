//! Rust 這側的介面語言字串。
//!
//! 前端有一份 `src/locales/*.ts`，這裡再有一份，看起來像重複——不是。兩邊管的是
//! 不同的東西，而且**沒有交集**：
//!
//! - 前端那份是面板裡的字，由 webview 畫出來。
//! - 這份是 webview 畫不到的字：系統匣選單、系統匣提示、啟動時的快捷鍵佔用通知。
//!   那個選單是 Windows 自己畫的原生選單，前端連它存在都不知道。
//!
//! 兩邊靠 `ui/locale` 這個偏好值對齊，語言 id 的字串必須一模一樣
//! （`zh-TW`／`en`／`ja`／`zh-CN`）。加語言時兩邊都要加，否則會出現面板是英文、
//! 右下角是中文的狀況。
//!
//! 這裡刻意不共用同一份 JSON：Rust 只需要五、六個字串，為了它去做一套執行期
//! JSON 解析與回退，換來的是「語言檔壞掉時系統匣沒有文字」這種更難查的故障。
//! 編譯期的 `&'static str` 沒有這個問題。

/// 介面語言。值與前端 `locales/types.ts` 的 `LocaleId` 同一組字串。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiLocale {
    ZhTw,
    En,
    Ja,
    ZhCn,
}

impl UiLocale {
    /// 產品的母語版本。要和前端的 `DEFAULT_LOCALE_ID` 一致。
    pub const DEFAULT: Self = Self::ZhTw;

    /// 把資料庫裡的值收斂成一個確定存在的語言。
    ///
    /// 認不得就回預設而不是報錯：這個值可能是舊版寫的、可能被手動改過。
    /// 介面語言讀不出來時該退回預設繼續跑，不該讓系統匣建不起來——那會讓
    /// 整個程式沒有任何入口。
    pub fn parse(value: &str) -> Self {
        match value {
            "zh-TW" => Self::ZhTw,
            "zh-CN" => Self::ZhCn,
            "en" => Self::En,
            "ja" => Self::Ja,
            // `en-US`、`ja-JP` 這類帶地區的標籤取主語言再試一次。
            other => match other.split('-').next().unwrap_or_default() {
                "en" => Self::En,
                "ja" => Self::Ja,
                // 只寫 `zh` 時給繁體：這是產品的母語版本，猜不到就往預設靠。
                "zh" => Self::ZhTw,
                _ => Self::DEFAULT,
            },
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ZhTw => "zh-TW",
            Self::En => "en",
            Self::Ja => "ja",
            Self::ZhCn => "zh-CN",
        }
    }

    pub fn strings(self) -> &'static UiStrings {
        match self {
            Self::ZhTw => &ZH_TW,
            Self::En => &EN,
            Self::Ja => &JA,
            Self::ZhCn => &ZH_CN,
        }
    }
}

/// 系統匣與啟動通知用得到的字串。
pub struct UiStrings {
    pub tray_show: &'static str,
    pub tray_capture: &'static str,
    pub tray_hide: &'static str,
    pub tray_quit: &'static str,
    /// 滑鼠停在系統匣圖示上時的提示。習慣上是「產品名 — 標語」。
    pub tray_tooltip: &'static str,
    /// 兩組全域快捷鍵的說明，用在「已被佔用」的通知裡。
    pub shortcut_translate: &'static str,
    pub shortcut_capture: &'static str,
    /// 列舉多組快捷鍵時的分隔符。中文用頓號，英文用逗號加空格。
    pub list_separator: &'static str,
    /// 快捷鍵註冊失敗時的提醒。參數是已經串好的快捷鍵清單。
    ///
    /// 用函式而不是 `{}` 樣板，是為了讓每個語言自己決定那份清單擺在句子的哪裡。
    pub shortcuts_occupied: fn(&str) -> String,
}

static ZH_TW: UiStrings = UiStrings {
    tray_show: "顯示隨譯",
    tray_capture: "截圖翻譯",
    tray_hide: "隱藏隨譯",
    tray_quit: "結束隨譯",
    tray_tooltip: "隨譯 Anylingo — 所見所選，皆可譯",
    shortcut_translate: "Ctrl＋Alt＋T（翻譯選取文字）",
    shortcut_capture: "Ctrl＋Alt＋R（截圖翻譯）",
    list_separator: "、",
    shortcuts_occupied: |list| {
        format!("{list} 已被其他程式佔用，可改用系統匣選單或面板上的按鈕。")
    },
};

static EN: UiStrings = UiStrings {
    tray_show: "Show Anylingo",
    tray_capture: "Capture and translate",
    tray_hide: "Hide Anylingo",
    tray_quit: "Quit Anylingo",
    tray_tooltip: "Anylingo — see it, select it, translate it",
    shortcut_translate: "Ctrl+Alt+T (translate selection)",
    shortcut_capture: "Ctrl+Alt+R (capture and translate)",
    list_separator: ", ",
    shortcuts_occupied: |list| {
        format!("{list} is already taken by another program. Use the tray menu or the panel buttons instead.")
    },
};

static JA: UiStrings = UiStrings {
    tray_show: "Anylingo を表示",
    tray_capture: "画面を切り取って翻訳",
    tray_hide: "Anylingo を隠す",
    tray_quit: "Anylingo を終了",
    tray_tooltip: "Anylingo — 見えるものは、すべて訳せる",
    shortcut_translate: "Ctrl＋Alt＋T（選択した文字を翻訳）",
    shortcut_capture: "Ctrl＋Alt＋R（画面を切り取って翻訳）",
    list_separator: "、",
    shortcuts_occupied: |list| {
        format!("{list} は他のプログラムが使用中です。通知領域のメニューかパネルのボタンをご利用ください。")
    },
};

static ZH_CN: UiStrings = UiStrings {
    tray_show: "显示随译",
    tray_capture: "截图翻译",
    tray_hide: "隐藏随译",
    tray_quit: "退出随译",
    tray_tooltip: "随译 Anylingo — 所见所选，皆可译",
    shortcut_translate: "Ctrl＋Alt＋T（翻译选中文字）",
    shortcut_capture: "Ctrl＋Alt＋R（截图翻译）",
    list_separator: "、",
    shortcuts_occupied: |list| {
        format!("{list} 已被其他程序占用，可改用托盘菜单或面板上的按钮。")
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    /// 前端與 Rust 靠 `ui/locale` 的字串值對齊，兩邊對不上就會一半中文一半英文。
    /// 這裡釘住 Rust 這側認得的那組值。
    #[test]
    fn every_locale_round_trips_through_its_id() {
        for locale in [UiLocale::ZhTw, UiLocale::En, UiLocale::Ja, UiLocale::ZhCn] {
            assert_eq!(UiLocale::parse(locale.as_str()), locale);
        }
    }

    #[test]
    fn unknown_values_fall_back_to_the_default_instead_of_failing() {
        assert_eq!(UiLocale::parse(""), UiLocale::DEFAULT);
        assert_eq!(UiLocale::parse("kl-GL"), UiLocale::DEFAULT);
        // 舊版寫進去的值、或被手動改過的值都走這條路。
        assert_eq!(UiLocale::parse("klingon"), UiLocale::DEFAULT);
    }

    #[test]
    fn regional_tags_land_on_their_base_language() {
        assert_eq!(UiLocale::parse("en-US"), UiLocale::En);
        assert_eq!(UiLocale::parse("en-GB"), UiLocale::En);
        assert_eq!(UiLocale::parse("ja-JP"), UiLocale::Ja);
        assert_eq!(UiLocale::parse("zh-Hant"), UiLocale::ZhTw);
    }

    /// 系統匣選單沒有文字就只剩四個空白列，使用者完全不知道哪一項是結束。
    #[test]
    fn no_locale_ships_an_empty_tray_label() {
        for locale in [UiLocale::ZhTw, UiLocale::En, UiLocale::Ja, UiLocale::ZhCn] {
            let strings = locale.strings();
            for label in [
                strings.tray_show,
                strings.tray_capture,
                strings.tray_hide,
                strings.tray_quit,
                strings.tray_tooltip,
                strings.shortcut_translate,
                strings.shortcut_capture,
                strings.list_separator,
            ] {
                assert!(!label.trim().is_empty(), "{} 有空白字串", locale.as_str());
            }
            assert!(!(strings.shortcuts_occupied)("Ctrl+Alt+T").trim().is_empty());
        }
    }
}
