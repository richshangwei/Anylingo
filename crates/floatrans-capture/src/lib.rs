use thiserror::Error;

pub mod ocr;
pub mod screen;

pub use ocr::tidy_ocr_text;
pub use screen::{CapturedImage, MIN_REGION_EDGE, ScreenRegion, png_base64, upscale_factor};

#[cfg(target_os = "windows")]
pub use ocr::recognize_text;
#[cfg(target_os = "windows")]
pub use screen::{capture_region, clipboard_image, virtual_screen_bounds};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedText(String);

impl SelectedText {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureRequest {
    pub application_id: String,
    pub trigger: CaptureTrigger,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureTrigger {
    /// 例行探測。使用者沒有表達任何意圖，只准讀，不准碰剪貼簿。
    PassiveProbe,
    /// 使用者剛做完像是圈字的滑鼠動作（拖曳或連點）。
    ///
    /// 這時 UIA 問不到文字，多半不是「沒有選取」，而是這個程式不透過 UIA
    /// 交代它的文字——Electron、Qt、Java、終端機、遊戲介面都是這樣。既然
    /// 使用者才剛圈完字，模擬一次 Ctrl+C 是合理的推測，也是這些程式唯一
    /// 問得到答案的方式。
    SelectionGesture,
    /// 使用者明確要求取字（快捷鍵、選單）。
    UserAction,
}

impl CaptureTrigger {
    /// UIA 問不到時，能不能改用模擬 Ctrl+C 的備援。
    ///
    /// 備援會朝別人的程式送按鍵，所以要有「使用者此刻確實想取字」的依據；
    /// 例行探測沒有這個依據。
    fn allows_clipboard(self) -> bool {
        matches!(self, Self::SelectionGesture | Self::UserAction)
    }
}

impl CaptureRequest {
    pub fn user_action(application_id: impl Into<String>) -> Self {
        Self {
            application_id: application_id.into(),
            trigger: CaptureTrigger::UserAction,
        }
    }

    pub fn selection_gesture(application_id: impl Into<String>) -> Self {
        Self {
            application_id: application_id.into(),
            trigger: CaptureTrigger::SelectionGesture,
        }
    }

