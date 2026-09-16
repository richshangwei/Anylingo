#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod locale;

use locale::UiLocale;

use std::{
    fs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use floatrans_capture::{
    CaptureOutcome, CaptureRequest, CapturedImage, CursorPosition, PreservingClipboardFallback,
    ScreenRegion, SelectionCapture, WindowsUiaSelectionReader, capture_region, clipboard_image,
    cursor_position, double_click_interval, foreground_is_own_process, left_mouse_button_is_down,
    png_base64, recognize_text, virtual_screen_bounds,
};
use floatrans_core::{RequestImage, TranslationEvent, TranslationRequest};
use floatrans_providers::{
    Anthropic, AzureOpenAi, FedGpt, GoogleGemini, OllamaNative, OpenAiCompatible,
    TranslationProvider,
};
use floatrans_storage::{
    LocalData, ModelProfile, ProviderKind, SqliteStore, WindowsCredentialStore,
};
use serde::{Deserialize, Serialize};
use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStatus {
    app_name: &'static str,
    version: &'static str,
    supported_providers: [&'static str; 9],
}

/// 偏好設定的鍵名。前後端共用同一組字串，打錯就會靜默讀到預設值，所以集中在此。
const PREF_SHOW_SOURCE: &str = "panel/show-source";
const PREF_AUTO_COLLAPSE: &str = "panel/auto-collapse";
/// 圈完字而 UI Automation 問不到內容時，允不允許模擬一次 Ctrl+C 取字。
///
/// 預設開啟：少了它，Electron、Qt、Java、終端機、遊戲介面這些不透過 UIA
/// 交代文字的程式，永遠不會冒出「譯」按鈕。留一個開關是因為這條路會朝
/// 前景程式送按鍵，總有人的工具會被這一下打擾。
const PREF_CLIPBOARD_FALLBACK: &str = "capture/clipboard-fallback";
/// 是否在目前 Windows 使用者登入時自動啟動隨譯。預設關閉，必須由使用者主動開啟。
const PREF_LAUNCH_AT_LOGIN: &str = "app/launch-at-login";
/// 圖片要用什麼辨識出文字。值見 `ImageRecognition`。
const PREF_IMAGE_RECOGNITION: &str = "capture/image-recognition";
/// 目前選用的模型設定檔 id。
///
/// 這個選擇本來只活在前端，關掉重開就跳回第一個設定檔。改存起來有兩個理由：
/// 一是使用者的選擇本該記住，二是圖片交給模型辨識時 Rust 這側需要知道要用誰——
/// 而截圖覆蓋層是另一個視窗，它不會載入設定檔清單，問不到主面板選了什麼。
const PREF_ACTIVE_PROFILE: &str = "model/active-profile";
/// 介面語言。值見 `locale::UiLocale`，與前端 `locales/types.ts` 的 `LocaleId` 同一組字串。
///
/// 這個偏好比其他幾個多管一件事：**系統匣選單是 Rust 建的原生選單**，前端換了語言
/// 不會動到它，所以寫入這個鍵的時候要順手把選單重建一次（見 `set_choice_preference`）。
const PREF_UI_LOCALE: &str = "ui/locale";

/// 圖片轉文字的方式。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImageRecognition {
    /// Windows 內建 OCR。全程在本機，圖片不離開這台電腦。
    SystemOcr,
    /// 交給模型辨識。需要模型支援讀圖，**圖片會送到模型端點**。
    Model,
    /// 先問模型，模型不支援或出錯就退回系統 OCR。
    Auto,
}

impl ImageRecognition {
    fn parse(value: &str) -> Self {
        match value {
            "model" => Self::Model,
            "auto" => Self::Auto,
            // 認不得的值一律當成系統 OCR：那是唯一不會把畫面送出去的選項，
            // 設定損毀時不該默默改成上傳。
            _ => Self::SystemOcr,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::SystemOcr => "ocr",
            Self::Model => "model",
            Self::Auto => "auto",
        }
    }
}

/// 譯文面板展開與收合後的邏輯尺寸，定位計算與 set_panel_collapsed 共用。
///
/// 字級放大後 420×540 裝不下空狀態（標題、兩段提示、快捷鍵列與截圖按鈕），
/// 底部會被裁掉並長出卷軸。這裡的尺寸要跟著 app.css 的字級一起看。
const PANEL_SIZE: (f64, f64) = (480.0, 640.0);
/// 收合後只剩一顆方形圖示。要跟著改 tauri.conf.json 的 `minWidth`：
/// 視窗的最小尺寸限制會蓋掉 set_size，比它小的收合尺寸根本套不上去。
const COLLAPSED_SIZE: (f64, f64) = (36.0, 36.0);
/// 簡易版面板的尺寸：跟隨選取游標出現，只顯示譯文與複製按鈕。
const PANEL_SIZE_MINI: (f64, f64) = (380.0, 220.0);

/// 面板剛開好之後這段時間內不自動收合。
///
/// 點「譯」按鈕的那一次放開左鍵，同時也會走到自動收合檢查，於是面板為了這次
/// 翻譯剛開好就被收掉——完整面板會縮成小標籤並彈到右下角，簡易面板則直接消失。
/// 這是使用者看到的「點一下面板會跳」。
const PANEL_SHOW_GRACE: std::time::Duration = std::time::Duration::from_millis(700);

struct AppState {
    data: Mutex<LocalData<WindowsCredentialStore>>,
    translation_generation: Arc<AtomicU64>,
    explanation_generation: Arc<AtomicU64>,
    pending_selection: Mutex<Option<String>>,
    panel_visible_before_capture: Mutex<bool>,
    startup_notice: Mutex<Option<String>>,
    /// 釘選後面板固定在目前位置；未釘選時會跟著這次選取的位置移動。
    panel_pinned: Mutex<bool>,
    /// 簡易模式：只顯示譯文與複製按鈕，由「譯」按鈕與快捷鍵觸發。
    /// 截圖翻譯與系統匣入口仍使用完整面板。
    panel_mini: Mutex<bool>,
    /// 面板最後一次為了顯示譯文而開啟的時間，供 PANEL_SHOW_GRACE 判斷。
    panel_shown_at: Mutex<Option<std::time::Instant>>,
    /// 全螢幕閱讀模式。開著時所有會改尺寸或位置的路徑都要讓開，
    /// 否則一次新翻譯就會把視窗從全螢幕拉回小面板。
    panel_fullscreen: Mutex<bool>,
    /// 系統匣圖示。留著它是為了換介面語言時能把選單整個換掉——
    /// Tauri 沒有改單一選單項文字的 API，只能重建，而重建需要這個 handle。
    tray: Mutex<Option<tauri::tray::TrayIcon>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelProfileView {
    id: String,
    name: String,
    provider: String,
    endpoint: String,
    model: String,
    has_credential: bool,
}

impl From<ModelProfile> for ModelProfileView {
    fn from(profile: ModelProfile) -> Self {
        Self {
            id: profile.id,
            name: profile.name,
            provider: provider_name(&profile.provider).into(),
            endpoint: profile.endpoint,
            model: profile.model,
            has_credential: profile.credential_key.is_some(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveModelProfile {
    id: String,
    name: String,
    provider: String,
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

/// 覆蓋視窗回報的框選範圍，單位是 CSS 像素加上該視窗的縮放比例。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegionSelection {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f64,
}

impl RegionSelection {
    fn to_screen_region(&self, origin_x: i32, origin_y: i32) -> ScreenRegion {
        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        ScreenRegion::new(
            origin_x + (self.x * scale).round() as i32,
            origin_y + (self.y * scale).round() as i32,
            (self.width * scale).round().max(0.0) as u32,
            (self.height * scale).round().max(0.0) as u32,
        )
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslationStarted {
    source_text: String,
    target_language: String,
}

/// 更新檢查的三種結果。舊版把「沒有更新頻道」與「已是最新」都回 None，
/// 前端因此無法給出正確說明，也無法告訴使用者這個建置根本不會自動更新。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
enum UpdateStatus {
    /// 建置時沒有帶入更新端點或公鑰，例如本機自用或免安裝版。
    Disabled,
    UpToDate { current: String },
    Available { version: String, notes: Option<String> },
}

#[tauri::command]
fn runtime_status() -> RuntimeStatus {
    RuntimeStatus {
        app_name: "隨譯",
        version: env!("CARGO_PKG_VERSION"),
        supported_providers: [
            "anthropic",
            "azure-openai",
            "google-gemini",
            "openai-compatible",
            "openrouter",
            "xai",
            "ollama-native",
            "fedgpt",
            "custom-endpoint",
        ],
    }
}

#[tauri::command]
fn model_profiles(state: tauri::State<'_, AppState>) -> Result<Vec<ModelProfileView>, String> {
    state
        .data
        .lock()
        .map_err(|_| "模型設定目前無法使用".to_owned())?
        .model_profiles()
        .map(|profiles| profiles.into_iter().map(Into::into).collect())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_model_profile(
    state: tauri::State<'_, AppState>,
    input: SaveModelProfile,
) -> Result<ModelProfileView, String> {
    let provider = parse_provider(&input.provider)?;
    let credential_key = (!matches!(provider, ProviderKind::OllamaNative))
        .then(|| format!("profile/{}/api-key", input.id));
    let profile = ModelProfile {
        id: input.id.trim().to_owned(),
        name: input.name.trim().to_owned(),
        provider,
        endpoint: input.endpoint.trim().trim_end_matches('/').to_owned(),
        model: input.model.trim().to_owned(),
        credential_key,
    };
    if profile.id.is_empty()
        || profile.name.is_empty()
        || profile.endpoint.is_empty()
        || profile.model.is_empty()
    {
        return Err("請完整填寫設定名稱、端點與模型".into());
    }

    let secret = input
        .api_key
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    state
        .data
        .lock()
        .map_err(|_| "模型設定目前無法使用".to_owned())?
        .save_model_profile(&profile, secret)
        .map_err(|error| error.to_string())?;
    Ok(profile.into())
}

/// 介面偏好。預設值刻意寫在這裡而不是前端：前端只是呈現層，
/// 若兩邊各寫一份預設值，改了一邊就會不一致。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Preferences {
    /// 翻譯後是否自動展開原文。預設關閉，譯文才是使用者要看的東西。
    show_source: bool,
    /// 點面板以外的地方時是否自動收合。預設開啟。
    auto_collapse: bool,
    /// 圈完字而 UIA 問不到內容時，是否用模擬 Ctrl+C 補上。預設開啟。
    clipboard_fallback: bool,
    /// 登入 Windows 後是否自動啟動。預設關閉。
    launch_at_login: bool,
    /// 圖片辨識方式：`ocr`／`model`／`auto`。預設 `ocr`。
    ///
    /// 預設值刻意是「不上傳」的那一個。截圖可能拍到任何東西，把它送到模型端點
    /// 是使用者該自己決定的事，不能因為升級了一版就默默開始送。
    image_recognition: String,
    /// 介面語言，例如 `zh-TW`。預設是產品的母語版本。
    ui_locale: String,
}

#[tauri::command]
fn preferences(state: tauri::State<'_, AppState>) -> Result<Preferences, String> {
    let data = state
        .data
        .lock()
        .map_err(|_| "偏好設定目前無法使用".to_owned())?;
    Ok(Preferences {
        show_source: data
            .flag(PREF_SHOW_SOURCE, false)
            .map_err(|error| error.to_string())?,
        auto_collapse: data
            .flag(PREF_AUTO_COLLAPSE, true)
            .map_err(|error| error.to_string())?,
        clipboard_fallback: data
            .flag(PREF_CLIPBOARD_FALLBACK, true)
            .map_err(|error| error.to_string())?,
        launch_at_login: data
            .flag(PREF_LAUNCH_AT_LOGIN, false)
            .map_err(|error| error.to_string())?,
        image_recognition: data
            .choice(PREF_IMAGE_RECOGNITION, ImageRecognition::SystemOcr.as_str())
            .map_err(|error| error.to_string())?,
        ui_locale: data
            .choice(PREF_UI_LOCALE, UiLocale::DEFAULT.as_str())
            .map_err(|error| error.to_string())?,
    })
}

/// 只問介面語言。
///
/// 「譯」按鈕與框選覆蓋層是另外兩個 webview，它們用不到設定檔、釘選狀態那些東西，
/// 但畫面上仍有字（按鈕的提示、框選的說明）。給它們一個只回一個字串的指令，
/// 比讓它們去拉整包偏好乾淨。
#[tauri::command]
fn ui_locale(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let data = state
        .data
        .lock()
        .map_err(|_| "偏好設定目前無法使用".to_owned())?;
    data.choice(PREF_UI_LOCALE, UiLocale::DEFAULT.as_str())
        .map(|value| UiLocale::parse(&value).as_str().to_owned())
        .map_err(|error| error.to_string())
}

/// 設定選項多於兩個的偏好。`set_preference` 只收布林，裝不下三選一。
#[tauri::command]
fn set_choice_preference(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    // 逐鍵驗值，不要照單全收：寫進一個認不得的值，讀取端只會安靜地退回預設，
    // 使用者則會看到設定「按了沒有用」。
    let value = match key.as_str() {
        PREF_IMAGE_RECOGNITION => ImageRecognition::parse(&value).as_str().to_owned(),
        PREF_UI_LOCALE => UiLocale::parse(&value).as_str().to_owned(),
        PREF_ACTIVE_PROFILE => value,
        _ => return Err(format!("不支援的偏好設定：{key}")),
    };
    state
        .data
        .lock()
        .map_err(|_| "偏好設定目前無法使用".to_owned())?
        .set_choice(&key, &value)
        .map_err(|error| error.to_string())?;

    // 換語言要多做兩件 webview 自己做不到的事，而且**要在存檔成功之後**才做——
    // 存不進去卻換了選單，重開之後又會變回舊語言，比完全沒反應更難理解。
    if key == PREF_UI_LOCALE {
        let locale = UiLocale::parse(&value);
        // 系統匣選單是 Windows 畫的原生選單，前端改不到它。
        apply_tray_locale(&app, locale);
        // 「譯」按鈕與框選覆蓋層建好之後就一直活著，不會重新載入，所以用廣播推過去。
        let _ = app.emit("panel://locale", locale.as_str());
    }
    Ok(())
}

/// 依目前語言重建系統匣選單與提示。
///
/// Tauri 沒有「改某一個選單項的文字」這種 API，只能整個選單換掉。重建失敗時
/// 靜靜留著舊選單：那頂多是語言沒跟上，總比把唯一的入口弄不見好——面板可以隱藏，
/// 系統匣圖示是使用者叫回面板的最後一條路。
fn apply_tray_locale(app: &tauri::AppHandle, locale: UiLocale) {
    let handle = app.clone();
    // Windows 的選單只能在建立它的執行緒（也就是主執行緒）上動。同步指令目前確實
    // 跑在主執行緒，但那是 Tauri 的實作細節而不是我們的保證——哪天這個指令為了
    // 別的理由改成 async，選單就會從背景執行緒被動到，症狀是換語言沒反應或直接當掉。
    // 繞一趟事件迴圈，兩種情況都對。
    let _ = app.run_on_main_thread(move || {
        let strings = locale.strings();
        let Some(tray) = handle
            .state::<AppState>()
            .tray
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
        else {
            return;
        };
        let Ok(menu) = build_tray_menu(&handle, locale) else {
            return;
        };
        let _ = tray.set_menu(Some(menu));
        let _ = tray.set_tooltip(Some(strings.tray_tooltip));
    });
}

/// 系統匣選單。啟動時與換語言時共用同一份，兩邊才不會長出不一樣的項目。
fn build_tray_menu(
    app: &tauri::AppHandle,
    locale: UiLocale,
) -> tauri::Result<Menu<tauri::Wry>> {
    let strings = locale.strings();
    let show = MenuItem::with_id(app, "show", strings.tray_show, true, None::<&str>)?;
    let shot = MenuItem::with_id(app, "shot", strings.tray_capture, true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", strings.tray_hide, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", strings.tray_quit, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &shot, &hide, &quit])
}

/// 啟動時讀出介面語言。系統匣要在前端還沒載入之前就把文字擺好。
fn startup_locale(app: &tauri::AppHandle) -> UiLocale {
    app.try_state::<AppState>()
        .and_then(|state| {
            let data = state.data.lock().ok()?;
            data.choice(PREF_UI_LOCALE, UiLocale::DEFAULT.as_str()).ok()
        })
        .map(|value| UiLocale::parse(&value))
        .unwrap_or(UiLocale::DEFAULT)
}

/// 上次選用的模型設定檔。前端啟動時回讀，讓選擇跨重啟保留。
#[tauri::command]
fn active_model_profile(state: tauri::State<'_, AppState>) -> Option<String> {
    let data = state.data.lock().ok()?;
    data.choice(PREF_ACTIVE_PROFILE, "")
        .ok()
        .filter(|id| !id.is_empty())
}

#[tauri::command]
fn set_preference(
    state: tauri::State<'_, AppState>,
    key: String,
    value: bool,
) -> Result<(), String> {
    if !matches!(
        key.as_str(),
        PREF_SHOW_SOURCE | PREF_AUTO_COLLAPSE | PREF_CLIPBOARD_FALLBACK | PREF_LAUNCH_AT_LOGIN
    ) {
        return Err(format!("不支援的偏好設定：{key}"));
    }
    let mut data = state
        .data
        .lock()
        .map_err(|_| "偏好設定目前無法使用".to_owned())?;

    if key == PREF_LAUNCH_AT_LOGIN {
        // 先改 Windows，再存偏好。若系統拒絕寫入，設定畫面會收到錯誤，資料庫也不會
        // 留下一個其實沒有生效的勾選狀態。
        let previous = data
            .flag(PREF_LAUNCH_AT_LOGIN, false)
            .map_err(|error| error.to_string())?;
        set_launch_at_login(value)?;
        if let Err(error) = data.set_flag(&key, value) {
            // SQLite 寫入失敗時盡力把 Windows 還原，避免畫面下次讀到舊值、系統卻
            // 已經套用新值。還原也失敗時仍以原始的儲存錯誤回報，較能指出根因。
            let _ = set_launch_at_login(previous);
            return Err(error.to_string());
        }
        return Ok(());
    }

    data.set_flag(&key, value)
        .map_err(|error| error.to_string())
}

/// 寫入目前使用者的 Run 登錄值，不需要系統管理員權限，也不會影響其他帳號。
///
/// 執行檔路徑一定加雙引號，安裝在含空白路徑時 Windows 才不會把前半段誤認成程式。
#[cfg(windows)]
fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::{
            Foundation::ERROR_FILE_NOT_FOUND,
            System::Registry::{
                HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW,
                RegOpenKeyExW, RegSetValueExW,
            },
        },
        core::w,
    };

    let command = if enabled {
        let executable =
            std::env::current_exe().map_err(|error| format!("無法取得隨譯執行檔位置：{error}"))?;
        let mut command = Vec::<u16>::new();
        command.push('"' as u16);
        command.extend(executable.as_os_str().encode_wide());
        command.push('"' as u16);
        command.push(0);
        Some(command)
    } else {
        None
    };

    let mut run_key = HKEY::default();
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Run"),
            None,
            KEY_SET_VALUE,
            &mut run_key,
        )
    };
    if opened.0 != 0 {
        return Err(format!(
            "無法開啟 Windows 開機啟動設定（錯誤碼 {}）",
            opened.0
        ));
    }

    let operation = if let Some(command) = command {
        let bytes =
            unsafe { std::slice::from_raw_parts(command.as_ptr().cast::<u8>(), command.len() * 2) };
        unsafe { RegSetValueExW(run_key, w!("Floatrans"), None, REG_SZ, Some(bytes)) }
    } else {
        unsafe { RegDeleteValueW(run_key, w!("Floatrans")) }
    };
    let _ = unsafe { RegCloseKey(run_key) };

    if operation.0 == 0 || (!enabled && operation == ERROR_FILE_NOT_FOUND) {
        Ok(())
    } else {
        Err(format!(
            "無法更新 Windows 開機啟動設定（錯誤碼 {}）",
            operation.0
        ))
    }
}

#[cfg(not(windows))]
fn set_launch_at_login(_enabled: bool) -> Result<(), String> {
    Err("此功能目前只支援 Windows".to_owned())
}

/// 依模型設定檔建出對應的供應商。翻譯與解釋共用同一份設定，
/// 所以這段不該複製兩次。
fn build_provider(
    profile: ModelProfile,
    secret: Option<String>,
) -> Result<Box<dyn TranslationProvider>, String> {
    let provider: Box<dyn TranslationProvider> = match profile.provider {
        ProviderKind::OpenAiCompatible
        | ProviderKind::OpenRouter
        | ProviderKind::XAi
        | ProviderKind::CustomEndpoint => Box::new(
            OpenAiCompatible::new(profile.endpoint, profile.model, secret)
                .map_err(|error| error.to_string())?,
        ),
        ProviderKind::Anthropic => Box::new(
            Anthropic::new(profile.endpoint, profile.model, secret.unwrap_or_default())
                .map_err(|error| error.to_string())?,
        ),
        ProviderKind::AzureOpenAi => Box::new(
            AzureOpenAi::new(profile.endpoint, profile.model, secret.unwrap_or_default())
                .map_err(|error| error.to_string())?,
        ),
        ProviderKind::GoogleGemini => Box::new(
            GoogleGemini::new(profile.endpoint, profile.model, secret.unwrap_or_default())
                .map_err(|error| error.to_string())?,
        ),
        ProviderKind::FedGpt => Box::new(
            FedGpt::new(profile.endpoint, profile.model, secret.unwrap_or_default())
                .map_err(|error| error.to_string())?,
        ),
        ProviderKind::OllamaNative => Box::new(
            OllamaNative::new(profile.endpoint, profile.model)
                .map_err(|error| error.to_string())?,
        ),
    };
    Ok(provider)
}

/// 讀出模型設定檔與對應的 API Key。
fn profile_with_secret(
    state: &tauri::State<'_, AppState>,
    profile_id: &str,
) -> Result<(ModelProfile, Option<String>), String> {
    let data = state
        .data
        .lock()
        .map_err(|_| "模型設定目前無法使用".to_owned())?;
    let profile = data
        .model_profile(profile_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "找不到選取的模型設定".to_owned())?;
    let secret = match &profile.credential_key {
        Some(key) => data.model_secret(key).map_err(|error| error.to_string())?,
        None => None,
    };
    Ok((profile, secret))
}

/// 把截圖轉成文字，依偏好決定走系統 OCR 還是模型辨識。
///
/// 兩個入口（框選截圖、貼上圖片）共用這裡，設定才不會只在其中一條路上生效。
async fn recognize_captured_image(
    app: &tauri::AppHandle,
    image: CapturedImage,
) -> Result<String, String> {
    let mode = image_recognition_mode(app);
    if mode == ImageRecognition::SystemOcr {
        return system_ocr(image).await;
    }

    // 模型辨識要跑好幾秒，而 OCR 幾乎是瞬間完成的。沒有這個提示，
    // 使用者會盯著一個沒有動靜的面板，以為截圖翻譯壞了。
    let _ = app.emit_to("main", "capture://recognizing", "正在用模型辨識圖片…");

    match transcribe_with_model(app, &image).await {
        Ok(text) => Ok(text),
        Err(error) if mode == ImageRecognition::Auto => {
            // 「自動」的意思就是模型問不出來時換系統 OCR。最常見的原因是
            // 這個模型根本不讀圖，端點會直接回 400。
            //
            // 原因要帶出來。退回 OCR 之後畫面上會有結果，使用者不會發現模型
            // 這條路失敗過，也就永遠不知道自己選的模型其實不支援讀圖。
            // 這則訊息隨即會被辨識結果取代，所以截短就好，不必完整。
            let reason: String = error.chars().take(120).collect();
            let _ = app.emit_to(
                "main",
                "capture://recognizing",
                format!("模型無法辨識圖片（{reason}），改用系統 OCR…"),
            );
            system_ocr(image).await
        }
        Err(error) => Err(error),
    }
}

async fn system_ocr(image: CapturedImage) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || recognize_text(&image))
        .await
        .map_err(|_| "文字辨識中斷".to_owned())?
        .map_err(|error| error.to_string())
}

/// 請模型把圖片裡的文字抄出來。
///
/// 刻意只抄字、不直接叫模型看圖給譯文：抄出來的文字要進面板的原文欄位，
/// 使用者才能修改後重譯、或要求解釋。少了這一步，圖片翻譯就會是唯一
/// 不能回頭修原文的路徑。
async fn transcribe_with_model(
    app: &tauri::AppHandle,
    image: &CapturedImage,
) -> Result<String, String> {
    let (profile, secret) = active_profile_with_secret(app)?;
    let base64 = png_base64(image).map_err(|error| error.to_string())?;
    let provider = build_provider(profile, secret)?;
    let request = TranslationRequest::transcribing(RequestImage {
        media_type: "image/png".into(),
        base64,
    });

    let collected = Arc::new(Mutex::new(String::new()));
    let sink_text = Arc::clone(&collected);
    let mut sink = move |event| {
        let TranslationEvent::Delta(delta) = event;
        if let Ok(mut text) = sink_text.lock() {
            text.push_str(&delta);
        }
    };
    provider
        .translate(&request, &mut sink)
        .await
        .map_err(|error| error.to_string())?;

    let text = collected
        .lock()
        .map_err(|_| "辨識結果無法讀取".to_owned())?
        .trim()
        .to_owned();
    Ok(text)
}

fn image_recognition_mode(app: &tauri::AppHandle) -> ImageRecognition {
    let Some(state) = app.try_state::<AppState>() else {
        return ImageRecognition::SystemOcr;
    };
    let Ok(data) = state.data.lock() else {
        return ImageRecognition::SystemOcr;
    };
    data.choice(PREF_IMAGE_RECOGNITION, ImageRecognition::SystemOcr.as_str())
        .map(|value| ImageRecognition::parse(&value))
        .unwrap_or(ImageRecognition::SystemOcr)
}

/// 目前選用的模型設定檔與金鑰，供圖片辨識使用。
///
/// 和 `profile_with_secret` 的差別只在來源：那個由前端指名 profile_id，
/// 這個從偏好設定回讀——截圖覆蓋層問不到主面板選了哪個模型。
fn active_profile_with_secret(
    app: &tauri::AppHandle,
) -> Result<(ModelProfile, Option<String>), String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "模型設定目前無法使用".to_owned())?;
    let data = state
        .data
        .lock()
        .map_err(|_| "模型設定目前無法使用".to_owned())?;

    let selected = data.choice(PREF_ACTIVE_PROFILE, "").unwrap_or_default();
    let profile = match data.model_profile(&selected) {
        Ok(Some(profile)) => profile,
        // 記住的那個設定檔被刪掉了就退回第一個，不要因此讓圖片辨識整個不能用。
        _ => data
            .model_profiles()
            .map_err(|error| error.to_string())?
            .into_iter()
            .next()
            .ok_or_else(|| "還沒有任何模型設定，請先在設定裡新增".to_owned())?,
    };
    let secret = match &profile.credential_key {
        Some(key) => data.model_secret(key).map_err(|error| error.to_string())?,
        None => None,
    };
    Ok((profile, secret))
}

