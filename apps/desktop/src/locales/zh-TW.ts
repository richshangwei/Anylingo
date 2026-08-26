import type { Locale } from "./types";

/**
 * 繁體中文——這是**基準語言**。
 *
 * 字串以這裡為準再翻到其他語言，`types.ts` 的介面也是照這份的實際需要長出來的。
 * 改文案時先改這裡，其他三個檔案的同一個鍵才知道要跟著動。
 */
export const zhTW = {
  id: "zh-TW",
  nativeName: "繁體中文",
  defaultTargetLanguage: "繁體中文",
  strings: {
    brand: {
      name: "隨譯",
      slogan: "所見所選，皆可譯",
      tagline: "不限網頁，整台電腦的字都能譯"
    },

    common: {
      translate: "翻譯",
      stop: "停止",
      cancel: "取消",
      close: "關閉",
      expand: "展開",
      collapse: "收合",
      clear: "清空",
      back: "返回",
      copy: "複製",
      copied: "已複製",
      pasteImage: "貼上圖片",
      submitHint: "Ctrl＋Enter 翻譯",
      apiKey: "API Key",
      keyPlus: "＋",
      panelLabel: "翻譯面板"
    },

    status: {
      idle: "待命",
      streaming: "翻譯中",
      cancelled: "已停止",
      waitingForModel: "正在等待模型回應…"
    },

    titlebar: {
      home: "回到首頁",
      homeHint: "回到首頁：清空這一輪的原文與譯文，還原視窗",
      capture: "截圖翻譯",
      captureHint: "截圖翻譯（Ctrl＋Alt＋R）",
      settings: "模型設定",
      pin: "釘選目前位置",
      unpin: "取消釘選位置",
      pinHint: "釘選目前位置，面板不再跟著移動",
      unpinHint: "取消釘選，面板會跟著選取位置移動",
      fullscreen: "放大至全螢幕",
      restore: "還原視窗大小",
      collapseHint: "收合至右下角",
      expandHint: "展開",
      hidePanel: "關閉面板"
    },

    strip: {
      model: "模型",
      modelProfile: "模型設定檔",
      noModel: "尚未設定模型",
      target: "目標",
      targetLanguage: "目標語言"
    },

    empty: {
      pasteEyebrow: "貼上文字",
      textToTranslate: "要翻譯的文字",
      pastePlaceholder: "貼上或輸入要翻譯的文字，也可以直接貼上截圖",
      selectHint: "在任何應用程式反白文字後，按下快捷鍵。",
      captureHint: "選不到文字時，改用框選截圖，辨識畫面上的字再翻譯。",
      capture: "截圖翻譯",
      pasteText: "貼上文字翻譯",
      pasteImage: "貼上圖片翻譯",
      updateAvailable: (version) => `有新版本 ${version}`,
      upToDate: "沒有新版本"
    },

    card: {
      source: "原文",
      sourceEditable: "原文，可修改後重新翻譯",
      sourcePlaceholder: "清空後可以自己貼上或輸入要翻譯的文字",
      translation: "譯文",
      explanation: "解釋",
      explanationLoading: "正在請模型說明…",
      explanationEmpty: "這個模型沒有回覆說明，可以再試一次或換一個模型。",
      hide: "收起"
    },

    footer: {
      pinned: "已釘選",
      pinnedHint: "面板已釘選在目前位置",
      explain: "解釋",
      explaining: "解釋中…",
      explainHint: "請模型補充說明術語、縮寫與語氣，不影響上方譯文",
      copyTranslation: "複製譯文"
    },

    menu: {
      home: "回到首頁",
      capture: "截圖翻譯",
      pasteImage: "貼上圖片翻譯",
      pin: "釘選目前位置",
      unpin: "取消釘選位置",
      fullscreen: "放大至全螢幕",
      restore: "還原視窗大小",
      collapse: "收合至右下角",
      expand: "展開面板",
      copyTranslation: "複製譯文",
      settings: "模型設定",
      hidePanel: "隱藏面板"
    },

    mini: {
      expandToFull: "展開為完整面板",
      dockLabel: (appName) => `展開${appName}翻譯面板`,
      dockHint: (appName) => `${appName}　·　點一下展開，拖曳可移動`
    },

    region: {
      hint: "拖曳框選要翻譯的畫面範圍　·　Esc 取消",
      translateSelection: "翻譯選取文字",
      searchSelection: "用預設瀏覽器搜尋選取文字"
    },

    hints: {
      pinned: "已釘選位置，面板不再跟著選取移動",
      unpinned: "已取消釘選，面板會跟著選取位置移動",
      cannotCollapseWhileSettingsOpen: "設定開啟時無法收合，請先關閉設定",
      cannotCollapseWhileFullscreen: "全螢幕時無法收合，請先還原視窗"
    },

    errors: {
      noModelProfile: "請先新增模型設定。",
      copyFailed: (detail) => `複製失敗：${detail}`
    },

    update: {
      eyebrow: "UPDATE AVAILABLE",
      bannerEyebrow: "UPDATES",
      banner: (appName, version) => `${appName} ${version} 已可更新`,
      installing: "安裝中…",
      install: "更新",
      title: "有新版本可以更新",
      notesLabel: "這次更新的內容",
      notesFor: (version) => `${version} 更新了什麼`,
      notesEmpty: "這個版本沒有附上更新說明。",
      warning: "更新會關閉目前的翻譯視窗並重新啟動，進行中的翻譯會中斷。",
      failed: (detail) => `更新失敗：${detail}`,
      downloading: "下載中…",
      restarting: "下載完成，即將重新啟動…",
      later: "稍後再說",
      installNow: "立即更新",
      checking: "檢查中…",
      check: "檢查更新",
      checkFailed: (detail) => `檢查失敗：${detail}`,
      notChecked: "尚未檢查",
      disabled: "此建置沒有更新頻道，需手動下載新版",
      upToDate: (version) => `沒有新版本，目前是 ${version}`,
      available: (version) => `有新版本 ${version} 可以更新`
    },

    settings: {
      eyebrow: "MODEL PROFILE",
      title: "模型設定",
      close: "關閉設定",
      profileName: "設定名稱",
      provider: "供應商",
      providerGroupNative: "原生供應商",
      providerGroupGateway: "閘道與地端",
      providerOllama: "Ollama（本機）",
      providerFedGpt: "公司內部 API",
      providerCustom: "自訂端點",
      endpoint: "API Base URL",
      endpointPlaceholder: "https://api.example.com",
      deployment: "部署名稱",
      modelName: "模型名稱",
      modelPlaceholder: "輸入模型 ID",
      openRouterPlaceholder: "例如 anthropic/claude-sonnet-4.5",
      apiKeyPlaceholder: "留白則保留既有金鑰",
      credentialNote: "金鑰只會儲存在 Windows 認證管理員。",
      save: "儲存設定",
      saving: "儲存中…",
      note: {
        fedgpt: "端點與模型名稱請依所屬單位提供的設定填寫。",
        anthropic: "使用 Anthropic Messages 串流 API。",
        gemini: "使用 Gemini streamGenerateContent API。",
        azure: "模型名稱請填入 Azure 的部署名稱。",
        custom: "端點需相容 OpenAI Chat Completions API。",
        openaiCompatible: "使用 OpenAI Chat Completions 相容介面。"
      },
      defaultProfileName: {
        ollama: "本機 Ollama",
        fedgpt: "公司內部 API",
        custom: "自訂端點"
      }
    },

    prefs: {
      language: "介面語言",
      languageLead: "面板、選單與系統匣要用哪個語言顯示。選好後會記住，下次啟動直接套用。",
      panel: "面板行為",
      showSource: "翻譯後展開原文",
      showSourceHint: "關閉時只顯示譯文，需要對照再手動展開。",
      autoCollapse: "點面板以外的地方時自動收合",
      autoCollapseHint: "收合成右下角的小標籤；翻譯或解釋進行中不會收合。",
      imageRecognition: "圖片辨識",
      imageRecognitionLead: "截圖翻譯與貼上圖片翻譯時，用什麼把圖片裡的字讀出來。",
      systemOcr: "系統 OCR",
      systemOcrHint:
        "Windows 內建辨識，全程在本機，圖片不會離開這台電腦。速度快，但對手寫字、藝術字、低解析度畫面較弱。",
      modelOcr: "模型辨識",
      modelOcrHint: "交給上方選用的模型讀圖，複雜版面與手寫字準確得多。",
      modelOcrHintEmphasis: "圖片會送到模型端點",
      autoOcr: "自動",
      autoOcrHint: "先試模型，模型不支援讀圖或出錯就退回系統 OCR。同樣",
      autoOcrHintEmphasis: "會把圖片送到模型端點",
      imagePrivacyNote:
        "截圖可能拍到畫面上的任何東西。選用模型辨識等於把那張圖上傳到你設定的端點，地端 Ollama 不出這台電腦，雲端服務則會離開。",
      selection: "選取取字",
      clipboardFallback: "問不到選取內容時，改用複製取字",
      clipboardFallbackHint:
        "讓 Electron、Qt、Java、終端機這類不交代文字的程式也能選取即譯。只在拖曳或連點圈字後才會執行，並會原樣還原你的剪貼簿。",
      updates: "UPDATES"
    }
  }
} as const satisfies Locale;