    pub fn probe(application_id: impl Into<String>) -> Self {
        Self {
            application_id: application_id.into(),
            trigger: CaptureTrigger::PassiveProbe,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureOutcome {
    Text(SelectedText),
    Excluded(ExclusionReason),
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExclusionReason {
    PasswordField,
}

#[derive(Debug, Error)]
#[error("selection capture failed: {message}")]
pub struct CaptureError {
    message: String,
}

impl CaptureError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub trait SelectionReader {
    fn read_selection(&self) -> Result<SelectionRead, CaptureError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectionRead {
    pub text: Option<String>,
    pub is_password: bool,
}

impl SelectionRead {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            is_password: false,
        }
    }

    pub fn unavailable() -> Self {
        Self::default()
    }

    pub fn password_field() -> Self {
        Self {
            text: None,
            is_password: true,
        }
    }
}

pub trait ClipboardFallback {
    fn copy_selected_text(&self) -> Result<Option<String>, CaptureError>;
}

pub struct SelectionCapture<U, C> {
    selection_reader: U,
    clipboard_fallback: C,
}

impl<U, C> SelectionCapture<U, C>
where
    U: SelectionReader,
    C: ClipboardFallback,
{
    pub fn new(selection_reader: U, clipboard_fallback: C) -> Self {
        Self {
            selection_reader,
            clipboard_fallback,
        }
    }

    pub fn capture(&self, request: CaptureRequest) -> Result<CaptureOutcome, CaptureError> {
        let clipboard_allowed = request.trigger.allows_clipboard();
        let selection = match self.selection_reader.read_selection() {
            Ok(selection) => selection,
            // UIA 出錯只代表「這個程式問不出來」，不代表這次取字失敗。
            //
            // 自繪介面（Electron、Qt、Java）常在這裡直接回錯誤或乾脆不回應而
            // 逾時。舊版讓錯誤一路往外拋，備援因此根本沒機會執行——同一段選取
            // 用快捷鍵也一樣是「讀取選取文字時發生錯誤」。既然還有備援可走，
            // 就當作沒讀到，把機會留給它。
            Err(error) if clipboard_allowed => {
                trace(&format!(
                    "selection reader failed ({error}); trying clipboard"
                ));
                SelectionRead::unavailable()
            }
            Err(error) => return Err(error),
        };
        if selection.is_password {
            return Ok(CaptureOutcome::Excluded(ExclusionReason::PasswordField));
        }

        if let Some(text) = selection.text {
            return Ok(CaptureOutcome::Text(SelectedText::new(text)));
        }

        if !clipboard_allowed {
            return Ok(CaptureOutcome::Unavailable);
        }

        Ok(match self.clipboard_fallback.copy_selected_text()? {
            Some(text) => CaptureOutcome::Text(SelectedText::new(text)),
            None => CaptureOutcome::Unavailable,
        })
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsUiaSelectionReader;

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Debug, Default)]
pub struct PreservingClipboardFallback;

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

#[cfg(target_os = "windows")]
pub fn left_mouse_button_is_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000 != 0 }
}

/// 前景視窗是不是自己的行程。
///
/// 選取探測讀的是「焦點元素」，不分那是誰家的視窗。原文欄位可以編輯之後，
/// 使用者會在隨譯自己的面板裡選字，探測就會把譯文或自己剛貼上的原文當成新的
/// 選取內容，在面板上彈出「譯」按鈕，點下去等於翻譯自己。
#[cfg(target_os = "windows")]
pub fn foreground_is_own_process() -> bool {
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_invalid() {
        return false;
    }
    let mut process = 0u32;
    unsafe { GetWindowThreadProcessId(foreground, Some(&mut process)) };
    process != 0 && process == unsafe { GetCurrentProcessId() }
}

/// 系統設定的連點判定間隔。
///
/// 選取探測靠它分辨「連點兩下選詞」和「兩次無關的單擊」。這個值使用者可以在
/// 控制台調整，寫死一個數字會讓調過的人得到不一樣的行為。
#[cfg(target_os = "windows")]
pub fn double_click_interval() -> std::time::Duration {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
    std::time::Duration::from_millis(unsafe { GetDoubleClickTime() } as u64)
}

#[cfg(target_os = "windows")]
pub fn cursor_position() -> Option<CursorPosition> {
    use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some(CursorPosition {
        x: point.x,
        y: point.y,
    })
}

#[cfg(target_os = "windows")]
impl ClipboardFallback for PreservingClipboardFallback {
    fn copy_selected_text(&self) -> Result<Option<String>, CaptureError> {
        copy_selection_preserving_clipboard()
    }
}

#[cfg(target_os = "windows")]
impl SelectionReader for WindowsUiaSelectionReader {
    fn read_selection(&self) -> Result<SelectionRead, CaptureError> {
        uia_worker::read_selection()
    }
}

/// 讀取選取內容用的常駐 UI Automation 執行緒。
///
/// 舊版是每次探測都 `std::thread::spawn` 一條新執行緒，在裡面 `UIAutomation::new()`
/// 讀完就結束，呼叫端 `join()` 等它。
///
/// 真正的問題是那個沒有上限的 `join()`：UIA 是跨行程呼叫，對方沒回應時不會自己
/// 返回。只要碰上一次，整條選取探測迴圈就永遠停在那裡，「譯」按鈕從此不再出現，
/// 使用者只能關掉重開。
///
/// 順帶修掉的還有 COM 的生命週期：`UIAutomation::new()` 會 `CoInitializeEx`，
/// 但那條執行緒結束前沒有人 `CoUninitialize`，等於每按放一次滑鼠左鍵就少收一次尾。
///（實測 400 次探測看不出控制代碼成長，所以它不是使用者遇到的那個症狀的成因，
/// 但既然是配對不完整的呼叫，就一起處理。）
///
/// 改成一條常駐執行緒：COM 初始化與收尾各一次，呼叫端等回覆有逾時，逾時就把這條
/// 執行緒棄置、下次探測重開一條，不必重開隨譯。
#[cfg(target_os = "windows")]
mod uia_worker {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{
            Mutex, MutexGuard, OnceLock,
            mpsc::{Sender, SyncSender, channel, sync_channel},
        },
        time::Duration,
    };

    use uiautomation::{
        UIAutomation, UIElement,
        patterns::{UITextChildPattern, UITextPattern},
    };
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

    use super::{CaptureError, SelectionRead, trace};

    /// 一次探測最多等這麼久。正常情況是幾十毫秒，會用到這個上限就代表對方
    /// 應用程式沒有回應——放掉這一次沒關係，但不能讓選取探測跟著卡死。
    const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