#[tauri::command]
async fn translate_selection(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    profile_id: String,
    source_text: String,
    target_language: String,
) -> Result<(), String> {
    if source_text.trim().is_empty() {
        return Err("沒有可翻譯的文字".into());
    }

    let (profile, secret) = profile_with_secret(&state, &profile_id)?;

    let generation = state.translation_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let current_generation = Arc::clone(&state.translation_generation);
    let request = TranslationRequest::new(source_text.clone(), target_language.clone());
    app.emit(
        "translation://started",
        TranslationStarted {
            source_text,
            target_language,
        },
    )
    .map_err(|error| error.to_string())?;

    let provider = build_provider(profile, secret)?;

    let stream_app = app.clone();
    let mut sink = move |event| {
        if current_generation.load(Ordering::SeqCst) != generation {
            return;
        }
        let TranslationEvent::Delta(delta) = event;
        let _ = stream_app.emit("translation://delta", delta);
    };

    match provider.translate(&request, &mut sink).await {
        Ok(()) if state.translation_generation.load(Ordering::SeqCst) == generation => {
            app.emit("translation://completed", ())
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        Ok(()) => Ok(()),
        Err(error) if state.translation_generation.load(Ordering::SeqCst) == generation => {
            let message = error.to_string();
            let _ = app.emit("translation://failed", message.clone());
            Err(message)
        }
        Err(_) => Ok(()),
    }
}

#[tauri::command]
fn cancel_translation(state: tauri::State<'_, AppState>) {
    state.translation_generation.fetch_add(1, Ordering::SeqCst);
}

/// 針對目前這段原文另外要一份補充說明。刻意使用獨立的 generation 計數器：
/// 解釋不該取消進行中的翻譯，翻譯也不該取消解釋。
#[tauri::command]
async fn explain_translation(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    profile_id: String,
    source_text: String,
    target_language: String,
) -> Result<(), String> {
    if source_text.trim().is_empty() {
        return Err("沒有可解釋的文字".into());
    }

    let (profile, secret) = profile_with_secret(&state, &profile_id)?;
    let generation = state.explanation_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let current_generation = Arc::clone(&state.explanation_generation);
    let request = TranslationRequest::explaining(source_text, target_language);
    app.emit("explanation://started", ())
        .map_err(|error| error.to_string())?;

    let provider = build_provider(profile, secret)?;
    let stream_app = app.clone();
    let mut sink = move |event| {
        if current_generation.load(Ordering::SeqCst) != generation {
            return;
        }
        let TranslationEvent::Delta(delta) = event;
        let _ = stream_app.emit("explanation://delta", delta);
    };

    match provider.translate(&request, &mut sink).await {
        Ok(()) if state.explanation_generation.load(Ordering::SeqCst) == generation => {
            app.emit("explanation://completed", ())
                .map_err(|error| error.to_string())?;
            Ok(())
        }
        Ok(()) => Ok(()),
        Err(error) if state.explanation_generation.load(Ordering::SeqCst) == generation => {
            let message = error.to_string();
            let _ = app.emit("explanation://failed", message.clone());
            Err(message)
        }
        Err(_) => Ok(()),
    }
}

#[tauri::command]
fn cancel_explanation(state: tauri::State<'_, AppState>) {
    state.explanation_generation.fetch_add(1, Ordering::SeqCst);
}

#[tauri::command]
fn accept_pending_selection(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // 先收按鈕再取字。取不到時（連點兩下、或這段選取已經被下一次探測換掉）
    // 底下會提早返回，按鈕留在畫面上就變成一顆按了沒反應的鈕。
    if let Some(action) = app.get_webview_window("action") {
        let _ = action.hide();
    }
    let text = state
        .pending_selection
        .lock()
        // 鎖被毒化不該讓「譯」按鈕從此按不動：守著的只是一段文字，
        // 前一位持有者就算 panic 了，值仍然是完好的。
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
        .ok_or_else(|| "選取文字已失效，請重新選取".to_owned())?;
    show_panel_at_cursor(&app, true);
    app.emit_to("main", "capture://captured", text)
        .map_err(|error| error.to_string())
}

/// 「搜」按鈕用的搜尋網址前綴，查詢字串直接接在後面。
///
/// 寫死 Google 是因為它是目前唯一「哪個語言的使用者都認得」的預設值。要換成
/// 別家（Bing、DuckDuckGo、百度）只要改這一行——真正該做的是開成偏好設定，
/// 但那要連著設定畫面一起長，先不混進來。
const SEARCH_ENDPOINT: &str = "https://www.google.com/search?q=";

/// 網址長度的預算（以編碼後的位元組計）。
///
/// 使用者很可能整段文章圈起來，而瀏覽器與 ShellExecute 對網址長度都有上限
/// （實務上兩千出頭）。中日文一個字編出來就是九個字元，超得比想像中快，
/// 所以寧可截斷也不要整條開不起來。
const SEARCH_QUERY_BUDGET: usize = 1600;

/// 把選取的文字編成查詢字串。
///
/// 連續空白（含換行）先併成一個空格：選取的往往橫跨數行，原樣編出來是一串
/// `%0D%0A`，搜尋結果會因此變差。
fn search_query(text: &str) -> String {
    let mut query = String::new();
    for (index, word) in text.split_whitespace().enumerate() {
        let piece = format!("{}{}", if index == 0 { "" } else { "%20" }, percent_encode(word));
        if query.len() + piece.len() > SEARCH_QUERY_BUDGET {
            break;
        }
        query.push_str(&piece);
    }
    query
}

/// 百分號編碼。為了一個查詢參數引入 `url` crate 不划算，這裡只有一條規則：
/// RFC 3986 的 unreserved 字元原樣留著，其餘一律逐位元組轉成 %XX。
fn percent_encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(*byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// 交給系統的檔案關聯開啟網址，也就是使用者自己設的預設瀏覽器。
fn open_in_default_browser(url: &str) -> Result<(), String> {
    use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    use windows::core::PCWSTR;

    let operation = wide("open");
    let target = wide(url);
    // SAFETY: 兩個字串都以 NUL 收尾，且在呼叫結束前都還活著。
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute 沿用至今的老式約定：回傳值 <= 32 是錯誤碼，不是控制代碼。
    if result.0 as isize <= 32 {
        return Err(format!(
            "開啟瀏覽器失敗（錯誤碼 {}）",
            result.0 as isize
        ));
    }
    Ok(())
}

/// Rust 字串轉成 Win32 要的 NUL 結尾 UTF-16。
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 選取文字丟給系統的預設瀏覽器搜尋。
///
/// 和「譯」共用同一份 `pending_selection`：一樣先收按鈕再取字，理由見
/// `accept_pending_selection`。搜尋不叫出面板——使用者的注意力接下來在瀏覽器，
/// 這時再蓋一片譯文面板上去只是擋路。
#[tauri::command]
fn search_pending_selection(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if let Some(action) = app.get_webview_window("action") {
        let _ = action.hide();
    }
    let text = state
        .pending_selection
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
        .ok_or_else(|| "選取文字已失效，請重新選取".to_owned())?;
    let query = search_query(&text);
    if query.is_empty() {
        return Err("選取的內容沒有可搜尋的文字".to_owned());
    }
    open_in_default_browser(&format!("{SEARCH_ENDPOINT}{query}"))
}

/// 啟動時累積的提醒（例如快捷鍵被佔用），面板讀取後即清除。
#[tauri::command]
fn startup_notice(state: tauri::State<'_, AppState>) -> Option<String> {
    state
        .startup_notice
        .lock()
        .ok()
        .and_then(|mut notice| notice.take())
}

#[tauri::command]
fn begin_region_capture(app: tauri::AppHandle) -> Result<(), String> {
    show_region_overlay(&app)
}

#[tauri::command]
fn cancel_region_capture(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(overlay) = app.get_webview_window("region") {
        let _ = overlay.hide();
    }
    let was_visible = app
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .panel_visible_before_capture
                .lock()
                .ok()
                .map(|remembered| *remembered)
        })
        .unwrap_or(true);
    if was_visible {
        show_panel(&app, false);
    }
    Ok(())
}

