<script lang="ts">
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, tick } from "svelte";
  import {
    createTranslationPanelState,
    type TranslationPanelSnapshot
  } from "./lib/translation-panel-state";
  import { appNameEn, sealGlyph } from "./lib/branding";
  import { DEFAULT_LOCALE_ID, LOCALES, resolveLocale, type LocaleId } from "./locales";
  import Icon from "./lib/Icon.svelte";

  const windowLabel = "__TAURI_INTERNALS__" in window ? getCurrentWindow().label : "main";
  const isActionWindow = windowLabel === "action";
  const isRegionWindow = windowLabel === "region";

  /// 送出就好、不等回覆的指令一律走這裡。
  ///
  /// 被拒絕的 Promise 沒人接就會冒到 window 上，而 main.ts 的診斷守衛會把整個
  /// 介面換成錯誤畫面且回不去——例如連點兩下「譯」按鈕，第二下的
  /// accept_pending_selection 必然失敗，那顆按鈕就從此不再正常顯示，
  /// 使用者只能關掉重開。指令失敗頂多是這一次沒生效，不該弄壞介面。
  function fireAndForget(command: string, args?: Record<string, unknown>) {
    invoke(command, args).catch((error) => console.error(`[隨譯] ${command}`, error));
  }
  /**
   * 目前的介面語言。真值存在 Rust（偏好 `ui/locale`），這裡只是回讀的副本。
   *
   * 先給預設值再由 `loadPreferences()` 覆蓋，所以理論上有一瞬間會是繁體中文。
   * 實務上看不到：面板平常是隱藏的，要等使用者叫出來才顯示，而偏好在掛載當下
   * 就送出去問了。
   */
  let uiLocale: LocaleId = DEFAULT_LOCALE_ID;
  /**
   * 畫面上所有文字的來源。**元件裡不該再出現任何寫死的文案**——
   * 漏一個，那個角落就會在其他三個語言下維持中文。
   */
  $: locale = resolveLocale(uiLocale);
  $: t = locale.strings;
  $: appName = t.brand.name;
  /**
   * 標題列在產品名底下還有一行羅馬字商標。中文版是「隨譯／ANYLINGO」，
   * 英文版兩行都會是 Anylingo——同一個字疊兩次不是設計，是漏判，所以這裡要藏起來。
   */
  $: showWordmark = appName.toLowerCase() !== appNameEn.toLowerCase();
  // 斷行規則、標點間距與字型挑選都看 <html lang>。不跟著換的話，日文會套到
  // 中文的字形（同一個 Unicode 碼位在兩地的字形不同），英文則會用中文的斷行規則。
  $: if (typeof document !== "undefined") document.documentElement.lang = locale.id;

  const panel = createTranslationPanelState();
  let snapshot: TranslationPanelSnapshot = panel.snapshot();
  let sourceExpanded = false;
  // 原文的可編輯副本。翻譯開始時同步過來，使用者可以改幾個字重譯，
  // 也可以清空後貼上任意文字——不是每段想翻的文字都選得到。
  let sourceDraft = "";
  // 空狀態下的貼上模式。預設仍顯示原本的引導畫面，按「貼上文字翻譯」才切過來，
  // 否則輸入框會把空狀態撐出卷軸。
  let composing = false;
  // 預設值由 Rust 提供，前端不另外寫一份，避免兩邊不一致
  let showSourceByDefault = false;
  let autoCollapse = true;
  let clipboardFallback = true;
  let launchAtLogin = false;
  // 預設是唯一不會把畫面送出去的選項，實際值由 Rust 提供。
  let imageRecognition: "ocr" | "model" | "auto" = "ocr";
  // 由 Rust 的 runtime_status 填入。空字串代表還沒問到，首頁就先不顯示版號列，
  // 不要閃一個 "v" 出來。
  let appVersion = "";
  // 預設不釘選，面板才會跟著每次選取的位置走。
  let pinned = false;
  // 簡易模式：跟隨選取游標出現的精簡面板，只顯示譯文與複製按鈕。
  // 由「譯」按鈕與快捷鍵觸發；截圖翻譯與系統匣入口使用完整面板。
  let miniMode = false;
  let collapsed = false;
  // 全螢幕閱讀：長譯文在小面板裡要捲很久，攤開整個螢幕看比較省事。
  let fullscreen = false;
  let copied = false;
  type ProviderId =
    | "anthropic"
    | "azure-openai"
    | "google-gemini"
    | "openai-compatible"
    | "openrouter"
    | "xai"
    | "ollama-native"
    | "fedgpt"
    | "custom-endpoint";
  type ModelProfile = {
    id: string;
    name: string;
    provider: ProviderId;
    endpoint: string;
    model: string;
    hasCredential: boolean;
  };

  let profiles: ModelProfile[] = [];
  let modelProfile = "local-ollama";
  /**
   * 翻譯目標語言的清單，取自語言登錄表——加一個介面語言，它自然也成為可選的目標。
   *
   * 這些字串會**原樣送進提示詞**（模型收到的就是「繁體中文」「English」這種自然
   * 語言的語言名），所以它們不是介面文案，不隨介面語言改寫。英文介面下這個下拉
   * 選單依然顯示「日本語」而不是「Japanese」，因為顯示的就是要送出去的值。
   */
  const TARGET_LANGUAGES = [...new Set(LOCALES.map((entry) => entry.defaultTargetLanguage))];
  let targetLanguage = resolveLocale(DEFAULT_LOCALE_ID).defaultTargetLanguage;
  /**
   * 使用者動過目標語言沒有。沒動過就跟著介面語言走：選了英文介面卻預設翻成繁體
   * 中文，第一次用的人得先改一次下拉選單才會得到預期的結果。動過之後就以他選的
   * 為準，換介面語言不該把翻譯目標一起換掉。
   */
  let targetLanguageTouched = false;
  $: if (!targetLanguageTouched) targetLanguage = locale.defaultTargetLanguage;
  let captureNotice = "";
  let settingsOpen = false;
  let savingSettings = false;
  let settingsError = "";
  type UpdateStatus =
    | { state: "disabled" }
    | { state: "upToDate"; current: string }
    | { state: "available"; version: string; notes?: string };
  let updateStatus: UpdateStatus | null = null;
  let checkingUpdate = false;
  let updateError = "";
  $: availableUpdate = updateStatus?.state === "available" ? updateStatus : null;
  let installingUpdate = false;
  let updateProgress: { downloaded: number; total: number } | null = null;
  $: updateProgressPct = updateProgress && updateProgress.total > 0
    ? Math.min(100, Math.round((updateProgress.downloaded / updateProgress.total) * 100))
    : 0;
  let profileDraft = {
    id: "local-ollama",
    name: resolveLocale(DEFAULT_LOCALE_ID).strings.settings.defaultProfileName.ollama,
    provider: "ollama-native" as ModelProfile["provider"],
    endpoint: "http://127.0.0.1:11434",
    model: "qwen3:8b",
    apiKey: ""
  };

  /**
   * 新增設定檔時的預設值。名稱要跟著介面語言走，所以是反應式而不是常數。
   *
   * 只有三個是文案（本機 Ollama、公司內部 API、自訂端點），其餘都是廠商的商標，
   * 任何語言下都拼一樣，不進語言檔。
   *
   * 這裡只影響**新建**的設定檔。已經存到 SQLite 的名字是使用者的資料，
   * 換介面語言不會、也不該把它們改掉。
   */
  $: providerDefaults = {
    anthropic: { name: "Anthropic Claude", endpoint: "https://api.anthropic.com", model: "claude-sonnet-4-5" },
    "azure-openai": { name: "Azure OpenAI", endpoint: "https://YOUR-RESOURCE-NAME.openai.azure.com", model: "" },
    "google-gemini": { name: "Google Gemini", endpoint: "https://generativelanguage.googleapis.com", model: "gemini-3.5-flash" },
    "openai-compatible": { name: "OpenAI", endpoint: "https://api.openai.com", model: "gpt-5-mini" },
    openrouter: { name: "OpenRouter", endpoint: "https://openrouter.ai/api", model: "" },
    xai: { name: "xAI Grok", endpoint: "https://api.x.ai", model: "" },
    "ollama-native": { name: t.settings.defaultProfileName.ollama, endpoint: "http://127.0.0.1:11434", model: "qwen3:8b" },
    fedgpt: { name: t.settings.defaultProfileName.fedgpt, endpoint: "", model: "" },
    "custom-endpoint": { name: t.settings.defaultProfileName.custom, endpoint: "", model: "" }
  } satisfies Record<ProviderId, { name: string; endpoint: string; model: string }>;

  function selectProvider(provider: ProviderId) {
    const defaults = providerDefaults[provider];
    profileDraft = { ...profileDraft, provider, ...defaults, apiKey: "" };
  }

  /// 廠商名是商標，只有「金鑰」兩個字跟著語言換。
  function credentialLabel(provider: ProviderId) {
    if (provider === "anthropic") return `Anthropic ${t.common.apiKey}`;
    if (provider === "google-gemini") return `Gemini ${t.common.apiKey}`;
    if (provider === "azure-openai") return `Azure ${t.common.apiKey}`;
    return t.common.apiKey;
  }

  function providerNote(provider: ProviderId) {
    if (provider === "fedgpt") return t.settings.note.fedgpt;
    if (provider === "anthropic") return t.settings.note.anthropic;
    if (provider === "google-gemini") return t.settings.note.gemini;
    if (provider === "azure-openai") return t.settings.note.azure;
    if (provider === "custom-endpoint") return t.settings.note.custom;
    return t.settings.note.openaiCompatible;
  }

  type RegionRect = { x: number; y: number; width: number; height: number };
  let regionOrigin: { x: number; y: number } | null = null;
  let regionRect: RegionRect | null = null;

  let translationCard: HTMLElement | null = null;

  // 解釋是譯文之外的補充說明，獨立於翻譯串流，兩者不互相取消。
  let explanation = "";
  let explaining = false;
  let explanationError = "";
  // 一旦按過就保持顯示。只靠 explanation 判斷的話，模型回空字串時整塊會消失，
  // 使用者按了按鈕卻什麼都沒發生，也無從得知是成功還是失敗。
  let explanationRequested = false;
  // 模型常自帶 Markdown 粗體與標題記號，而這裡是純文字呈現，
  // 不清掉就會看到字面上的 ** 與 #。只處理成對的粗體與行首標題，不動其他字元。
  $: explanationText = explanation
    .replace(/\*\*(.+?)\*\*/g, "$1")
    .replace(/^\s{0,3}#{1,6}\s+/gm, "");

  async function explainTranslation() {
    if (!snapshot.sourceText || !modelProfile) return;
    explanation = "";
    explanationError = "";
    explanationRequested = true;
    explaining = true;
    void scrollToExplanation();
    try {
      await invoke("explain_translation", {
        profileId: modelProfile,
        sourceText: snapshot.sourceText,
        targetLanguage: targetLanguage
      });
    } catch (error) {
      explanationError = String(error);
      explaining = false;
    }
  }

  function dismissExplanation() {
    if (explaining) fireAndForget("cancel_explanation");
    explaining = false;
    explanation = "";
    explanationError = "";
    explanationRequested = false;
  }

  /// 解釋顯示在譯文下方，長譯文時會落在可視範圍外。
  /// 按了按鈕卻看不到東西等同沒反應，所以要捲過去。
  async function scrollToExplanation() {
    await tick();
    translationCard?.scrollTo({ top: translationCard.scrollHeight, behavior: "smooth" });
  }

  function refresh() {
    snapshot = panel.snapshot();
  }

  /// 換一段新譯文時要回到最上面，否則會停在上一段捲到的位置。
  async function resetTranslationScroll() {
    await tick();
    translationCard?.scrollTo({ top: 0 });
  }

  async function startRegionCapture() {
    captureNotice = "";
    try {
      await invoke("begin_region_capture");
    } catch (error) {
      captureNotice = String(error);
    }
  }

  function beginRegionDrag(event: PointerEvent) {
    if (event.button !== 0) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    regionOrigin = { x: event.clientX, y: event.clientY };
    regionRect = { x: event.clientX, y: event.clientY, width: 0, height: 0 };
  }

  function updateRegionDrag(event: PointerEvent) {
    if (!regionOrigin) return;
    regionRect = {
      x: Math.min(regionOrigin.x, event.clientX),
      y: Math.min(regionOrigin.y, event.clientY),
      width: Math.abs(event.clientX - regionOrigin.x),
      height: Math.abs(event.clientY - regionOrigin.y)
    };
  }

  function endRegionDrag() {
    const selected = regionRect;
    regionOrigin = null;
    regionRect = null;
    if (!selected) return;
    // 太小的框選當成誤觸，直接退出而不是送出一張沒有文字的圖。
    if (selected.width < 8 || selected.height < 8) {
      fireAndForget("cancel_region_capture");
      return;
    }
    // 辨識失敗的訊息由 Rust 用 capture://unavailable 送到面板，這裡不必再接一次；
    // 但覆蓋視窗自己沒有地方顯示錯誤，讓 Promise 就這樣被拒絕會弄壞這個視窗的介面。
    fireAndForget("capture_screen_region", {
      selection: { ...selected, scale: window.devicePixelRatio }
    });
  }

  /// 把使用者自己輸入或貼上的原文送去翻譯。
  async function translateDraft() {
    const text = sourceDraft.trim();
    if (!text) return;
    composing = false;
    // 自己貼的原文預設展開，否則按下翻譯後畫面上看不出剛才貼了什麼
    sourceExpanded = true;
    await translate(text);
  }

  function clearSource() {
    sourceDraft = "";
  }

  /// Ctrl＋Enter 送出。原文常是多行，Enter 必須留給換行。
  function sourceKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      void translateDraft();
    }
  }

  /// 辨識剪貼簿裡的圖片。讓使用者可以改用自己慣用的截圖工具，
  /// 也能處理內建框選來不及框的畫面（影片、動畫、會自動關閉的選單）。
  async function pasteImageTranslate() {
    captureNotice = "";
    try {
      await invoke("translate_clipboard_image");
      composing = false;
    } catch (error) {
      captureNotice = String(error);
    }
  }

  /// 貼上圖片時攔下來走 OCR；貼上文字維持瀏覽器原生行為，否則輸入框就不能貼字了。
  function handlePaste(event: ClipboardEvent) {
    // 「譯」按鈕與框選覆蓋層共用同一份 App.svelte，貼上只對翻譯面板有意義。
    if (isActionWindow || isRegionWindow) return;
    const items = event.clipboardData?.items;
    if (!items) return;
    const hasImage = Array.from(items).some((item) => item.type.startsWith("image/"));
    if (!hasImage) return;
    event.preventDefault();
    void pasteImageTranslate();
  }

  function stopTranslation() {
    panel.cancel();
    refresh();
    fireAndForget("cancel_translation");
  }

  function togglePinned() {
    pinned = !pinned;
    fireAndForget("set_panel_pinned", { pinned });
    flashHint(pinned ? t.hints.pinned : t.hints.unpinned);
  }

  // 釘選只影響「下一次翻譯時面板會不會移動」，當下畫面沒有變化，
  // 沒有回饋的話使用者會以為按鈕壞了。
  let hint = "";
  let hintTimer: number | undefined;
  function flashHint(message: string) {
    hint = message;
    window.clearTimeout(hintTimer);
    hintTimer = window.setTimeout(() => (hint = ""), 2600);
  }

  let menu: { x: number; y: number } | null = null;

  function openMenu(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    // 輸入欄位維持系統原生選單，才能複製貼上
    if (target?.closest("input, select, textarea")) return;
    event.preventDefault();
    // 收合後整個視窗只剩右下角一顆 36×36 的圖示，比選單本身還小。選單一開就會
    // 整片蓋住那顆圖示——而它是收合狀態下唯一能點回展開的地方，等於把面板鎖死。
    // 原生選單同樣會蓋住，所以上面的 preventDefault 要留著，只是不再開自己的選單。
    if (collapsed) return;
    // 要和 app.css 的 .context-menu 對齊：這裡只是用來把選單夾在視窗內，
    // 數字對不上就會被邊緣切掉。字級放大並多了「全螢幕」「回到首頁」兩項後一起改。
    const width = 230;
    const height = 314;
    menu = {
      x: Math.min(event.clientX, window.innerWidth - width - 6),
      y: Math.min(event.clientY, window.innerHeight - height - 6)
    };
  }

  function runFromMenu(action: () => void) {
    menu = null;
    action();
  }

  // 設定與更新對話框是覆蓋整個視窗的彈窗，而收合後視窗只剩右下角的小標籤。
  // 兩者同時發生時，彈窗會蓋掉小標籤上唯一的展開按鈕，面板就再也打不開，
  // 連系統匣的「顯示隨譯」也救不回來——只能重開程式。
  $: modalOpen = settingsOpen || updatePromptOpen;

  function toggleCollapsed() {
    if (!collapsed && modalOpen) {
      flashHint(t.hints.cannotCollapseWhileSettingsOpen);
      return;
    }
    // 全螢幕時收合會把視窗縮成角落小標籤，離開全螢幕後尺寸也對不回來。
    if (fullscreen) {
      flashHint(t.hints.cannotCollapseWhileFullscreen);
      return;
    }
    collapsed = !collapsed;
    fireAndForget("set_panel_collapsed", { collapsed });
  }

  /// 全螢幕與收合、簡易模式互斥，切換前先把那兩個狀態解掉。
  function toggleFullscreen() {
    fullscreen = !fullscreen;
    if (fullscreen) {
      collapsed = false;
      miniMode = false;
    }
    fireAndForget("set_panel_fullscreen", { fullscreen });
  }

  /// 從簡易模式切換回完整面板，讓使用者可以使用解釋、設定等完整功能。
  function expandToFull() {
    miniMode = false;
    fireAndForget("set_panel_mini", { mini: false });
  }

  /// 收合狀態下只剩右下角的小標籤，譯文與對話框都無處可放，先把面板攤開。
  function expandFromCollapsed() {
    if (!collapsed) return;
    collapsed = false;
    fireAndForget("set_panel_collapsed", { collapsed: false });
  }

  /// 收合後的小標籤整塊都可以點來展開，但它同時也是搬動面板的把手。
  ///
  /// 不能沿用 data-tauri-drag-region：它會把 mousedown 直接當成拖曳吃掉，
  /// click 事件就永遠不會發生。所以這裡自己分——放開時位移在門檻內算點擊，
  /// 超過門檻才交給視窗拖曳。
  const DOCK_DRAG_THRESHOLD = 4;
  let dockPress: { x: number; y: number } | null = null;

  function beginDockPress(event: PointerEvent) {
    if (event.button !== 0) return;
    dockPress = { x: event.clientX, y: event.clientY };
  }

  function trackDockPress(event: PointerEvent) {
    if (!dockPress) return;
    const moved =
      Math.abs(event.clientX - dockPress.x) > DOCK_DRAG_THRESHOLD ||
      Math.abs(event.clientY - dockPress.y) > DOCK_DRAG_THRESHOLD;
    if (!moved) return;
    // 拖曳一旦交給視窗，這裡就收不到 pointerup 了，狀態必須當場清掉。
    dockPress = null;
    if (!("__TAURI_INTERNALS__" in window)) return;
    getCurrentWindow()
      .startDragging()
      .catch((error) => console.error("[隨譯] startDragging", error));
  }

  function endDockPress(event: PointerEvent) {
    if (!dockPress || event.button !== 0) return;
    dockPress = null;
    toggleCollapsed();
  }

  /// 回到起始畫面：清掉這一輪的原文、譯文與解釋，並把視窗還原成標準的完整面板。
  ///
  /// 全螢幕與簡易版面都是為了讀某一段譯文才切過去的，內容都清掉之後還留在那裡，
  /// 只會讓人以為程式卡住了。
  async function goHome() {
    try {
      if (snapshot.status === "streaming") stopTranslation();
      dismissExplanation();
      panel.reset();
      refresh();
      sourceDraft = "";
      sourceExpanded = false;
      composing = false;
      captureNotice = "";
      copied = false;

      collapsed = false;
      miniMode = false;
      // 這兩步有先後：離開全螢幕時 Tauri 會還原成進入前的尺寸，若和調成完整面板
      // 的那一步搶跑，視窗就會停在進全螢幕之前的簡易尺寸上。
      if (fullscreen) {
        fullscreen = false;
        await invoke("set_panel_fullscreen", { fullscreen: false });
      }
      // set_panel_mini 會一併把視窗調回完整面板的尺寸，並把超出工作區的部分拉回來，
      // 所以收合狀態不必再送一次 set_panel_collapsed。
      await invoke("set_panel_mini", { mini: false });
      await resetTranslationScroll();
    } catch (error) {
      console.error("[隨譯] goHome", error);
    }
  }

  /// 視窗會在 Rust 的 setup 掛上 state 之前就載入完成（release 版特別明顯，
  /// 因為前端是打包好的、載入極快），這時指令會回 "state not managed"。
  /// 這種錯誤只是還沒準備好，重試即可；其他錯誤照原樣往外拋。
  async function invokeWhenReady<T>(command: string): Promise<T> {
    let lastError: unknown;
    for (let attempt = 0; attempt < 25; attempt += 1) {
      try {
        return await invoke<T>(command);
      } catch (error) {
        lastError = error;
        if (!String(error).includes("state not managed")) throw error;
        await new Promise((resolve) => setTimeout(resolve, 120));
      }
    }
    throw lastError;
  }

  type ImageRecognition = "ocr" | "model" | "auto";
  type Preferences = {
    showSource: boolean;
    autoCollapse: boolean;
    clipboardFallback: boolean;
    launchAtLogin: boolean;
    imageRecognition: ImageRecognition;
    uiLocale: string;
  };

  async function loadPreferences() {
    const prefs = await invokeWhenReady<Preferences>("preferences");
    showSourceByDefault = prefs.showSource;
    autoCollapse = prefs.autoCollapse;
    clipboardFallback = prefs.clipboardFallback;
    launchAtLogin = prefs.launchAtLogin;
    imageRecognition = prefs.imageRecognition;
    // 經過 resolveLocale 而不是直接指派：資料庫裡可能是舊版寫的、或某個已經
    // 移除的語言檔留下的孤兒，認不得就退回預設，不要讓面板整片變成 undefined。
    uiLocale = resolveLocale(prefs.uiLocale).id;
  }

  /**
   * 換介面語言。存回 Rust 之外還要讓系統匣跟著換——那份選單是 Rust 建的，
   * 前端改自己的字串不會動到它，少了這一步就會出現「面板是英文、右下角是中文」。
   * 重建的動作在 Rust 的 `set_choice_preference` 裡，這裡只要把值送到就好。
   */
  async function selectLocale(id: LocaleId) {
    if (id === uiLocale) return;
    uiLocale = id;
    await saveChoicePreference("ui/locale", id);
  }

  async function savePreference(key: string, value: boolean) {
    if (key === "panel/show-source") showSourceByDefault = value;
    if (key === "panel/auto-collapse") autoCollapse = value;
    if (key === "capture/clipboard-fallback") clipboardFallback = value;
    if (key === "app/launch-at-login") launchAtLogin = value;
    try {
      await invoke("set_preference", { key, value });
    } catch (error) {
      settingsError = String(error);
      await loadPreferences().catch(() => undefined);
    }
  }

  async function saveChoicePreference(key: string, value: string) {
    if (key === "capture/image-recognition") imageRecognition = value as ImageRecognition;
    try {
      await invoke("set_choice_preference", { key, value });
    } catch (error) {
      settingsError = String(error);
    }
  }

  async function loadProfiles() {
    profiles = await invokeWhenReady<ModelProfile[]>("model_profiles");
    // 先回讀上次的選擇。這個值同時是 Rust 用模型辨識圖片時的依據——
    // 截圖覆蓋層是另一個視窗，問不到主面板選了哪個模型，只能靠存起來的這一份。
    const remembered = await invokeWhenReady<string | null>("active_model_profile").catch(() => null);
    if (remembered && profiles.some((profile) => profile.id === remembered)) {
      modelProfile = remembered;
    }
    if (!profiles.some((profile) => profile.id === modelProfile) && profiles[0]) {
      modelProfile = profiles[0].id;
    }
    rememberActiveProfile();
  }

  /// 把目前選用的設定檔存回 Rust。選單改變與載入完成都要送，
  /// 否則 Rust 那份會停在上一次、圖片辨識就用到別的模型。
  function rememberActiveProfile() {
    if (!modelProfile) return;
    void invoke("set_choice_preference", {
      key: "model/active-profile",
      value: modelProfile
    }).catch(() => undefined);
  }

  async function translate(sourceText: string) {
    if (!modelProfile) {
      captureNotice = t.errors.noModelProfile;
      settingsOpen = true;
      return;
    }
    captureNotice = "";
    // 舊的解釋是針對上一段原文的，換原文就作廢
    dismissExplanation();
    sourceDraft = sourceText;
    panel.start(sourceText, targetLanguage);
    refresh();
    void resetTranslationScroll();
    try {
      await invoke("translate_selection", {
        profileId: modelProfile,
        sourceText,
        targetLanguage
      });
    } catch (error) {
      captureNotice = String(error);
      panel.fail();
      refresh();
    }
  }

  function editSelectedProfile() {
    const selected = profiles.find((profile) => profile.id === modelProfile);
    if (selected) {
      profileDraft = { ...selected, apiKey: "" };
    }
    // 設定對話框覆蓋整個視窗。塞進小標籤或簡易面板時它會蓋掉自己的關閉按鈕，
    // 使用者連取消都按不到，所以開設定前一律先攤成完整面板。
    if (miniMode) expandToFull();
    expandFromCollapsed();
    settingsError = "";
    settingsOpen = true;
  }

  async function saveSettings() {
    savingSettings = true;
    settingsError = "";
    try {
      const saved = await invoke<ModelProfile>("save_model_profile", { input: profileDraft });
      await loadProfiles();
      modelProfile = saved.id;
      settingsOpen = false;
    } catch (error) {
      settingsError = String(error);
    } finally {
      savingSettings = false;
    }
  }

  let updatePromptOpen = false;
  let dismissedVersion = "";

  async function refreshUpdateStatus(options?: { prompt?: boolean; force?: boolean }) {
    checkingUpdate = true;
    updateError = "";
    try {
      updateStatus = await invoke<UpdateStatus>("check_for_update", {
        force: options?.force ?? false,
      });
      // 偵測到新版時主動跳出詢問，但同一版被關掉後就不再打擾。
      if (
        options?.prompt &&
        updateStatus.state === "available" &&
        updateStatus.version !== dismissedVersion
      ) {
        updatePromptOpen = true;
      }
    } catch (error) {
      updateStatus = null;
      updateError = String(error);
    } finally {
      checkingUpdate = false;
    }
  }

  function dismissUpdatePrompt() {
    if (updateStatus?.state === "available") dismissedVersion = updateStatus.version;
    updatePromptOpen = false;
  }

  function updateStatusLabel() {
    if (checkingUpdate) return t.update.checking;
    if (updateError) return t.update.checkFailed(updateError);
    if (!updateStatus) return t.update.notChecked;
    if (updateStatus.state === "disabled") return t.update.disabled;
    // 「沒有新版本」要講在最前面。使用者按下「檢查更新」想知道的就是這件事，
    // 把目前版號擺在前面（舊版寫「已是最新版本（0.2.1）」）會讓人先讀到一個版號，
    // 還要再想一下那是不是新的。
    if (updateStatus.state === "upToDate") return t.update.upToDate(updateStatus.current);
    return t.update.available(updateStatus.version);
  }

  /// 更新說明拆成一行一項。latest.json 的 notes 是 CHANGELOG 的條列原文，
  /// 這裡去掉條列符號交給 <li> 自己畫，免得出現兩個項目符號。
  function releaseNoteLines(notes: string | undefined): string[] {
    if (!notes) return [];
    return notes
      .split("\n")
      .map((line) => line.replace(/^\s*[-*・]\s*/, "").trim())
      .filter(Boolean);
  }

  async function installAvailableUpdate() {
    installingUpdate = true;
    updateError = "";
    updateProgress = null;
    try {
      await invoke("install_update");
      // 安裝完成後 Tauri updater 會自動重啟程式，這裡不會執行到。
    } catch (error) {
      updateError = String(error);
      installingUpdate = false;
      updateProgress = null;
    }
  }

  async function copyTranslation() {
    if (!snapshot.translatedText) return;
    try {
      await navigator.clipboard.writeText(snapshot.translatedText);
      copied = true;
      window.setTimeout(() => (copied = false), 1400);
    } catch (error) {
      captureNotice = t.errors.copyFailed(String(error));
    }
  }

  async function hidePanel() {
    try {
      await invoke("hide_panel_window");
    } catch {
      // 瀏覽器預覽沒有 Tauri 可用，退而求其次把面板淡出。
      document.body.classList.add("browser-preview-hidden");
    }
  }

  onMount(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;

    // 「譯」按鈕與框選覆蓋層是各自獨立的 webview，各跑一份這個元件。它們用不到
    // 設定檔、釘選狀態那些東西，但**還是有字要顯示**（按鈕的提示、框選的說明），
    // 所以介面語言要在早退之前就問。少了這一段，主面板換成英文之後，選取時冒出來
    // 的那顆按鈕與框選畫面上的說明仍然是中文。
    void invokeWhenReady<string>("ui_locale")
      .then((id) => (uiLocale = resolveLocale(id).id))
      .catch(() => undefined);
    // 這兩個 webview 建好之後就一直活著（只是藏起來），不會因為使用者在設定裡
    // 換了語言而重新載入。所以換語言要用廣播推過來，否則得重開程式才會生效。
    const localeChanged = listen<string>("panel://locale", ({ payload }) => {
      uiLocale = resolveLocale(payload).id;
    });

    if (isActionWindow || isRegionWindow) {
      return () => void localeChanged.then((off) => off());
    }

    void loadProfiles().catch((error) => (captureNotice = String(error)));
    void loadPreferences().catch(() => undefined);
    // 版號的真值只有 Rust 有（env!("CARGO_PKG_VERSION")）。前端不自己寫一份，
    // 否則改版時漏改這裡，首頁就會長期顯示一個過期的版號。
    void invokeWhenReady<{ version: string }>("runtime_status")
      .then((status) => (appVersion = status.version))
      .catch(() => undefined);
    // 釘選狀態的真值在 Rust，webview 重新載入後要回讀才不會顯示錯的狀態
    void invokeWhenReady<boolean>("panel_pinned")
      .then((value) => (pinned = value))
      .catch(() => undefined);
    void invokeWhenReady<boolean>("panel_mini")
      .then((value) => (miniMode = value))
      .catch(() => undefined);
    void invokeWhenReady<boolean>("panel_fullscreen")
      .then((value) => (fullscreen = value))
      .catch(() => undefined);
    void invokeWhenReady<string | null>("startup_notice")
      .then((notice) => {
        if (notice) captureNotice = notice;
      })
      .catch(() => undefined);
    void refreshUpdateStatus({ prompt: true });

    const unlisten: Promise<UnlistenFn>[] = [
      listen<{ sourceText: string; targetLanguage: string }>(
        "translation://started",
        ({ payload }) => {
          sourceDraft = payload.sourceText;
          panel.start(payload.sourceText, payload.targetLanguage);
          refresh();
          void resetTranslationScroll();
        }
      ),
      listen<string>("translation://delta", ({ payload }) => {
        panel.append(payload);
        refresh();
      }),
      listen("translation://completed", () => {
        panel.complete();
        refresh();
      }),
      listen<string>("translation://failed", ({ payload }) => {
        captureNotice = payload;
        panel.fail();
        refresh();
      }),
      listen("explanation://started", () => {
        explanation = "";
        explaining = true;
      }),
      listen<string>("explanation://delta", ({ payload }) => {
        explanation += payload;
      }),
      listen("explanation://completed", () => {
        explaining = false;
      }),
      listen<string>("explanation://failed", ({ payload }) => {
        explanationError = payload;
        explaining = false;
      }),
      listen<string>("capture://captured", ({ payload }) => {
        if (!miniMode) {
          sourceExpanded = showSourceByDefault;
          expandFromCollapsed();
        }
        void translate(payload);
      }),
      listen<boolean>("panel://mini", ({ payload }) => {
        miniMode = payload;
        // Rust 這時已經把視窗調成簡易或完整尺寸並移到游標旁，收合旗標留著的話
        // 前端會用小標籤版面渲染一個已經放大的視窗。
        collapsed = false;
      }),
      listen("panel://auto-collapse", () => {
        // 開著設定或更新對話框時收合，彈窗會蓋住展開按鈕，面板就打不開了。
        // 這是最容易踩到的路徑：開著設定去點別的程式就會發生。
        if (modalOpen) return;
        // 全螢幕是特意攤開來讀的，點一下別處就收掉等於白開
        if (fullscreen) return;
        // 簡易模式沒有收合狀態，直接隱藏面板
        if (miniMode) {
          void hidePanel();
          return;
        }
        // 翻譯或解釋還在跑就先不收，使用者正等著看結果
        if (collapsed || snapshot.status === "streaming" || explaining) return;
        collapsed = true;
        fireAndForget("set_panel_collapsed", { collapsed: true });
      }),
      listen<string>("capture://unavailable", ({ payload }) => {
        expandFromCollapsed();
        captureNotice = payload;
      }),
      // 模型辨識圖片要跑好幾秒，而系統 OCR 幾乎是瞬間完成的。
      // 沒有這個提示，使用者會盯著一個沒有動靜的面板，以為截圖翻譯壞了。
      listen<string>("capture://recognizing", ({ payload }) => {
        expandFromCollapsed();
        captureNotice = payload;
      }),
      listen<{ chunkLength: number; contentLength: number | null }>(
        "update://progress",
        ({ payload }) => {
          if (!installingUpdate) return;
          const total = payload.contentLength ?? 0;
          updateProgress = {
            downloaded: (updateProgress?.downloaded ?? 0) + payload.chunkLength,
            total,
          };
        }
      ),
      listen("update://installing", () => {
        updateProgress = null;
      }),
      localeChanged
    ];

    return () => {
      void Promise.all(unlisten).then((handlers) => handlers.forEach((handler) => handler()));
    };
  });