    type Answer = Result<SelectionRead, CaptureError>;
    /// 呼叫端連同請求一起遞進去的回郵地址。
    type Reply = SyncSender<Answer>;

    fn worker() -> &'static Mutex<Option<Sender<Reply>>> {
        static WORKER: OnceLock<Mutex<Option<Sender<Reply>>>> = OnceLock::new();
        WORKER.get_or_init(|| Mutex::new(None))
    }

    /// 鎖被毒化也要能繼續用：這裡守的只是一個 Sender，前一位持有者就算 panic 了，
    /// 裡面的值仍然是完好的，而選取探測不該因此永久失效。
    fn lock() -> MutexGuard<'static, Option<Sender<Reply>>> {
        worker()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(super) fn read_selection() -> Answer {
        let (reply, answer) = sync_channel(1);
        if !dispatch(reply) {
            return Err(CaptureError::new("UI Automation worker unavailable"));
        }
        match answer.recv_timeout(PROBE_TIMEOUT) {
            Ok(answer) => answer,
            Err(_) => {
                // 卡在某個沒回應的程式上了。棄置這條執行緒（它日後醒來會發現
                // 沒人收件而自行收工），下次探測開一條新的。
                trace("UI Automation probe timed out; retiring worker");
                *lock() = None;
                Err(CaptureError::new("UI Automation timed out"))
            }
        }
    }

    /// 把這次探測交給常駐執行緒。上一條已經收工（逾時被棄置）就重開一條再送一次。
    fn dispatch(reply: Reply) -> bool {
        let mut worker = lock();
        for _ in 0..2 {
            if worker.get_or_insert_with(spawn).send(reply.clone()).is_ok() {
                return true;
            }
            *worker = None;
        }
        false
    }

    fn spawn() -> Sender<Reply> {
        let (jobs, inbox) = channel::<Reply>();
        std::thread::spawn(move || {
            // COM 的生命週期自己管：`UIAutomation::new()` 會 CoInitializeEx 卻沒有
            // 對應的收尾，所以改用 `new_direct()`，初始化與 CoUninitialize 都在這裡配對。
            if let Err(error) = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok() {
                let message = format!("COM 初始化失敗：{error}");
                while let Ok(reply) = inbox.recv() {
                    let _ = reply.send(Err(CaptureError::new(message.clone())));
                }
                return;
            }

            // 建立失敗多半是暫時的（例如系統正忙），留到下次請求再試一次，
            // 不要讓這條執行緒從此只會回錯誤。
            let mut automation: Option<UIAutomation> = None;
            while let Ok(reply) = inbox.recv() {
                if automation.is_none() {
                    match UIAutomation::new_direct() {
                        Ok(created) => automation = Some(created),
                        Err(error) => {
                            let _ = reply.send(Err(CaptureError::new(error.to_string())));
                            continue;
                        }
                    }
                }
                let client = automation.as_ref().expect("automation client");
                let answer = catch_unwind(AssertUnwindSafe(|| read(client)))
                    .unwrap_or_else(|_| Err(CaptureError::new("UI Automation worker panicked")));
                let _ = reply.send(answer);
            }

            drop(automation);
            unsafe { CoUninitialize() };
        });
        jobs
    }

    /// 沿祖先往上找文字供應者時最多走幾層。
    ///
    /// 再往上就是視窗本身，那裡的「選取」和使用者剛剛圈起來的那一段已經沒有
    /// 關係了，問到的多半是別處殘留的選取，反而會冒出一顆莫名其妙的「譯」。
    const ANCESTOR_LIMIT: usize = 3;