#[tauri::command]
async fn capture_screen_region(
    app: tauri::AppHandle,
    selection: RegionSelection,
) -> Result<(), String> {
    let overlay = app
        .get_webview_window("region")
        .ok_or_else(|| "找不到截圖視窗".to_owned())?;
    let origin = overlay.outer_position().map_err(|error| error.to_string())?;
    overlay.hide().map_err(|error| error.to_string())?;

    let region = selection.to_screen_region(origin.x, origin.y);
    let captured = tauri::async_runtime::spawn_blocking(move || {
        // 覆蓋視窗要先真的從畫面上消失，否則會把自己的遮罩一起截進去。
        std::thread::sleep(std::time::Duration::from_millis(140));
        capture_region(region)
    })
    .await
    .map_err(|_| "截圖中斷".to_owned())?
    .map_err(|error| error.to_string())?;

    show_panel_at_cursor(&app, false);
    match recognize_captured_image(&app, captured).await {
        Ok(text) if !text.trim().is_empty() => app
            .emit_to("main", "capture://captured", text)
            .map_err(|error| error.to_string()),
        Ok(_) => app
            .emit_to("main", "capture://unavailable", "這個範圍沒有辨識到文字")
            .map_err(|error| error.to_string()),
        Err(message) => {
            let _ = app.emit_to("main", "capture://unavailable", message.clone());
            Err(message)
        }
    }
}

