/**
 * 介面語言的字串合約。
 *
 * 每個語言一個檔案，全部以 `satisfies Locale` 對齊這份介面——少一個鍵、多一個鍵，
 * TypeScript 當場就會指出來。加一個新語言的完整流程只有三步：
 *
 *   1. 複製 `zh-TW.ts` 改成新的檔名
 *   2. 把字串翻過去（介面會逼你翻完，漏掉的鍵編不過）
 *   3. 在 `index.ts` 的 `LOCALES` 陣列加一行
 *
 * 沒有第四步。畫面上不該再出現任何寫死的文案，新語言就不會有「翻了一半」的角落。
 *
 * 帶變數的字串一律做成函式而不是佔位符樣板（`{0}`、`%s` 這類）。理由是語序：
 * 中文說「沒有新版本，目前是 0.2.5」，英文說「Up to date (0.2.5)」，日文的助詞
 * 又得跟著動。函式讓每個語言自己決定變數擺哪裡，樣板則會把中文的語序偷偷變成
 * 所有語言的語序。
 */

/** BCP 47 標籤。同時寫進 `<html lang>`，斷行與字型選擇要靠它。 */
export type LocaleId = "zh-TW" | "zh-CN" | "en" | "ja";

export interface Locale {
  readonly id: LocaleId;
  /**
   * 語言選單上顯示的名字，一律用該語言自己的說法（native name）。
   *
   * 不用「繁體中文／Traditional Chinese」這種當前介面語言的說法：使用者會來換語言，
   * 多半正是因為看不懂目前這個語言。列表用母語寫，找得到自己那一行才是重點。
   */
  readonly nativeName: string;
  /**
   * 首次啟動時的預設翻譯目標語言。
   *
   * 這個字串會**原樣送給模型**（提示詞裡寫的就是自然語言的語言名），所以它不是
   * 介面文案，不可以為了好看而改寫。介面語言是英文卻預設翻成繁體中文很奇怪，
   * 所以由各語言自己決定一個合理的起點；使用者選過之後以選過的為準。
   */
  readonly defaultTargetLanguage: string;
  readonly strings: Strings;
}

export interface Strings {
  brand: {
    /** 產品名。中文圈用「隨譯」，其餘語言用羅馬字商標，免得畫面上出現讀不出來的字。 */
    name: string;
    slogan: string;
    tagline: string;
  };

  /** 多處共用的短詞。同一個概念只留一份，才不會「複製／拷貝」在不同角落各說各話。 */
  common: {
    translate: string;
    stop: string;
    cancel: string;
    close: string;
    expand: string;
    collapse: string;
    clear: string;
    back: string;
    copy: string;
    copied: string;
    pasteImage: string;
    submitHint: string;
    apiKey: string;
    /**
     * 快捷鍵中間的加號。
     *
     * 中日文排版用全形「＋」才不會在方塊字之間顯得太瘦，英文用半形「+」——
     * 全形加號夾在 Ctrl 和 Alt 中間會撐出一個不該有的空格。是排版不是翻譯，
     * 但一樣得跟著語言走。
     */
    keyPlus: string;
    /** 主面板的無障礙名稱。畫面上看不到，螢幕閱讀器唸的就是這個。 */
    panelLabel: string;
  };

  status: {
    idle: string;
    streaming: string;
    cancelled: string;
    waitingForModel: string;
  };

  titlebar: {
    home: string;
    homeHint: string;
    capture: string;
    captureHint: string;
    settings: string;
    pin: string;
    unpin: string;
    pinHint: string;
    unpinHint: string;
    fullscreen: string;
    restore: string;
    collapseHint: string;
    expandHint: string;
    hidePanel: string;
  };

  strip: {
    model: string;
    modelProfile: string;
    noModel: string;
    target: string;
    targetLanguage: string;
  };

  empty: {
    pasteEyebrow: string;
    textToTranslate: string;
    pastePlaceholder: string;
    selectHint: string;
    captureHint: string;
    capture: string;
    pasteText: string;
    pasteImage: string;
    updateAvailable(version: string): string;
    upToDate: string;
  };

  card: {
    source: string;
    sourceEditable: string;
    sourcePlaceholder: string;
    translation: string;
    explanation: string;
    explanationLoading: string;
    explanationEmpty: string;
    hide: string;
  };

  footer: {
    pinned: string;
    pinnedHint: string;
    explain: string;
    explaining: string;
    explainHint: string;
    copyTranslation: string;
  };

  menu: {
    home: string;
    capture: string;
    pasteImage: string;
    pin: string;
    unpin: string;
    fullscreen: string;
    restore: string;
    collapse: string;
    expand: string;
    copyTranslation: string;
    settings: string;
    hidePanel: string;
  };

  mini: {
    expandToFull: string;
    dockLabel(appName: string): string;
    dockHint(appName: string): string;
  };

  region: {
    hint: string;
    translateSelection: string;
    searchSelection: string;
  };

  hints: {
    pinned: string;
    unpinned: string;
    cannotCollapseWhileSettingsOpen: string;
    cannotCollapseWhileFullscreen: string;
  };

  errors: {
    noModelProfile: string;
    copyFailed(detail: string): string;
  };

  update: {
    /** 版面上的全大寫小標。是排版元素，翻不翻由各語言自己決定。 */
    eyebrow: string;
    bannerEyebrow: string;
    banner(appName: string, version: string): string;
    installing: string;
    install: string;
    title: string;
    notesLabel: string;
    notesFor(version: string): string;
    notesEmpty: string;
    warning: string;
    failed(detail: string): string;
    downloading: string;
    restarting: string;
    later: string;
    installNow: string;
    checking: string;
    check: string;
    checkFailed(detail: string): string;
    notChecked: string;
    disabled: string;
    upToDate(version: string): string;
    available(version: string): string;
  };

  settings: {
    eyebrow: string;
    title: string;
    close: string;
    profileName: string;
    provider: string;
    providerGroupNative: string;
    providerGroupGateway: string;
    providerOllama: string;
    providerFedGpt: string;
    providerCustom: string;
    endpoint: string;
    endpointPlaceholder: string;
    deployment: string;
    modelName: string;
    modelPlaceholder: string;
    openRouterPlaceholder: string;
    apiKeyPlaceholder: string;
    credentialNote: string;
    save: string;
    saving: string;
    /** 各家供應商的一句話說明，接在 `credentialNote` 前面。 */
    note: {
      fedgpt: string;
      anthropic: string;
      gemini: string;
      azure: string;
      custom: string;
      openaiCompatible: string;
    };
    /** 模型設定檔的預設名稱。使用者可以改，這裡只是新增時的起點。 */
    defaultProfileName: {
      ollama: string;
      fedgpt: string;
      custom: string;
    };
  };

  prefs: {
    language: string;
    languageLead: string;
    panel: string;
    showSource: string;
    showSourceHint: string;
    autoCollapse: string;
    autoCollapseHint: string;
    startup: string;
    launchAtLogin: string;
    launchAtLoginHint: string;
    imageRecognition: string;
    imageRecognitionLead: string;
    systemOcr: string;
    systemOcrHint: string;
    modelOcr: string;
    modelOcrHint: string;
    modelOcrHintEmphasis: string;
    autoOcr: string;
    autoOcrHint: string;
    autoOcrHintEmphasis: string;
    imagePrivacyNote: string;
    selection: string;
    clipboardFallback: string;
    clipboardFallbackHint: string;
    updates: string;
  };
}