    fn read(automation: &UIAutomation) -> Answer {
        let focused = automation
            .get_focused_element()
            .map_err(|error| CaptureError::new(error.to_string()))?;

        // 屬性讀不到就當作不是密碼欄。這個查詢在自繪介面上失敗得很平常，
        // 讓它擋掉整次取字，等於整個程式都不能翻；而密碼欄的內容本來就不會
        // 從 TextPattern 出來，放行的代價遠小於誤擋。
        if focused.is_password().unwrap_or(false) {
            return Ok(SelectionRead::password_field());
        }

        // 三條路都問過，才算「這個程式問不出來」。
        if let Some(text) = selection_text(&focused) {
            trace("selection from focused element");
            return Ok(SelectionRead::text(text));
        }
        // Chromium 把 <input>/<textarea> 的鍵盤焦點落在葉節點上，選取範圍卻記在
        // 外層的文件供應者身上。只問焦點元素會拿到空的——這正是網頁輸入框裡選字
        // 不冒出「譯」按鈕，但同一頁的內文選字就可以的原因。
        // TextChildPattern 的用途就是從葉節點指回那個容器。
        if let Some(text) = text_container(&focused).as_ref().and_then(selection_text) {
            trace("selection from text-child container");
            return Ok(SelectionRead::text(text));
        }
        // 其他框架也有同樣的分層，只是不一定提供 TextChildPattern，就自己往上走。
        if let Some(text) = ancestor_selection(automation, &focused) {
            trace("selection from ancestor text provider");
            return Ok(SelectionRead::text(text));
        }

        trace(&format!("no UIA selection ({})", describe(&focused)));
        Ok(SelectionRead::unavailable())
    }

    /// 從這個元素的 TextPattern 讀出選取文字。
    ///
    /// 沒有這個模式、問不到選取、或選到的全是空白，一律回 None——對呼叫端來說
    /// 都是同一件事：這裡沒有答案，換下一條路。
    fn selection_text(element: &UIElement) -> Option<String> {
        let ranges = element
            .get_pattern::<UITextPattern>()
            .ok()?
            .get_selection()
            .ok()?;
        let text = ranges
            .into_iter()
            .filter_map(|range| range.get_text(-1).ok())
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        (!text.is_empty()).then_some(text)
    }

    fn text_container(element: &UIElement) -> Option<UIElement> {
        element
            .get_pattern::<UITextChildPattern>()
            .ok()?
            .get_text_container()
            .ok()
    }

    fn ancestor_selection(automation: &UIAutomation, focused: &UIElement) -> Option<String> {
        // 走控制項檢視而不是原始檢視：原始檢視會把每一層裝飾用的容器都算進去，
        // 網頁上光是巢狀的 div 就能吃光這幾層預算，真正持有文字的那個祖先反而走不到。
        let walker = automation.get_control_view_walker().ok()?;
        let mut current = walker.get_parent(focused).ok()?;
        for _ in 0..ANCESTOR_LIMIT {
            if let Some(text) = selection_text(&current) {
                return Some(text);
            }
            current = walker.get_parent(&current).ok()?;
        }
        None
    }