/// 辨識剪貼簿裡的圖片並送去翻譯。
///
/// 內建框選之外的另一條路：使用者可以用自己慣用的截圖工具（Win＋Shift＋S、
/// ShareX……）拍好圖再貼進來。對於「框選期間畫面會變動」的情況（影片、動畫、
/// 會自動關閉的選單）尤其有用——那些內容用內建框選根本來不及框。
///
/// 這裡不重新定位面板：使用者是在面板裡按 Ctrl＋V 的，把視窗挪到游標旁只會突兀。
#[tauri::command]
async fn translate_clipboard_image(app: tauri::AppHandle) -> Result<(), String> {
    let image = tauri::async_runtime::spawn_blocking(clipboard_image)
        .await
        .map_err(|_| "讀取剪貼簿圖片中斷".to_owned())?
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "剪貼簿裡沒有圖片。請先用截圖工具複製一張圖，再貼上。".to_owned())?;

    let text = recognize_captured_image(&app, image).await.inspect_err(|message| {
        let _ = app.emit_to("main", "capture://unavailable", message.clone());
    })?;

    if text.trim().is_empty() {
        let _ = app.emit_to("main", "capture://unavailable", "這張圖片沒有辨識到文字");
        return Ok(());
    }
    app.emit_to("main", "capture://captured", text)
        .map_err(|error| error.to_string())
}