</script>

<svelte:window
  onpaste={handlePaste}
  onkeydown={(event) => {
    if (event.key !== "Escape") return;
    if (isRegionWindow) {
      fireAndForget("cancel_region_capture");
      return;
    }
    // 全螢幕是無邊框的，Esc 直接隱藏會讓整個畫面突然消失，看起來像當掉。
    // 先還原成視窗，再按一次才是隱藏——和其他全螢幕程式的習慣一致。
    if (fullscreen) {
      toggleFullscreen();
      return;
    }
    void hidePanel();
  }}
/>

{#if isRegionWindow}
  <div
    class="region-overlay"
    class:selecting={!!regionRect}
    role="presentation"
    onpointerdown={beginRegionDrag}
    onpointermove={updateRegionDrag}
    onpointerup={endRegionDrag}
    oncontextmenu={(event) => {
      event.preventDefault();
      fireAndForget("cancel_region_capture");
    }}
  >
    {#if regionRect}
      <div
        class="region-selection"
        style="left: {regionRect.x}px; top: {regionRect.y}px; width: {regionRect.width}px; height: {regionRect.height}px;"
      >
        <span class="region-size">{Math.round(regionRect.width)} × {Math.round(regionRect.height)}</span>
      </div>
    {:else}
      <p class="region-hint">{t.region.hint}</p>
    {/if}
  </div>
{:else if isActionWindow}
  <!-- 選取文字後跟在游標旁的那一小排。兩顆按鈕共用同一份選取內容，按下哪一顆
       都會把它取走並收起這個視窗，所以順序就是使用者最常按的順序：先譯後搜。 -->
  <div class="selection-actions">
    <button
      class="selection-action"
      aria-label={t.region.translateSelection}
      title={t.region.translateSelection}
      onclick={() => fireAndForget("accept_pending_selection")}
    >{sealGlyph}</button>
    <button
      class="selection-action search"
      aria-label={t.region.searchSelection}
      title={t.region.searchSelection}
      onclick={() => fireAndForget("search_pending_selection")}
    ><Icon name="search" size={15} /></button>
  </div>
{:else}
<main
  class:collapsed
  class:mini={miniMode}
  class:fullscreen
  class="panel-shell"
  aria-label={t.common.panelLabel}
  oncontextmenu={openMenu}
>
  {#if miniMode && !collapsed}
    <div class="mini-panel" data-tauri-drag-region>
      <div class="mini-header" data-tauri-drag-region>
        <span class="mini-seal-small" aria-hidden="true">{sealGlyph}</span>
        <span class="mini-status">
          <span class:streaming={snapshot.status === "streaming"} class="status-dot"></span>
          {snapshot.status === "streaming" ? t.status.streaming : snapshot.status === "idle" ? t.status.idle : ""}
        </span>
        <button class="icon-button" aria-label={t.mini.expandToFull} title={t.mini.expandToFull} onclick={expandToFull}>
          <Icon name="expand" size={14} />
        </button>
        <button class="icon-button" aria-label={t.titlebar.hidePanel} title={t.common.close} onclick={hidePanel}>
          <Icon name="close" size={14} />
        </button>
      </div>
      <div class="mini-body" bind:this={translationCard}>
        <p class:placeholder={!snapshot.translatedText} class="mini-translation-text">
          {snapshot.translatedText || captureNotice || t.status.waitingForModel}
        </p>
      </div>
      <div class="mini-footer">
        {#if snapshot.status === "streaming"}
          <button class="text-button danger" onclick={stopTranslation}>{t.common.stop}</button>
        {/if}
        <button class="primary-button mini-copy-btn" disabled={!snapshot.translatedText} onclick={copyTranslation}>
          {copied ? t.common.copied : t.common.copy}
        </button>
      </div>
    </div>
  {:else if collapsed}
    <!-- 收合後只剩一顆圖示，整塊就是展開鈕。名稱不再顯示，所以識別完全靠
         aria-label 與 title——螢幕閱讀器與滑鼠停留提示是這裡僅有的說明。 -->
    <div
      class="mini-dock"
      role="button"
      tabindex="0"
      aria-label={t.mini.dockLabel(appName)}
      title={t.mini.dockHint(appName)}
      onpointerdown={beginDockPress}
      onpointermove={trackDockPress}
      onpointerup={endDockPress}
      onpointercancel={() => (dockPress = null)}
      onkeydown={(event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        toggleCollapsed();
      }}
    >
      <span class="mini-seal" aria-hidden="true">{sealGlyph}</span>
    </div>
  {:else}
  {#if availableUpdate}
    <aside class="update-banner" role="status">
      <span>{t.update.banner(appName, availableUpdate.version)}</span>
      <button disabled={installingUpdate} onclick={() => void installAvailableUpdate()}>{installingUpdate ? t.update.installing : t.update.install}</button>
    </aside>
  {/if}
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <span class="seal" aria-hidden="true">
        <span class="seal-frame"></span>
        <span class="seal-glyph">{sealGlyph}</span>
      </span>
      <div data-tauri-drag-region>
        <strong>{appName}</strong>
        {#if showWordmark}<span>{appNameEn}</span>{/if}
      </div>
    </div>
    <div class="window-actions">
      <button
        class="icon-button"
        aria-label={t.titlebar.home}
        title={t.titlebar.homeHint}
        onclick={() => void goHome()}
      >
        <Icon name="home" />
      </button>
      <button class="icon-button" aria-label={t.titlebar.capture} title={t.titlebar.captureHint} onclick={() => void startRegionCapture()}>
        <Icon name="capture" />
      </button>
      <button class="icon-button" aria-label={t.titlebar.settings} title={t.titlebar.settings} onclick={editSelectedProfile}>
        <Icon name="settings" />
      </button>
      <button
        class:active={pinned}
        class="icon-button"
        aria-label={pinned ? t.titlebar.unpin : t.titlebar.pin}
        aria-pressed={pinned}
        title={pinned ? t.titlebar.unpinHint : t.titlebar.pinHint}
        onclick={togglePinned}
      >
        <Icon name="pin" filled={pinned} />
      </button>
      <button
        class:active={fullscreen}
        class="icon-button"
        aria-label={fullscreen ? t.titlebar.restore : t.titlebar.fullscreen}
        aria-pressed={fullscreen}
        title={fullscreen ? t.titlebar.restore : t.titlebar.fullscreen}
        onclick={toggleFullscreen}
      >
        <Icon name={fullscreen ? "restore" : "fullscreen"} />
      </button>
      <button class="icon-button" aria-label={collapsed ? t.menu.expand : t.common.collapse} title={collapsed ? t.titlebar.expandHint : t.titlebar.collapseHint} onclick={toggleCollapsed}>
        <Icon name={collapsed ? "expand" : "collapse"} />
      </button>
      <button class="icon-button" aria-label={t.titlebar.hidePanel} title={t.common.close} onclick={hidePanel}>
        <Icon name="close" />
      </button>
    </div>
  </header>

  <section class="context-strip" aria-label={t.titlebar.settings}>
    <label>
      <span>{t.strip.model}</span>
      <select bind:value={modelProfile} onchange={rememberActiveProfile} aria-label={t.strip.modelProfile}>
        {#each profiles as profile}
          <option value={profile.id}>{profile.name}</option>
        {/each}
        {#if profiles.length === 0}<option value="">{t.strip.noModel}</option>{/if}
      </select>
    </label>
    <span class="route" aria-hidden="true">→</span>
    <label>
      <span>{t.strip.target}</span>
      <select
        bind:value={targetLanguage}
        aria-label={t.strip.targetLanguage}
        onchange={() => (targetLanguageTouched = true)}
      >
        {#each TARGET_LANGUAGES as language}
          <option>{language}</option>
        {/each}
      </select>
    </label>
  </section>

  {#if snapshot.status === "idle"}
    <section class="empty-state" aria-live="polite">
      {#if composing}
        <p class="eyebrow">{t.empty.pasteEyebrow}</p>
        <textarea
          class="source-input tall"
          bind:value={sourceDraft}
          aria-label={t.empty.textToTranslate}
          placeholder={t.empty.pastePlaceholder}
          spellcheck="false"
          onkeydown={sourceKeydown}
        ></textarea>
        <div class="source-actions">
          <span class="source-tip">{t.common.submitHint}</span>
          <button class="text-button" onclick={() => (composing = false)}>{t.common.back}</button>
          <button class="text-button" onclick={() => void pasteImageTranslate()}>{t.common.pasteImage}</button>
          <button class="primary-button" disabled={!sourceDraft.trim()} onclick={() => void translateDraft()}>{t.common.translate}</button>
        </div>
      {:else}
        <span class="selection-mark" aria-hidden="true"></span>
        <p class="eyebrow">{t.brand.slogan}</p>
        <h1>{t.brand.tagline}</h1>
        <p class="hint">{t.empty.selectHint}</p>
        <kbd>Ctrl</kbd><span class="key-plus">{t.common.keyPlus}</span><kbd>Alt</kbd><span class="key-plus">{t.common.keyPlus}</span><kbd>T</kbd>
        <p class="hint">{t.empty.captureHint}</p>
        <div class="shot-row">
          <button class="primary-button" onclick={() => void startRegionCapture()}>{t.empty.capture}</button>
          <span><kbd>Ctrl</kbd><span class="key-plus">{t.common.keyPlus}</span><kbd>Alt</kbd><span class="key-plus">{t.common.keyPlus}</span><kbd>R</kbd></span>
        </div>
        <div class="shot-row compose-row">
          <button class="text-button" onclick={() => (composing = true)}>{t.empty.pasteText}</button>
          <button class="text-button" onclick={() => void pasteImageTranslate()}>{t.empty.pasteImage}</button>
        </div>
        {#if appVersion}
          <p class="version-line">
            {appName} v{appVersion}
            {#if availableUpdate}
              <!-- 只有真的偵測到新版才給點。沒有新版時這裡是純文字，不是停用的按鈕：
                   會不會亮起來，本身就是「有沒有新版」最直接的回答。 -->
              <button class="text-button version-update" onclick={() => (updatePromptOpen = true)}>
                {t.empty.updateAvailable(availableUpdate.version)}
              </button>
            {:else if updateStatus?.state === "upToDate"}
              <span class="version-uptodate">{t.empty.upToDate}</span>
            {/if}
          </p>
        {/if}
      {/if}
      {#if captureNotice}<p class="notice" role="status">{captureNotice}</p>{/if}
    </section>
  {:else}
    <section class="translation-card" aria-live="polite" bind:this={translationCard}>
      <button class="source-toggle" onclick={() => (sourceExpanded = !sourceExpanded)}>
        <span>{t.card.source}</span>
        <span>{sourceExpanded ? t.common.collapse : t.common.expand}</span>
      </button>
      {#if sourceExpanded}
        <textarea
          class="source-input"
          bind:value={sourceDraft}
          aria-label={t.card.sourceEditable}
          placeholder={t.card.sourcePlaceholder}
          spellcheck="false"
          onkeydown={sourceKeydown}
        ></textarea>
        <div class="source-actions">
          <span class="source-tip">{t.common.submitHint}</span>
          <button class="text-button" disabled={!sourceDraft} onclick={clearSource}>{t.common.clear}</button>
          <button class="text-button" onclick={() => void pasteImageTranslate()}>{t.common.pasteImage}</button>
          <button
            class="primary-button"
            disabled={!sourceDraft.trim() || snapshot.status === "streaming"}
            onclick={() => void translateDraft()}
          >{t.common.translate}</button>
        </div>
      {/if}

      <div class="translation-heading">
        <span>{t.card.translation}</span>
        <span class:streaming={snapshot.status === "streaming"} class="status-dot"></span>
      </div>
      <p class:placeholder={!snapshot.translatedText} class="translation-text">
        {snapshot.translatedText || captureNotice || t.status.waitingForModel}
      </p>

      {#if explanationRequested}
        <div class="explanation">
          <div class="explanation-heading">
            <span>{t.card.explanation}</span>
            <span class:streaming={explaining} class="status-dot"></span>
            <button class="text-button" onclick={dismissExplanation}>{explaining ? t.common.stop : t.card.hide}</button>
          </div>
          {#if explanationError}
            <p class="form-error" role="alert">{explanationError}</p>
          {:else if explanation}
            <p class="explanation-text">{explanationText}</p>
          {:else if explaining}
            <p class="explanation-text placeholder">{t.card.explanationLoading}</p>
          {:else}
            <p class="explanation-text placeholder">{t.card.explanationEmpty}</p>
          {/if}
        </div>
      {/if}
    </section>
  {/if}

  <footer>
    <div class="status-copy">
      <span class:streaming={snapshot.status === "streaming"} class="status-dot"></span>
      {snapshot.status === "streaming" ? t.status.streaming : snapshot.status === "cancelled" ? t.status.cancelled : t.status.idle}
      {#if pinned}<span class="pin-badge" title={t.footer.pinnedHint}><Icon name="pin" filled size={12} />{t.footer.pinned}</span>{/if}
    </div>
    <div class="result-actions">
      {#if snapshot.status === "streaming"}
        <button class="text-button danger" onclick={stopTranslation}>{t.common.stop}</button>
      {/if}
      <button
        class="text-button"
        title={t.footer.explainHint}
        disabled={!snapshot.translatedText || explaining}
        onclick={() => void explainTranslation()}
      >{explaining ? t.footer.explaining : t.footer.explain}</button>
      <button class="primary-button" disabled={!snapshot.translatedText} onclick={copyTranslation}>
        {copied ? t.common.copied : t.footer.copyTranslation}
      </button>
    </div>
  </footer>
  {/if}
</main>

{#if hint}
  <div class="hint-toast" role="status">{hint}</div>
{/if}

{#if updatePromptOpen && availableUpdate}
  <div class="dialog-backdrop" role="presentation">
    <div class="update-dialog" role="alertdialog" aria-modal="true" aria-labelledby="update-title">
      <p class="eyebrow">{t.update.eyebrow}</p>
      <h2 id="update-title">{t.update.title}</h2>
      <p class="update-version">{appName} {availableUpdate.version}</p>

      <div class="release-notes">
        {#if releaseNoteLines(availableUpdate.notes).length}
          <p class="notes-label">{t.update.notesLabel}</p>
          <ul>
            {#each releaseNoteLines(availableUpdate.notes) as line}
              <li>{line}</li>
            {/each}
          </ul>
        {:else}
          <p class="notes-empty">{t.update.notesEmpty}</p>
        {/if}
      </div>

      <p class="update-warning">{t.update.warning}</p>

      {#if updateError}
        <p class="update-error">{t.update.failed(updateError)}</p>
      {/if}

      {#if installingUpdate && updateProgress}
        <div class="update-progress-bar" role="progressbar" aria-valuenow={updateProgressPct} aria-valuemin={0} aria-valuemax={100}>
          <div class="update-progress-fill" style="width: {updateProgressPct}%"></div>
        </div>
        <p class="update-progress-label">
          {#if updateProgressPct > 0}{updateProgressPct}%{:else}{t.update.downloading}{/if}
        </p>
      {:else if installingUpdate}
        <p class="update-progress-label">{t.update.restarting}</p>
      {/if}

      <div class="dialog-actions">
        <button type="button" class="text-button" disabled={installingUpdate} onclick={dismissUpdatePrompt}>{t.update.later}</button>
        <button
          type="button"
          class="primary-button"
          disabled={installingUpdate}
          onclick={() => void installAvailableUpdate()}
        >{installingUpdate ? t.update.installing : t.update.installNow}</button>
      </div>
    </div>
  </div>
{/if}

{#if menu}
  <div
    class="menu-catcher"
    role="presentation"
    onpointerdown={() => (menu = null)}
    oncontextmenu={(event) => { event.preventDefault(); menu = null; }}
  ></div>
  <div class="context-menu" role="menu" style="left: {menu.x}px; top: {menu.y}px;">
    <button role="menuitem" onclick={() => runFromMenu(() => void goHome())}>
      <span>{t.menu.home}</span>
    </button>
    <button role="menuitem" onclick={() => runFromMenu(() => void startRegionCapture())}>
      <span>{t.menu.capture}</span><kbd>Ctrl{t.common.keyPlus}Alt{t.common.keyPlus}R</kbd>
    </button>
    <button role="menuitem" onclick={() => runFromMenu(() => void pasteImageTranslate())}>
      <span>{t.menu.pasteImage}</span><kbd>Ctrl{t.common.keyPlus}V</kbd>
    </button>
    <button role="menuitemcheckbox" aria-checked={pinned} onclick={() => runFromMenu(togglePinned)}>
      <span>{pinned ? t.menu.unpin : t.menu.pin}</span><span class="menu-mark"><Icon name="pin" filled={pinned} size={14} /></span>
    </button>
    <button role="menuitemcheckbox" aria-checked={fullscreen} onclick={() => runFromMenu(toggleFullscreen)}>
      <span>{fullscreen ? t.menu.restore : t.menu.fullscreen}</span><span class="menu-mark"><Icon name={fullscreen ? "restore" : "fullscreen"} size={14} /></span>
    </button>
    <button role="menuitem" onclick={() => runFromMenu(toggleCollapsed)}>
      <span>{collapsed ? t.menu.expand : t.menu.collapse}</span>
    </button>
    <hr />
    <button role="menuitem" disabled={!snapshot.translatedText} onclick={() => runFromMenu(() => void copyTranslation())}>
      <span>{t.menu.copyTranslation}</span>
    </button>
    <button role="menuitem" onclick={() => runFromMenu(editSelectedProfile)}>
      <span>{t.menu.settings}</span>
    </button>
    <hr />
    <button role="menuitem" onclick={() => runFromMenu(() => void hidePanel())}>
      <span>{t.menu.hidePanel}</span><kbd>Esc</kbd>
    </button>
  </div>
{/if}

{#if settingsOpen}
  <div class="dialog-backdrop" role="presentation">
    <form class="settings-dialog" aria-label={t.settings.title} onsubmit={(event) => { event.preventDefault(); void saveSettings(); }}>
      <div class="dialog-heading">
        <div><p class="eyebrow">{t.settings.eyebrow}</p><h2>{t.settings.title}</h2></div>
        <button type="button" class="icon-button" aria-label={t.settings.close} onclick={() => (settingsOpen = false)}>×</button>
      </div>

      <label>{t.settings.profileName}<input bind:value={profileDraft.name} required /></label>
      <label>{t.settings.provider}
        <select value={profileDraft.provider} onchange={(event) => selectProvider(event.currentTarget.value as ProviderId)}>
          <optgroup label={t.settings.providerGroupNative}>
            <option value="anthropic">Anthropic</option>
            <option value="azure-openai">Azure OpenAI</option>
            <option value="google-gemini">Google Gemini</option>
            <option value="openai-compatible">OpenAI</option>
            <option value="xai">xAI</option>
          </optgroup>
          <optgroup label={t.settings.providerGroupGateway}>
            <option value="openrouter">OpenRouter</option>
            <option value="ollama-native">{t.settings.providerOllama}</option>
            <option value="fedgpt">{t.settings.providerFedGpt}</option>
            <option value="custom-endpoint">{t.settings.providerCustom}</option>
          </optgroup>
        </select>
      </label>
      <label>{t.settings.endpoint}<input bind:value={profileDraft.endpoint} required spellcheck="false" placeholder={t.settings.endpointPlaceholder} /></label>
      <label>{profileDraft.provider === "azure-openai" ? t.settings.deployment : t.settings.modelName}<input bind:value={profileDraft.model} required spellcheck="false" placeholder={profileDraft.provider === "openrouter" ? t.settings.openRouterPlaceholder : t.settings.modelPlaceholder} /></label>
      {#if profileDraft.provider !== "ollama-native"}
        <label>{credentialLabel(profileDraft.provider)}<input bind:value={profileDraft.apiKey} type="password" autocomplete="new-password" placeholder={t.settings.apiKeyPlaceholder} /></label>
        <p class="privacy-note">
          {providerNote(profileDraft.provider)}
          {t.settings.credentialNote}
        </p>
      {/if}
      {#if settingsError}<p class="form-error" role="alert">{settingsError}</p>{/if}

      <div class="pref-section">
        <p class="eyebrow">{t.prefs.language}</p>
        <p class="pref-lead">{t.prefs.languageLead}</p>
        {#each LOCALES as option}
          <label class="pref-row">
            <input
              type="radio"
              name="ui-locale"
              checked={uiLocale === option.id}
              onchange={() => void selectLocale(option.id)}
            />
            <!-- 語言名一律用該語言自己的說法，來換語言的人多半正是因為看不懂目前這個。 -->
            <span lang={option.id}>{option.nativeName}</span>
          </label>
        {/each}
      </div>

      <div class="pref-section">
        <p class="eyebrow">{t.prefs.panel}</p>
        <label class="pref-row">
          <input
            type="checkbox"
            checked={showSourceByDefault}
            onchange={(event) => void savePreference("panel/show-source", event.currentTarget.checked)}
          />
          <span>
            {t.prefs.showSource}
            <small>{t.prefs.showSourceHint}</small>
          </span>
        </label>
        <label class="pref-row">
          <input
            type="checkbox"
            checked={autoCollapse}
            onchange={(event) => void savePreference("panel/auto-collapse", event.currentTarget.checked)}
          />
          <span>
            {t.prefs.autoCollapse}
            <small>{t.prefs.autoCollapseHint}</small>
          </span>
        </label>
      </div>

      <div class="pref-section">
        <p class="eyebrow">{t.prefs.startup}</p>
        <label class="pref-row">
          <input
            type="checkbox"
            checked={launchAtLogin}
            onchange={(event) => void savePreference("app/launch-at-login", event.currentTarget.checked)}
          />
          <span>
            {t.prefs.launchAtLogin}
            <small>{t.prefs.launchAtLoginHint}</small>
          </span>
        </label>
      </div>

      <div class="pref-section">
        <p class="eyebrow">{t.prefs.imageRecognition}</p>
        <p class="pref-lead">{t.prefs.imageRecognitionLead}</p>
        <label class="pref-row">
          <input
            type="radio"
            name="image-recognition"
            checked={imageRecognition === "ocr"}
            onchange={() => void saveChoicePreference("capture/image-recognition", "ocr")}
          />
          <span>
            {t.prefs.systemOcr}
            <small>{t.prefs.systemOcrHint}</small>
          </span>
        </label>
        <label class="pref-row">
          <input
            type="radio"
            name="image-recognition"
            checked={imageRecognition === "model"}
            onchange={() => void saveChoicePreference("capture/image-recognition", "model")}
          />
          <span>
            {t.prefs.modelOcr}
            <small>{t.prefs.modelOcrHint}<strong>{t.prefs.modelOcrHintEmphasis}</strong></small>
          </span>
        </label>
        <label class="pref-row">
          <input
            type="radio"
            name="image-recognition"
            checked={imageRecognition === "auto"}
            onchange={() => void saveChoicePreference("capture/image-recognition", "auto")}
          />
          <span>
            {t.prefs.autoOcr}
            <small>{t.prefs.autoOcrHint}<strong>{t.prefs.autoOcrHintEmphasis}</strong></small>
          </span>
        </label>
        {#if imageRecognition !== "ocr"}
          <p class="privacy-note">{t.prefs.imagePrivacyNote}</p>
        {/if}
      </div>

      <div class="pref-section">
        <p class="eyebrow">{t.prefs.selection}</p>
        <label class="pref-row">
          <input
            type="checkbox"
            checked={clipboardFallback}
            onchange={(event) => void savePreference("capture/clipboard-fallback", event.currentTarget.checked)}
          />
          <span>
            {t.prefs.clipboardFallback}
            <small>{t.prefs.clipboardFallbackHint}</small>
          </span>
        </label>
      </div>

      <div class="update-row">
        <div>
          <p class="eyebrow">{t.prefs.updates}</p>
          <p class="update-state">{updateStatusLabel()}</p>
        </div>
        <div class="update-actions">
          <button type="button" class="text-button" disabled={checkingUpdate} onclick={() => void refreshUpdateStatus({ force: true })}>
            {checkingUpdate ? t.update.checking : t.update.check}
          </button>
          <!-- 沒有新版本時這顆是停用的。可按卻什麼都不會發生的按鈕，
               會讓人以為更新壞了而反覆點。 -->
          <button
            type="button"
            class="primary-button update-now"
            disabled={!availableUpdate || installingUpdate}
            onclick={() => void installAvailableUpdate()}
          >{installingUpdate ? t.update.installing : t.update.installNow}</button>
        </div>
      </div>
      {#if availableUpdate}
        <div class="update-notes-inline">
          <p class="notes-label">{t.update.notesFor(availableUpdate.version)}</p>
          {#if releaseNoteLines(availableUpdate.notes).length}
            <ul>
              {#each releaseNoteLines(availableUpdate.notes) as line}
                <li>{line}</li>
              {/each}
            </ul>
          {:else}
            <p class="notes-empty">{t.update.notesEmpty}</p>
          {/if}
          <p class="update-warning">{t.update.warning}</p>
        </div>
      {/if}
      <div class="dialog-actions">
        <button type="button" class="text-button" onclick={() => (settingsOpen = false)}>{t.common.cancel}</button>
        <button class="primary-button" disabled={savingSettings}>{savingSettings ? t.settings.saving : t.settings.save}</button>
      </div>
    </form>
  </div>
{/if}
{/if}