    /// 給 trace 用的元素身分。取不到字時，要知道當時焦點在什麼東西上，
    /// 才判斷得出是哪一層沒交出文字。
    fn describe(element: &UIElement) -> String {
        let control = element
            .get_control_type()
            .map(|control| format!("{control:?}"))
            .unwrap_or_else(|_| "?".into());
        let framework = element.get_framework_id().unwrap_or_else(|_| "?".into());
        let class = element.get_classname().unwrap_or_else(|_| "?".into());
        format!("control={control} framework={framework} class={class}")
    }
}

/// 診斷輸出。設 FLOATRANS_TRACE=1 才會印，平常完全安靜。
/// 取字失敗時症狀都一樣（「無法取得選取文字」），沒有這個就只能瞎猜是哪一關卡住。
fn trace(message: &str) {
    if std::env::var_os("FLOATRANS_TRACE").is_some() {
        eprintln!("[capture] {message}");
    }
}

/// 剪貼簿裡原本放著什麼。決定備援能不能安全執行。
#[cfg(target_os = "windows")]
enum ClipboardBefore {
    /// 空的：複製完清掉即可還原原狀。
    Empty,
    /// 每個格式的原始位元組，可以逐一放回去。
    ///
    /// 只保留純文字是不夠的：實務上幾乎沒有「純文字」剪貼簿——瀏覽器會附
    /// HTML Format、Word 附 RTF、.NET 附 System.String。只還原文字等於默默
    /// 吃掉使用者的格式資料，因此改成整份保存。
    Formats(Vec<(u32, Vec<u8>)>),
    /// 含有無法以位元組複製的控制代碼（點陣圖、metafile、延遲繪製）：一律不動。
    Unpreservable,
}

#[cfg(target_os = "windows")]
fn copy_selection_preserving_clipboard() -> Result<Option<String>, CaptureError> {
    use std::{
        thread,
        time::{Duration, Instant},
    };
    use windows::Win32::{
        Foundation::{HANDLE, HGLOBAL, HWND},
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
                GetClipboardOwner, GetClipboardSequenceNumber, OpenClipboard, SetClipboardData,
            },
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
        },
        UI::{
            Input::KeyboardAndMouse::{
                INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
                VIRTUAL_KEY, VK_CONTROL,
            },
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        },
    };

    /// 複製是否由前景那個程式做的。
    ///
    /// 不能直接比對 HWND：像 Notepad++ 這種以 Scintilla 子控制項執行複製的程式，
    /// 剪貼簿擁有者會是子視窗或隱藏的輔助視窗，永遠不等於前景的頂層視窗。
    /// 比對行程才涵蓋得到這些情況。
    fn owned_by(owner: HWND, foreground: HWND) -> bool {
        if owner == foreground {
            return true;
        }
        if owner.is_invalid() {
            return false;
        }
        let (mut owner_process, mut foreground_process) = (0u32, 0u32);
        unsafe { GetWindowThreadProcessId(owner, Some(&mut owner_process)) };
        unsafe { GetWindowThreadProcessId(foreground, Some(&mut foreground_process)) };
        owner_process != 0 && owner_process == foreground_process
    }

    const CF_TEXT: u32 = 1;
    const CF_OEMTEXT: u32 = 7;
    const CF_UNICODETEXT: u32 = 13;
    const CF_LOCALE: u32 = 16;

    struct ClipboardGuard;
    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseClipboard();
            }
        }
    }

    fn open_clipboard() -> Result<ClipboardGuard, CaptureError> {
        for _ in 0..5 {
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Ok(ClipboardGuard);
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        Err(CaptureError::new("clipboard is busy"))
    }

    /// 讀出目前剪貼簿的 Unicode 文字。必須在剪貼簿已開啟的狀態下呼叫。
    fn read_text() -> Option<String> {
        let handle = unsafe { GetClipboardData(CF_UNICODETEXT) }.ok()?;
        let global = HGLOBAL(handle.0);
        let pointer = unsafe { GlobalLock(global) } as *const u16;
        if pointer.is_null() {
            return None;
        }
        let mut length = 0;
        while unsafe { *pointer.add(length) } != 0 {
            length += 1;
        }
        let text = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, length) });
        unsafe {
            let _ = GlobalUnlock(global);
        }
        Some(text)
    }

    /// 這些格式的控制代碼不是記憶體區塊（GDI 物件、metafile），
    /// 無法用位元組複製的方式保存。
    fn is_handle_format(format: u32) -> bool {
        const CF_BITMAP: u32 = 2;
        const CF_METAFILEPICT: u32 = 3;
        const CF_PALETTE: u32 = 9;
        const CF_ENHMETAFILE: u32 = 14;
        const CF_OWNERDISPLAY: u32 = 0x0080;
        const CF_DSPBITMAP: u32 = 0x0082;
        const CF_DSPMETAFILEPICT: u32 = 0x0083;
        const CF_DSPENHMETAFILE: u32 = 0x008E;
        matches!(
            format,
            CF_BITMAP
                | CF_METAFILEPICT
                | CF_PALETTE
                | CF_ENHMETAFILE
                | CF_OWNERDISPLAY
                | CF_DSPBITMAP
                | CF_DSPMETAFILEPICT
                | CF_DSPENHMETAFILE
        )
    }

    /// 逐一保存剪貼簿上每個格式的原始位元組。必須在剪貼簿已開啟時呼叫。
    fn inspect() -> ClipboardBefore {
        // CF_TEXT / CF_OEMTEXT / CF_LOCALE 由系統從 CF_UNICODETEXT 自動合成，
        // 還原 CF_UNICODETEXT 後會自己再生，不必也不該自己塞回去。
        const SYNTHESIZED: [u32; 3] = [CF_TEXT, CF_OEMTEXT, CF_LOCALE];

        let mut saved = Vec::new();
        let mut format = 0u32;
        loop {
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            if is_handle_format(format) {
                return ClipboardBefore::Unpreservable;
            }
            if SYNTHESIZED.contains(&format) {
                continue;
            }
            let Ok(handle) = (unsafe { GetClipboardData(format) }) else {
                // 延遲繪製：資料要等到有人索取才產生，這裡拿不到就無法保證還原
                return ClipboardBefore::Unpreservable;
            };
            let global = HGLOBAL(handle.0);
            let size = unsafe { GlobalSize(global) };
            let pointer = unsafe { GlobalLock(global) } as *const u8;
            if pointer.is_null() || size == 0 {
                return ClipboardBefore::Unpreservable;
            }
            let bytes = unsafe { std::slice::from_raw_parts(pointer, size) }.to_vec();
            unsafe {
                let _ = GlobalUnlock(global);
            }
            saved.push((format, bytes));
        }

        if saved.is_empty() {
            ClipboardBefore::Empty
        } else {
            ClipboardBefore::Formats(saved)
        }
    }

    /// 把保存下來的位元組原樣放回剪貼簿。必須在剪貼簿已開啟時呼叫。
    fn write_formats(saved: &[(u32, Vec<u8>)]) -> Result<(), CaptureError> {
        unsafe { EmptyClipboard() }.map_err(|error| CaptureError::new(error.to_string()))?;
        for (format, bytes) in saved {
            let Ok(global) = (unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) }) else {
                continue;
            };
            let pointer = unsafe { GlobalLock(global) } as *mut u8;
            if pointer.is_null() {
                continue;
            }
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len()) };
            unsafe {
                let _ = GlobalUnlock(global);
            }
            // 成功之後記憶體所有權歸系統，不可自行釋放
            let _ = unsafe { SetClipboardData(*format, Some(HANDLE(global.0))) };
        }
        Ok(())
    }

    let foreground = unsafe { GetForegroundWindow() };
    if foreground == HWND::default() {
        return Ok(None);
    }

    /// 把使用者原本的剪貼簿放回去。
    ///
    /// **只有在確定剪貼簿上那份東西是我們這一下 Ctrl+C 造成的時候才可以呼叫。**
    /// 呼叫端要先確認真的取到文字了；取不到就代表兩件事之一：剪貼簿根本沒被動過
    /// （沒東西要還原），或者是**別人**寫進去的（那份內容不是我們的，蓋掉就是毀了它）。
    ///
    /// 後者不是理論上的顧慮，是實際發生過的：使用者按 Win+Shift+S 框選截圖，
    /// 框選的拖曳同時觸發了取字備援，而截圖是在放開左鍵之後才落到剪貼簿上，
    /// 落點正好在我們等待複製結果的那 650 毫秒之內。舊版無論如何都還原，
    /// 於是剛截好的圖被上一次複製的文字取代——系統的截圖功能看起來就像壞了。
    ///
    /// 備份為空時的還原路徑是 `EmptyClipboard()`，破壞性更直接：它會把別人
    /// 剛放上去的東西清成空的。
    ///
    /// `expected` 是我們讀走文字那一刻的序號，對不上就放棄還原。「取到文字了」
    /// 不等於「board 上現在那份是我們放的」：使用者圈完字順手按的 Ctrl+C 可能比
    /// 我們晚一步落下，蓋在我們讀完之後。舊版照樣還原，於是他明明按了複製，貼出
    /// 來的卻是上一次的內容。剪貼簿開著的時候別人動不了，所以在這裡驗是可靠的。
    fn restore(saved: &Option<Vec<(u32, Vec<u8>)>>, expected: u32) {
        let Ok(_clipboard) = open_clipboard() else {
            return;
        };
        if unsafe { GetClipboardSequenceNumber() } != expected {
            trace("clipboard changed after our copy; skipping restore");
            return;
        }
        match saved {
            Some(formats) => {
                let _ = write_formats(formats);
            }
            None => unsafe {
                let _ = EmptyClipboard();
            },
        }
    }

    /// 使用者手上還按著修飾鍵嗎。
    fn modifier_is_down() -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        };
        let down =
            |key: VIRTUAL_KEY| unsafe { GetAsyncKeyState(key.0 as i32) } as u16 & 0x8000 != 0;
        down(VK_CONTROL) || down(VK_MENU) || down(VK_SHIFT) || down(VK_LWIN) || down(VK_RWIN)
    }

    /// 等使用者放開手上的修飾鍵。
    ///
    /// 快捷鍵是在「按下」時就觸發的，此刻 Ctrl+Alt+T 三個鍵通常都還按著。
    /// 若立刻送出模擬的 Ctrl+C，來源程式收到的其實是 Ctrl+Alt+C，
    /// 絕大多數程式不會把它當成複製，於是備援永遠取不到文字。
    ///
    /// Ctrl 也要等。舊版特地不等它，理由是「我們本來就要送 Ctrl」——但我們送完
    /// 還要送 Ctrl 的**放開**，而 SendInput 的放開會改寫整個系統的鍵盤狀態：
    /// 使用者手指明明還按著 Ctrl，系統卻已認定它彈起來了。他接著按下 C，前景程式
    /// 收到的是沒有修飾鍵的純 c——可編輯的地方多一個字母，其他地方毫無反應，
    /// 得先放開 Ctrl 再按一次才會恢復。圈完字順手按 Ctrl+C 正是最常見的操作。
    fn wait_for_modifier_release() {
        let deadline = Instant::now() + Duration::from_millis(700);
        while Instant::now() < deadline && modifier_is_down() {
            thread::sleep(Duration::from_millis(20));
        }
        // 讓來源程式處理完放開的鍵，再送我們的按鍵
        thread::sleep(Duration::from_millis(40));
    }

    /// 等剪貼簿被前景那個程式換掉，把換上去的文字讀回來。
    ///
    /// 一併回報讀取當下的序號：還原之前要靠它確認 board 上那份仍是我們讀到的
    /// 那一份。
    fn await_copy(foreground: HWND, sequence_before: u32) -> Option<(String, u32)> {
        let deadline = Instant::now() + Duration::from_millis(650);
        while Instant::now() < deadline {
            let sequence = unsafe { GetClipboardSequenceNumber() };
            if sequence == sequence_before {
                thread::sleep(Duration::from_millis(15));
                continue;
            }

            let _clipboard = open_clipboard().ok()?;
            let owner = unsafe { GetClipboardOwner() }.unwrap_or_default();
            let matched = owned_by(owner, foreground);
            trace(&format!(
                "sequence changed; owner={owner:?} matches_foreground={matched}"
            ));
            if !matched || unsafe { GetClipboardSequenceNumber() } != sequence {
                return None;
            }
            let text = read_text();
            trace(&format!(
                "read {} chars",
                text.as_deref().map(str::len).unwrap_or(0)
            ));
            return text.map(|text| (text, sequence));
        }
        None
    }

    wait_for_modifier_release();

    // 等了 700 毫秒仍沒放開，就別送模擬按鍵了。送出去的 Ctrl 放開會把他按著的
    // 那個 Ctrl 一起關掉；Shift、Alt 還按著的話，我們這一下也不是 Ctrl+C，而是
    // Ctrl+Shift+C、Ctrl+Alt+C 之類的別的命令。
    //
    // 改成旁觀：他按著 Ctrl，多半就是正要自己複製。等那一下落到剪貼簿，讀走內容
    // 當作取字結果，然後什麼都不還原——board 上那份是他要留下來的東西。
    if modifier_is_down() {
        trace("modifier still held; observing the user's own copy instead of sending");
        let observed = await_copy(foreground, unsafe { GetClipboardSequenceNumber() });
        return Ok(observed
            .map(|(text, _)| text)
            .filter(|text| !text.trim().is_empty()));
    }

    // 快照要拖到這裡才取。放在等修飾鍵之前的話，那最多 740 毫秒也算進「快照可能
    // 過期」的窗口裡——使用者在那段期間複製的東西，事後完全看不出來。
    let (before, sequence_before) = {
        let _clipboard = open_clipboard()?;
        // 序號和快照在同一次開啟裡取。剪貼簿開著時誰也插不進來，兩者才確實指向
        // 同一份內容；日後序號和它對不上，就代表 board 上的不是我們保存的那份了。
        (inspect(), unsafe { GetClipboardSequenceNumber() })
    };
    let saved = match before {
        ClipboardBefore::Unpreservable => {
            trace("clipboard=Unpreservable -> refuse");
            return Ok(None);
        }
        ClipboardBefore::Empty => {
            trace("clipboard=Empty");
            None
        }
        ClipboardBefore::Formats(saved) => {
            trace(&format!("clipboard={} formats saved", saved.len()));
            Some(saved)
        }
    };

    let key = |virtual_key: VIRTUAL_KEY, flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virtual_key,
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let inputs = [
        key(VK_CONTROL, Default::default()),
        key(VIRTUAL_KEY(0x43), Default::default()),
        key(VIRTUAL_KEY(0x43), KEYEVENTF_KEYUP),
        key(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    trace(&format!("sent {sent}/{} inputs", inputs.len()));
    if sent != inputs.len() as u32 {
        // 按鍵沒送出去，剪貼簿就沒被我們動過，沒有東西要還原。
        return Ok(None);
    }

    let copied = await_copy(foreground, sequence_before);
    // 取到文字，才代表剪貼簿上那份是我們放的，該把使用者原本的內容換回去。
    //
    // 取不到就一律不碰：可能是剪貼簿根本沒動過（沒東西要還原），也可能是別人
    // 在這段期間寫了進去（Win+Shift+S 的截圖就是這樣落下來的）。分不出是哪一種
    // 的時候，「什麼都不做」是唯一不會毀掉別人資料的選項。
    match &copied {
        Some((_, sequence)) => restore(&saved, *sequence),
        None => trace("clipboard left untouched (nothing copied, or someone else wrote to it)"),
    }

    Ok(copied
        .map(|(text, _)| text)
        .filter(|text| !text.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedSelection(SelectionRead);

    impl SelectionReader for FixedSelection {
        fn read_selection(&self) -> Result<SelectionRead, CaptureError> {
            Ok(self.0.clone())
        }
    }

    struct FixedClipboard(Option<String>);

    impl ClipboardFallback for FixedClipboard {
        fn copy_selected_text(&self) -> Result<Option<String>, CaptureError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn ui_automation_selection_is_returned_without_clipboard_content() {
        let capture = SelectionCapture::new(
            FixedSelection(SelectionRead::text("selected text")),
            FixedClipboard(Some("unrelated clipboard".into())),
        );

        let outcome = capture.capture(CaptureRequest::user_action("notepad.exe"));

        assert_eq!(
            outcome.unwrap(),
            CaptureOutcome::Text(SelectedText::new("selected text"))
        );
    }

    #[test]
    fn explicit_user_action_uses_safe_clipboard_fallback_when_uia_is_unavailable() {
        let capture = SelectionCapture::new(
            FixedSelection(SelectionRead::unavailable()),
            FixedClipboard(Some("copied selection".into())),
        );

        let outcome = capture.capture(CaptureRequest::user_action("terminal.exe"));

        assert_eq!(
            outcome.unwrap(),
            CaptureOutcome::Text(SelectedText::new("copied selection"))
        );
    }

    #[test]
    fn password_fields_are_excluded_even_when_adapters_return_text() {
        let capture = SelectionCapture::new(
            FixedSelection(SelectionRead::password_field()),
            FixedClipboard(Some("secret".into())),
        );

        let outcome = capture.capture(CaptureRequest::user_action("vault.exe"));

        assert_eq!(
            outcome.unwrap(),
            CaptureOutcome::Excluded(ExclusionReason::PasswordField)
        );
    }

    #[test]
    fn passive_selection_probe_never_uses_the_clipboard_fallback() {
        let capture = SelectionCapture::new(
            FixedSelection(SelectionRead::unavailable()),
            FixedClipboard(Some("clipboard should stay untouched".into())),
        );

        let outcome = capture.capture(CaptureRequest::probe("browser.exe"));

        assert_eq!(outcome.unwrap(), CaptureOutcome::Unavailable);
    }

    struct FailingSelection;

    impl SelectionReader for FailingSelection {
        fn read_selection(&self) -> Result<SelectionRead, CaptureError> {
            Err(CaptureError::new("UI Automation timed out"))
        }
    }

    #[test]
    fn selection_gesture_uses_the_clipboard_fallback_when_uia_is_unavailable() {
        let capture = SelectionCapture::new(
            FixedSelection(SelectionRead::unavailable()),
            FixedClipboard(Some("copied selection".into())),
        );

        let outcome = capture.capture(CaptureRequest::selection_gesture("electron.exe"));

        assert_eq!(
            outcome.unwrap(),
            CaptureOutcome::Text(SelectedText::new("copied selection"))
        );
    }

    #[test]
    fn a_failing_selection_reader_still_falls_back_to_the_clipboard() {
        let capture = SelectionCapture::new(
            FailingSelection,
            FixedClipboard(Some("copied selection".into())),
        );

        for request in [
            CaptureRequest::user_action("qt-app.exe"),
            CaptureRequest::selection_gesture("qt-app.exe"),
        ] {
            assert_eq!(
                capture.capture(request).unwrap(),
                CaptureOutcome::Text(SelectedText::new("copied selection"))
            );
        }
    }

    #[test]
    fn a_failing_selection_reader_is_reported_when_no_fallback_is_allowed() {
        let capture = SelectionCapture::new(FailingSelection, FixedClipboard(None));

        assert!(
            capture
                .capture(CaptureRequest::probe("qt-app.exe"))
                .is_err()
        );
    }
}