/// 面板的「×」與 Esc 都走這裡。前端不能直接呼叫 window.hide()，
/// 那需要 core:window:allow-hide，而 core:default 並未授予。
#[tauri::command]
fn hide_panel_window(app: tauri::AppHandle) {
    hide_panel(&app);
}

/// 釘選只鎖住位置。面板本來就一直置頂，取消釘選不該讓它掉到來源視窗後面。
#[tauri::command]
fn set_panel_pinned(state: tauri::State<'_, AppState>, pinned: bool) -> Result<(), String> {
    *state
        .panel_pinned
        .lock()
        .map_err(|_| "釘選狀態目前無法使用".to_owned())? = pinned;
    Ok(())
}

/// 釘選狀態的真值在 Rust 這側。webview 重新載入時前端狀態會歸零，
/// 若不回讀就會和實際行為不一致（畫面顯示未釘選、實際卻鎖著）。
#[tauri::command]
fn panel_pinned(state: tauri::State<'_, AppState>) -> bool {
    state
        .panel_pinned
        .lock()
        .map(|pinned| *pinned)
        .unwrap_or(false)
}

/// 切換簡易模式。簡易模式只顯示譯文與複製按鈕，由「譯」按鈕與快捷鍵觸發；
/// 截圖翻譯與系統匣入口使用完整面板。切換時同步調整視窗尺寸。
#[tauri::command]
fn set_panel_mini(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    mini: bool,
) -> Result<(), String> {
    *state
        .panel_mini
        .lock()
        .map_err(|_| "簡易模式狀態目前無法使用".to_owned())? = mini;

    // 全螢幕時只記狀態、不動視窗，交給離開全螢幕時還原。
    if panel_is_fullscreen(&app) {
        return Ok(());
    }

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "找不到翻譯視窗".to_owned())?;
    // 同 set_panel_collapsed：螢幕要在改尺寸之前問，否則放大後溢出到隔壁螢幕，
    // 就會被夾到那台上面去。
    let monitor = window.current_monitor().map_err(|error| error.to_string())?;

    let (width, height) = if mini {
        PANEL_SIZE_MINI
    } else {
        PANEL_SIZE
    };
    window
        .set_size(tauri::LogicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    // 這條路也會從收合狀態展開（「回首頁」就是走這裡而不是 set_panel_collapsed），
    // 陰影要跟著回來，否則展開後的面板從此沒有陰影。
    apply_window_shadow(&window, false);

    // 切換尺寸後要把超出工作區的部分拉回來，避免面板跑到螢幕外。
    if let Some(monitor) = monitor {
        let work = monitor.work_area();
        let scale = monitor.scale_factor();
        let physical_width = (width * scale) as i32;
        let physical_height = (height * scale) as i32;
        let work_left = work.position.x;
        let work_top = work.position.y;
        let max_x = (work_left + work.size.width as i32 - physical_width - 16).max(work_left);
        let max_y = (work_top + work.size.height as i32 - physical_height - 16).max(work_top);
        let current = window
            .outer_position()
            .map_err(|error| error.to_string())?;
        window
            .set_position(tauri::PhysicalPosition::new(
                current.x.clamp(work_left, max_x),
                current.y.clamp(work_top, max_y),
            ))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// 全螢幕閱讀模式：長譯文在 480 寬的面板裡要捲很久，攤開整個螢幕看比較省事。
///
/// 交給 Tauri 的 set_fullscreen 處理，離開時它會自己還原成先前的大小與位置，
/// 不必自己記一份（自己記就得處理「全螢幕期間螢幕組態變了」這種情況）。
#[tauri::command]
fn set_panel_fullscreen(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    fullscreen: bool,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "找不到翻譯視窗".to_owned())?;
    window
        .set_fullscreen(fullscreen)
        .map_err(|error| error.to_string())?;
    *state
        .panel_fullscreen
        .lock()
        .map_err(|_| "全螢幕狀態目前無法使用".to_owned())? = fullscreen;
    Ok(())
}

#[tauri::command]
fn panel_fullscreen(state: tauri::State<'_, AppState>) -> bool {
    state
        .panel_fullscreen
        .lock()
        .map(|fullscreen| *fullscreen)
        .unwrap_or(false)
}

/// webview 重新載入時前端狀態會歸零，需回讀才能和實際行為一致。
#[tauri::command]
fn panel_mini(state: tauri::State<'_, AppState>) -> bool {
    state
        .panel_mini
        .lock()
        .map(|mini| *mini)
        .unwrap_or(false)
}

/// 依收合狀態開關視窗陰影。
///
/// Windows 上的無邊框視窗一旦要陰影，系統就會**在四邊加一圈 1px 的白框**
///（Tauri 對 `set_shadow` 的說明就是這麼寫的）。在 480×640 的完整面板上那圈白
/// 只是一道細邊；縮到 36×36 的圖示上，同樣一圈白就佔掉圖示邊長的相當比例，
/// 看起來就是圖示鑲了白邊、和圓角對不齊。
///
/// 所以陰影只給完整面板，收合時關掉——收合狀態自己用 CSS 的 box-shadow 撐起
/// 立體感，那道陰影畫在視窗內部，不需要系統幫忙。
fn apply_window_shadow(window: &tauri::WebviewWindow, collapsed: bool) {
    let _ = window.set_shadow(!collapsed);
}

#[tauri::command]
fn set_panel_collapsed(app: tauri::AppHandle, collapsed: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "找不到翻譯視窗".to_owned())?;
    // 先問螢幕再改尺寸。current_monitor() 是依視窗與各螢幕的重疊面積判斷的，
    // 小標籤從螢幕角落展開成完整面板時會大幅溢出到隔壁螢幕，改完尺寸再問就會
    // 查到隔壁那台，接著用它的工作區把面板夾過去——面板就跑到別的螢幕上了。
    let monitor = window.current_monitor().map_err(|error| error.to_string())?;

    // 展開回去要還原成收合前的版面尺寸。一律用 PANEL_SIZE 的話，簡易面板收合
    // 再展開就會變成 340×192 的版面塞在 420×540 的視窗裡。
    let (width, height) = if collapsed {
        COLLAPSED_SIZE
    } else if panel_is_mini(&app) {
        PANEL_SIZE_MINI
    } else {
        PANEL_SIZE
    };
    window
        .set_size(tauri::LogicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    apply_window_shadow(&window, collapsed);

    if let Some(monitor) = monitor {
        let work = monitor.work_area();
        let scale = monitor.scale_factor();
        let physical_width = (width * scale) as i32;
        let physical_height = (height * scale) as i32;
        let work_left = work.position.x;
        let work_top = work.position.y;
        let max_x = (work_left + work.size.width as i32 - physical_width - 16).max(work_left);
        let max_y = (work_top + work.size.height as i32 - physical_height - 16).max(work_top);
        let (x, y) = if collapsed {
            // 收合就停靠到右下角。
            (max_x, max_y)
        } else {
            // 展開時留在原地，只把超出工作區的部分拉回來。
            let current = window
                .outer_position()
                .map_err(|error| error.to_string())?;
            (
                current.x.clamp(work_left, max_x),
                current.y.clamp(work_top, max_y),
            )
        };
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// 把更新檢查的失敗翻成使用者做得了事的說法。
///
/// Tauri 對「端點回了 404 或不是合法的清單」一律吐同一句英文
/// （Could not fetch a valid release JSON from the remote）。使用者看到那句話
/// 只知道失敗，不知道是自己沒網路、還是發布端根本還沒放上 latest.json，
/// 而後者在剛接上更新頻道的那段期間幾乎是必然會遇到的狀況。
fn describe_update_failure(error: impl std::fmt::Display) -> String {
    let detail = error.to_string();
    if detail.contains("valid release JSON") || detail.contains("404") {
        return format!("更新來源上還沒有可用的版本清單（{detail}）");
    }
    detail
}

#[tauri::command]
async fn check_for_update(
    app: tauri::AppHandle,
    force: Option<bool>,
) -> Result<UpdateStatus, String> {
    let Some(updater) = configured_updater(&app)? else {
        return Ok(UpdateStatus::Disabled);
    };

    // 啟動時的自動檢查每 24 小時最多一次，避免每次開機都連線。
    // 手動點「檢查更新」傳 force=true 跳過節流。
    if !force.unwrap_or(false) {
        if let Some(last) = last_update_check(&app) {
            const MIN_INTERVAL_SECS: u64 = 24 * 60 * 60;
            if last.elapsed().map(|d| d.as_secs()).unwrap_or(0) < MIN_INTERVAL_SECS {
                return Ok(UpdateStatus::UpToDate {
                    current: env!("CARGO_PKG_VERSION").to_owned(),
                });
            }
        }
    }

    record_update_check(&app);
    match updater.check().await.map_err(describe_update_failure)? {
        Some(update) => Ok(UpdateStatus::Available {
            version: update.version,
            notes: update.body,
        }),
        None => Ok(UpdateStatus::UpToDate {
            current: env!("CARGO_PKG_VERSION").to_owned(),
        }),
    }
}

#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let updater = configured_updater(&app)?.ok_or_else(|| "此建置尚未設定更新頻道".to_owned())?;
    let update = updater
        .check()
        .await
        .map_err(describe_update_failure)?
        .ok_or_else(|| "目前已是最新版本".to_owned())?;
    let progress_app = app.clone();
    let installing_app = app.clone();
    update
        .download_and_install(
            move |chunk_length, content_length| {
                let _ = progress_app.emit(
                    "update://progress",
                    serde_json::json!({
                        "chunkLength": chunk_length,
                        "contentLength": content_length,
                    }),
                );
            },
            move || {
                // 下載完成、即將安裝。Windows NSIS 安裝程式會結束目前程式，
                // 這個事件讓前端可以顯示「即將重新啟動」。
                let _ = installing_app.emit("update://installing", ());
            },
        )
        .await
        .map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        // 必須是第一個註冊的外掛，這是 Tauri 對這個外掛的要求。
        //
        // 隨譯常駐系統匣、沒有工作列按鈕，使用者看不到它在不在跑，很容易重複點
        // 捷徑。多開的代價不只是多一個視窗：每個實例都會註冊同一組全域快捷鍵
        //（第二個註冊失敗，於是跳出誤導性的「快捷鍵被佔用」提示）、各自跑一條
        // 35ms 的滑鼠輪詢、各自放一個系統匣圖示，還會同時開同一個 SQLite 檔。
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // 這段在「既有的」實例裡執行，新啟動的行程隨即結束。
            // 使用者重複點捷徑的意圖就是把隨譯叫出來，所以直接顯示面板。
            show_panel(app, true);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            runtime_status,
            model_profiles,
            save_model_profile,
            translate_selection,
            cancel_translation,
            explain_translation,
            cancel_explanation,
            accept_pending_selection,
            search_pending_selection,
            startup_notice,
            begin_region_capture,
            cancel_region_capture,
            capture_screen_region,
            translate_clipboard_image,
            hide_panel_window,
            set_panel_pinned,
            panel_pinned,
            set_panel_mini,
            panel_mini,
            set_panel_fullscreen,
            panel_fullscreen,
            preferences,
            ui_locale,
            set_preference,
            set_choice_preference,
            active_model_profile,
            set_panel_collapsed,
            check_for_update,
            install_update
        ])
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data)?;
            let database = SqliteStore::open(app_data.join("floatrans.db"))?;
            let data = LocalData::new(
                database,
                WindowsCredentialStore::new("app.floatrans.desktop"),
            );
            // 設定檔視窗在 setup 之前就開始載入，所以 state 要盡早掛上；
            // 種子資料留到掛好之後再寫，別讓 webview 搶先呼叫指令時撲空。
            app.manage(AppState {
                data: Mutex::new(data),
                translation_generation: Arc::new(AtomicU64::new(0)),
                explanation_generation: Arc::new(AtomicU64::new(0)),
                pending_selection: Mutex::new(None),
                panel_visible_before_capture: Mutex::new(false),
                startup_notice: Mutex::new(None),
                panel_pinned: Mutex::new(false),
                panel_mini: Mutex::new(false),
                panel_shown_at: Mutex::new(None),
                panel_fullscreen: Mutex::new(false),
                tray: Mutex::new(None),
            });

            {
                let state = app.state::<AppState>();
                let mut data = state
                    .data
                    .lock()
                    .map_err(|_| "模型設定初始化失敗".to_owned())?;
                if data.model_profiles()?.is_empty() {
                    data.save_model_profile(
                        &ModelProfile {
                            id: "local-ollama".into(),
                            name: "本機 Ollama".into(),
                            provider: ProviderKind::OllamaNative,
                            endpoint: "http://127.0.0.1:11434".into(),
                            model: "qwen3:8b".into(),
                            credential_key: None,
                        },
                        None,
                    )?;
                }
            }

            start_passive_selection_watcher(app.handle().clone());

            // 系統匣要在前端載入之前就把文字擺好——使用者可能開機後直接按右下角的圖示，
            // 那時候主面板還沒被叫出來過，webview 一次都還沒跑。
            let startup_locale = startup_locale(app.handle());
            let menu = build_tray_menu(app.handle(), startup_locale)?;

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("application icon").clone())
                .tooltip(startup_locale.strings().tray_tooltip)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_panel(app, true),
                    "shot" => open_region_overlay(app),
                    "hide" => hide_panel(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_panel(tray.app_handle(), true);
                    }
                })
                .build(app)?;

            // 換語言時要拿它重建選單。選單項的文字改不了，只能整個換掉。
            if let Ok(mut slot) = app.state::<AppState>().tray.lock() {
                *slot = Some(tray);
            }

            if let Some(window) = app.get_webview_window("main") {
                let panel = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = panel.hide();
                    }
                });
            }

            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyT);
            let region_shortcut =
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyR);
            let handled_shortcut = shortcut.clone();
            let handled_region_shortcut = region_shortcut.clone();
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, pressed, event| {
                        if event.state() != ShortcutState::Pressed {
                            return;
                        }
                        if pressed == &handled_region_shortcut {
                            open_region_overlay(app);
                            return;
                        }
                        if pressed == &handled_shortcut {
                            let app = app.clone();
                            tauri::async_runtime::spawn_blocking(move || {
                                // 一定要先取字再顯示面板。面板是恆常置頂視窗，
                                // 顯示時會搶走前景：UI Automation 的「焦點元素」會變成面板本身，
                                // 剪貼簿備援模擬的 Ctrl+C 也會送到面板而不是來源程式，
                                // 於是任何應用程式都會取不到選取文字。
                                let capture = SelectionCapture::new(
                                    WindowsUiaSelectionReader,
                                    PreservingClipboardFallback,
                                );
                                let outcome =
                                    capture.capture(CaptureRequest::user_action("foreground"));
                                show_panel_at_cursor(&app, true);
                                match outcome {
                                    Ok(CaptureOutcome::Text(text)) => {
                                        let _ = app.emit("capture://captured", text.as_str());
                                    }
                                    Ok(CaptureOutcome::Excluded(reason)) => {
                                        let _ = app.emit(
                                            "capture://unavailable",
                                            format!("excluded: {reason:?}"),
                                        );
                                    }
                                    Ok(CaptureOutcome::Unavailable) => {
                                        let _ = app.emit(
                                            "capture://unavailable",
                                            "此應用程式無法取得選取文字",
                                        );
                                    }
                                    Err(_) => {
                                        let _ = app.emit(
                                            "capture://unavailable",
                                            "讀取選取文字時發生錯誤",
                                        );
                                    }
                                }
                            });
                        }
                    })
                    .build(),
            )?;
            // 快捷鍵可能已被其他程式佔用，這種情況只要提醒使用者，不該讓隨譯開不起來。
            let strings = startup_locale.strings();
            let mut occupied = Vec::new();
            if app.global_shortcut().register(shortcut).is_err() {
                occupied.push(strings.shortcut_translate);
            }
            if app.global_shortcut().register(region_shortcut).is_err() {
                occupied.push(strings.shortcut_capture);
            }
            if !occupied.is_empty()
                && let Ok(mut notice) = app.state::<AppState>().startup_notice.lock()
            {
                // 這句話會直接顯示在面板上，所以要跟著介面語言走。它在啟動當下就
                // 決定好了——之後換語言不會重算，但那是一次性的開機提示，
                // 為它做失效重算不值得。
                *notice = Some((strings.shortcuts_occupied)(
                    &occupied.join(strings.list_separator),
                ));
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Floatrans");
}

fn provider_name(provider: &ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::AzureOpenAi => "azure-openai",
        ProviderKind::GoogleGemini => "google-gemini",
        ProviderKind::OpenAiCompatible => "openai-compatible",
        ProviderKind::OpenRouter => "openrouter",
        ProviderKind::XAi => "xai",
        ProviderKind::OllamaNative => "ollama-native",
        ProviderKind::FedGpt => "fedgpt",
        ProviderKind::CustomEndpoint => "custom-endpoint",
    }
}

fn parse_provider(value: &str) -> Result<ProviderKind, String> {
    match value {
        "anthropic" => Ok(ProviderKind::Anthropic),
        "azure-openai" => Ok(ProviderKind::AzureOpenAi),
        "google-gemini" => Ok(ProviderKind::GoogleGemini),
        "openai-compatible" => Ok(ProviderKind::OpenAiCompatible),
        "openrouter" => Ok(ProviderKind::OpenRouter),
        "xai" => Ok(ProviderKind::XAi),
        "ollama-native" => Ok(ProviderKind::OllamaNative),
        "fedgpt" => Ok(ProviderKind::FedGpt),
        "custom-endpoint" => Ok(ProviderKind::CustomEndpoint),
        _ => Err("不支援的模型供應商".into()),
    }
}

fn show_panel(app: &tauri::AppHandle, focus: bool) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        if focus {
            let _ = window.set_focus();
        }
    }
}

fn hide_panel(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn panel_is_pinned(app: &tauri::AppHandle) -> bool {
    app.try_state::<AppState>()
        .and_then(|state| state.panel_pinned.lock().ok().map(|pinned| *pinned))
        .unwrap_or(false)
}

fn panel_is_mini(app: &tauri::AppHandle) -> bool {
    app.try_state::<AppState>()
        .and_then(|state| state.panel_mini.lock().ok().map(|mini| *mini))
        .unwrap_or(false)
}

/// 全螢幕期間所有改尺寸與位置的路徑都要讓開：新翻譯進來時若照常把視窗縮回
/// 小面板並挪到游標旁，使用者剛攤開的閱讀畫面就沒了。
fn panel_is_fullscreen(app: &tauri::AppHandle) -> bool {
    app.try_state::<AppState>()
        .and_then(|state| state.panel_fullscreen.lock().ok().map(|full| *full))
        .unwrap_or(false)
}

/// 把譯文面板挪到游標旁邊，讓譯文就出現在使用者正在看的地方。
/// 已釘選的面板留在原位。`mini` 決定用哪種尺寸算邊界。
fn position_panel_near_cursor(app: &tauri::AppHandle, mini: bool) {
    if panel_is_pinned(app) || panel_is_fullscreen(app) {
        return;
    }
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Some(cursor) = cursor_position() else {
        return;
    };
    let monitor = window
        .monitor_from_point(cursor.x as f64, cursor.y as f64)
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };

    let work = monitor.work_area();
    // 一律用展開後的尺寸算位置：呼叫這裡的路徑最後都會顯示譯文，
    // 而收合中的面板收到譯文就會自動展開。用當下的小尺寸會算錯邊界。
    let scale = monitor.scale_factor();
    let (logical_width, logical_height) = if mini {
        PANEL_SIZE_MINI
    } else {
        PANEL_SIZE
    };
    let panel_width = (logical_width * scale).round() as i32;
    let panel_height = (logical_height * scale).round() as i32;
    let (work_left, work_top) = (work.position.x, work.position.y);
    let work_right = work_left + work.size.width as i32;
    let work_bottom = work_top + work.size.height as i32;
    let margin = 14;

    // 預設放在游標右下；那一側放不下就翻到游標的另一邊。
    let mut x = cursor.x + margin;
    if x + panel_width > work_right {
        x = cursor.x - margin - panel_width;
    }
    let mut y = cursor.y + margin;
    if y + panel_height > work_bottom {
        y = cursor.y - margin - panel_height;
    }

    // 面板比工作區還大時，max 會小於 min，所以要先夾住上界再 clamp。
    x = x.clamp(work_left, (work_right - panel_width).max(work_left));
    y = y.clamp(work_top, (work_bottom - panel_height).max(work_top));
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

/// 譯文面板的共同進場流程：先就位，再顯示。
/// `mini` 控制是否以簡易模式顯示——簡易模式只顯示譯文與複製按鈕，
/// 由「譯」按鈕與快捷鍵觸發；截圖翻譯與系統匣入口使用完整面板。
fn show_panel_at_cursor(app: &tauri::AppHandle, mini: bool) {
    // 全螢幕閱讀時一律維持完整版面。簡易版面只有譯文與複製鈕，攤在整個螢幕上
    // 會是一大片空白。
    let mini = mini && !panel_is_fullscreen(app);
    set_panel_mini_state(app, mini);
    position_panel_near_cursor(app, mini);
    show_panel(app, false);
    // 記在顯示之後：自動收合檢查靠這個時間戳認出「面板是為了這次翻譯才開的」。
    if let Some(state) = app.try_state::<AppState>()
        && let Ok(mut shown_at) = state.panel_shown_at.lock()
    {
        *shown_at = Some(std::time::Instant::now());
    }
    // 通知前端切換模式，避免前後端狀態不同步
    let _ = app.emit_to("main", "panel://mini", mini);
}

/// 把簡易模式狀態寫進 AppState 並調整視窗尺寸，不重新定位。
fn set_panel_mini_state(app: &tauri::AppHandle, mini: bool) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut flag) = state.panel_mini.lock() {
            *flag = mini;
        }
    }
    if panel_is_fullscreen(app) {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let (width, height) = if mini {
            PANEL_SIZE_MINI
        } else {
            PANEL_SIZE
        };
        let _ = window.set_size(tauri::LogicalSize::new(width, height));
        apply_window_shadow(&window, false);
    }
}

/// 把框選用的覆蓋視窗攤開到整個桌面，並先收起會擋住畫面的隨譯視窗。
fn show_region_overlay(app: &tauri::AppHandle) -> Result<(), String> {
    let overlay = app
        .get_webview_window("region")
        .ok_or_else(|| "找不到截圖視窗".to_owned())?;
    let bounds = virtual_screen_bounds().ok_or_else(|| "無法取得螢幕範圍".to_owned())?;

    if let Some(action) = app.get_webview_window("action") {
        let _ = action.hide();
    }
    if let Some(panel) = app.get_webview_window("main") {
        let was_visible = panel.is_visible().unwrap_or(false);
        if let Some(state) = app.try_state::<AppState>()
            && let Ok(mut remembered) = state.panel_visible_before_capture.lock()
        {
            *remembered = was_visible;
        }
        let _ = panel.hide();
    }

    overlay
        .set_position(tauri::PhysicalPosition::new(bounds.x, bounds.y))
        .map_err(|error| error.to_string())?;
    overlay
        .set_size(tauri::PhysicalSize::new(bounds.width, bounds.height))
        .map_err(|error| error.to_string())?;
    overlay.show().map_err(|error| error.to_string())?;
    let _ = overlay.set_always_on_top(true);
    let _ = overlay.set_focus();
    Ok(())
}

fn open_region_overlay(app: &tauri::AppHandle) {
    if let Err(error) = show_region_overlay(app) {
        let _ = app.emit_to("main", "capture://unavailable", error);
        show_panel(app, true);
    }
}

/// 游標是否落在某個視窗的範圍內。
fn cursor_is_over(window: &tauri::WebviewWindow, cursor: &floatrans_capture::CursorPosition) -> bool {
    let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) else {
        return false;
    };
    cursor.x >= position.x
        && cursor.x < position.x + size.width as i32
        && cursor.y >= position.y
        && cursor.y < position.y + size.height as i32
}

/// 使用者點到面板以外的地方就把面板收起來。
///
/// 這裡刻意用滑鼠位置而不是視窗的 focus 事件：面板是以「不搶焦點」的方式顯示的，
/// 使用者從選取到看譯文可能完全沒點過它，focus 事件因此永遠不會觸發。
fn collapse_panel_if_clicked_away(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let enabled = state
        .data
        .lock()
        .ok()
        .and_then(|data| data.flag(PREF_AUTO_COLLAPSE, true).ok())
        .unwrap_or(true);
    if !enabled {
        return;
    }

    // 全螢幕是使用者為了讀長文才開的，點一下別處就收掉等於白開。
    if panel_is_fullscreen(app) {
        return;
    }

    // 面板剛為了這次翻譯開好就不要收。點「譯」按鈕的那一次放開左鍵同時觸發
    // 開面板與這裡的檢查，少了這道防線，面板會開好又立刻被收掉。
    let just_shown = state
        .panel_shown_at
        .lock()
        .ok()
        .and_then(|shown_at| *shown_at)
        .is_some_and(|shown_at| shown_at.elapsed() < PANEL_SHOW_GRACE);
    if just_shown {
        return;
    }

    let Some(panel) = app.get_webview_window("main") else {
        return;
    };
    if !panel.is_visible().unwrap_or(false) {
        return;
    }
    let Some(cursor) = cursor_position() else {
        return;
    };
    if cursor_is_over(&panel, &cursor) {
        return;
    }
    // 點在「譯」按鈕上代表正要開始新翻譯，收合會馬上被展開，白閃一次
    if let Some(action) = app.get_webview_window("action")
        && action.is_visible().unwrap_or(false)
        && cursor_is_over(&action, &cursor)
    {
        return;
    }

    // 收合狀態由前端持有，這裡只發事件，避免兩邊各記一份而不同步
    let _ = app.emit_to("main", "panel://auto-collapse", ());
}

/// 選取探測迴圈停跳這麼久就當作它卡死了，換一條新的。
///
/// 一輪探測正常是幾十毫秒，最慢的路徑（UIA 逾時）也就三秒多，
/// 所以這個門檻只會在真的卡住時才碰得到。
const WATCHER_STALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// 啟動選取探測，並配一條看門狗盯著它。
///
/// 探測迴圈只要停下來，「譯」按鈕就從此不再出現，使用者除了關掉重開沒有別的辦法——
/// 而它會停下來的原因不只一種：呼叫某個沒回應的程式、等主執行緒回覆視窗狀態、
/// 或是迴圈裡任何一處 panic。與其逐一堵死，不如讓它可以自己重來。
fn start_passive_selection_watcher(app: tauri::AppHandle) {
    let heartbeat = Arc::new(AtomicU64::new(0));
    let generation = Arc::new(AtomicU64::new(0));
    spawn_selection_loop(
        app.clone(),
        Arc::clone(&heartbeat),
        Arc::clone(&generation),
        0,
    );

    std::thread::spawn(move || {
        let mut last_beat = heartbeat.load(Ordering::SeqCst);
        let mut last_moved = std::time::Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let beat = heartbeat.load(Ordering::SeqCst);
            if beat != last_beat {
                last_beat = beat;
                last_moved = std::time::Instant::now();
                continue;
            }
            if last_moved.elapsed() < WATCHER_STALL_TIMEOUT {
                continue;
            }
            // 換一條新的。卡住的那條若日後醒來，會發現世代編號已經不是自己的
            // 而自行結束，不會和新的那條搶著顯示「譯」按鈕。
            let next = generation.fetch_add(1, Ordering::SeqCst) + 1;
            spawn_selection_loop(
                app.clone(),
                Arc::clone(&heartbeat),
                Arc::clone(&generation),
                next,
            );
            last_beat = heartbeat.load(Ordering::SeqCst);
            last_moved = std::time::Instant::now();
        }
    });
}

/// 拖過這麼多像素才算「圈了一段字」，而不是點一下時手抖。
const DRAG_SELECTION_DISTANCE: i32 = 6;
/// 連點判定時，兩下之間容許的位移。滑鼠再穩也會晃個一兩格。
const MULTI_CLICK_SLOP: i32 = 4;

/// 這次放開左鍵，看起來像不像剛圈完一段字。
///
/// 只有像的時候才准動剪貼簿備援：備援會朝前景程式送一次 Ctrl+C，對每一次單擊
/// 都這麼做，等於使用者按每一顆按鈕時我們都插一腳。拖過一段距離、或短時間內
/// 在原地連點（兩下選詞、三下選段），才是真的在圈字。
///
/// `last_release` 是上一次放開左鍵的時間與位置，判完就換成這一次的——連點靠它
/// 串起來，所以不論判定結果如何都要更新。
fn looks_like_text_selection(
    pressed_at: Option<CursorPosition>,
    released_at: Option<CursorPosition>,
    last_release: &mut Option<(std::time::Instant, CursorPosition)>,
) -> bool {
    fn moved(a: CursorPosition, b: CursorPosition) -> i32 {
        (a.x - b.x).abs().max((a.y - b.y).abs())
    }

    let Some(released_at) = released_at else {
        return false;
    };
    let dragged =
        pressed_at.is_some_and(|pressed| moved(pressed, released_at) >= DRAG_SELECTION_DISTANCE);
    let repeated = last_release.is_some_and(|(at, position)| {
        at.elapsed() <= double_click_interval() && moved(position, released_at) <= MULTI_CLICK_SLOP
    });
    *last_release = Some((std::time::Instant::now(), released_at));
    dragged || repeated
}

/// 讀取「UIA 問不到時可以模擬 Ctrl+C」這個偏好。
///
/// 讀不到（state 還沒掛上、鎖被毒化）一律當成開啟：這是預設值，
/// 而讓取字在啟動初期悄悄失效，比偶爾多送一次 Ctrl+C 難查得多。
fn clipboard_fallback_enabled(app: &tauri::AppHandle) -> bool {
    let Some(state) = app.try_state::<AppState>() else {
        return true;
    };
    let Ok(data) = state.data.lock() else {
        return true;
    };
    data.flag(PREF_CLIPBOARD_FALLBACK, true).unwrap_or(true)
}

fn spawn_selection_loop(
    app: tauri::AppHandle,
    heartbeat: Arc<AtomicU64>,
    generation: Arc<AtomicU64>,
    mine: u64,
) {
    std::thread::spawn(move || {
        let mut was_down = false;
        let mut pressed_at: Option<CursorPosition> = None;
        let mut last_release: Option<(std::time::Instant, CursorPosition)> = None;
        loop {
            // 看門狗已經換人了：這條是卡住又醒來的舊迴圈，就此收工。
            if generation.load(Ordering::SeqCst) != mine {
                return;
            }
            heartbeat.fetch_add(1, Ordering::SeqCst);
            let is_down = left_mouse_button_is_down();
            if !was_down && is_down {
                pressed_at = cursor_position();
            }
            if was_down && !is_down {
                // 位置要趁現在記：底下的等待加起來至少 110 毫秒，走到剪貼簿備援
                // 還會再久一些，那時游標早已移開，拿它既判不出有沒有拖曳，
                //「譯」按鈕也會冒在離選取內容很遠的地方。
                let released_at = cursor_position();
                let gesture = looks_like_text_selection(pressed_at, released_at, &mut last_release);
                // 先讓這次點擊送達 Rust，面板才來得及在收合檢查之前開起來。
                // 少了這段等待，收合檢查會和 accept_pending_selection 搶快，
                // 誰先跑看運氣，面板就時好時壞地開了又被收掉。
                // 兩段加起來仍是放開左鍵後 110 毫秒才探測選取內容。
                std::thread::sleep(std::time::Duration::from_millis(80));
                collapse_panel_if_clicked_away(&app);
                std::thread::sleep(std::time::Duration::from_millis(30));
                // 前景是隨譯自己就別探測。原文欄位可編輯之後，使用者會在面板裡
                // 選字，探測會把譯文或自己剛貼上的原文當成新的選取內容，在面板
                // 上疊一顆「譯」按鈕，點下去等於翻譯自己。
                if foreground_is_own_process() {
                    if let Some(action) = app.get_webview_window("action") {
                        let _ = action.hide();
                    }
                    was_down = is_down;
                    std::thread::sleep(std::time::Duration::from_millis(35));
                    continue;
                }
                let capture =
                    SelectionCapture::new(WindowsUiaSelectionReader, PreservingClipboardFallback);
                // 圈完字才准走剪貼簿備援。UIA 問得到的照舊每次都問——那條路不碰
                // 別人的程式，多問幾次沒有代價。
                let request = if gesture && clipboard_fallback_enabled(&app) {
                    CaptureRequest::selection_gesture("foreground")
                } else {
                    CaptureRequest::probe("foreground")
                };
                match capture.capture(request) {
                    Ok(CaptureOutcome::Text(text)) => {
                        if let Some(state) = app.try_state::<AppState>() {
                            let mut pending = state
                                .pending_selection
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            *pending = Some(text.as_str().to_owned());
                        }
                        if let (Some(action), Some(position)) = (
                            app.get_webview_window("action"),
                            released_at.or_else(cursor_position),
                        ) {
                            let _ = action.set_position(tauri::PhysicalPosition::new(
                                position.x.saturating_add(12),
                                position.y.saturating_add(12),
                            ));
                            let _ = action.show();
                        }
                    }
                    _ => {
                        if let Some(action) = app.get_webview_window("action") {
                            let _ = action.hide();
                        }
                    }
                }
            }
            was_down = is_down;
            std::thread::sleep(std::time::Duration::from_millis(35));
        }
    });
}

fn configured_updater(
    app: &tauri::AppHandle,
) -> Result<Option<tauri_plugin_updater::Updater>, String> {
    let (Some(endpoint), Some(public_key)) = (
        option_env!("FLOATRANS_UPDATE_ENDPOINT"),
        option_env!("FLOATRANS_UPDATE_PUBKEY"),
    ) else {
        return Ok(None);
    };
    let endpoint = endpoint
        .parse()
        .map_err(|_| "更新網址格式無效".to_owned())?;
    app.updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|error| error.to_string())?
        .pubkey(public_key)
        .build()
        .map(Some)
        .map_err(|error| error.to_string())
}

/// 上次自動檢查更新的時間。記在 app_data_dir 下的簡單文字檔，
/// 不進資料庫：這只是節流用的時間戳，遺失或損毀只會導致多檢查一次。
fn last_update_check(app: &tauri::AppHandle) -> Option<std::time::SystemTime> {
    use std::time::UNIX_EPOCH;
    let path = app.path().app_data_dir().ok()?.join("last-update-check");
    let content = fs::read_to_string(&path).ok()?;
    let secs: u64 = content.trim().parse().ok()?;
    Some(UNIX_EPOCH + std::time::Duration::from_secs(secs))
}

fn record_update_check(app: &tauri::AppHandle) {
    use std::time::SystemTime;
    let Some(dir) = app.path().app_data_dir().ok() else {
        return;
    };
    let _ = fs::create_dir_all(&dir);
    let secs = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = fs::write(dir.join("last-update-check"), secs.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_survives_encoding_but_everything_else_gets_escaped() {
        assert_eq!(search_query("rust-lang"), "rust-lang");
        // 空白是分隔字，一律走 %20 而不是 +：`+` 只在 form-urlencoded 裡是空白。
        assert_eq!(search_query("hello world"), "hello%20world");
        assert_eq!(search_query("譯"), "%E8%AD%AF");
        // 查詢字串裡的 & 和 = 若原樣送出，會被當成參數分隔而截斷搜尋內容。
        assert_eq!(search_query("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn line_breaks_collapse_into_single_spaces() {
        assert_eq!(search_query("  one\r\n\ttwo   three  "), "one%20two%20three");
    }

    #[test]
    fn a_selection_with_nothing_but_whitespace_yields_no_query() {
        // 呼叫端靠這個空字串擋下「開一個沒有搜尋內容的分頁」。
        assert_eq!(search_query(" \r\n\t "), "");
    }

    #[test]
    fn overlong_selections_are_truncated_on_a_word_boundary() {
        // 中日文一個字編出來就是九個字元，整段文章圈起來很容易超過網址上限。
        let query = search_query(&"譯 ".repeat(1000));
        assert!(query.len() <= SEARCH_QUERY_BUDGET);
        // 截在詞與詞之間，不會留下半截 %E8 這種切壞的逸出序列。
        assert!(query.ends_with("%E8%AD%AF"));
    }
}
