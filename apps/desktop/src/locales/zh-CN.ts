import type { Locale } from "./types";

/**
 * 简体中文。
 *
 * 不是把 `zh-TW.ts` 做一次字形转换就算数——两岸的用词差别在这个程式里正好落在
 * 最常出现的几个词上：「軟體／软件」「介面／界面」「檔案／文件」「模型設定檔／
 * 模型配置」「剪貼簿／剪贴板」「快捷鍵／快捷键」「辨識／识别」。整份都按简体
 * 中文的习惯用词重写，而不只是转字。
 */
export const zhCN = {
  id: "zh-CN",
  nativeName: "简体中文",
  defaultTargetLanguage: "简体中文",
  strings: {
    brand: {
      name: "随译",
      slogan: "所见所选，皆可译",
      tagline: "不限网页，整台电脑的字都能译"
    },

    common: {
      translate: "翻译",
      stop: "停止",
      cancel: "取消",
      close: "关闭",
      expand: "展开",
      collapse: "收起",
      clear: "清空",
      back: "返回",
      copy: "复制",
      copied: "已复制",
      pasteImage: "粘贴图片",
      submitHint: "Ctrl＋Enter 翻译",
      apiKey: "API 密钥",
      keyPlus: "＋",
      panelLabel: "翻译面板"
    },

    status: {
      idle: "待命",
      streaming: "翻译中",
      cancelled: "已停止",
      waitingForModel: "正在等待模型响应…"
    },

    titlebar: {
      home: "回到首页",
      homeHint: "回到首页：清空这一轮的原文与译文，还原窗口",
      capture: "截图翻译",
      captureHint: "截图翻译（Ctrl＋Alt＋R）",
      settings: "模型配置",
      pin: "固定当前位置",
      unpin: "取消固定位置",
      pinHint: "固定当前位置，面板不再跟随移动",
      unpinHint: "取消固定，面板会跟随选中位置移动",
      fullscreen: "放大至全屏",
      restore: "还原窗口大小",
      collapseHint: "收起到右下角",
      expandHint: "展开",
      hidePanel: "关闭面板"
    },

    strip: {
      model: "模型",
      modelProfile: "模型配置",
      noModel: "尚未配置模型",
      target: "目标",
      targetLanguage: "目标语言"
    },

    empty: {
      pasteEyebrow: "粘贴文本",
      textToTranslate: "要翻译的文本",
      pastePlaceholder: "粘贴或输入要翻译的文本，也可以直接粘贴截图",
      selectHint: "在任意程序中选中文字后，按下快捷键。",
      captureHint: "选不到文字时，改用框选截图，识别画面上的字再翻译。",
      capture: "截图翻译",
      pasteText: "粘贴文本翻译",
      pasteImage: "粘贴图片翻译",
      updateAvailable: (version) => `有新版本 ${version}`,
      upToDate: "没有新版本"
    },

    card: {
      source: "原文",
      sourceEditable: "原文，可修改后重新翻译",
      sourcePlaceholder: "清空后可以自己粘贴或输入要翻译的文本",
      translation: "译文",
      explanation: "解释",
      explanationLoading: "正在请模型说明…",
      explanationEmpty: "这个模型没有返回说明，可以再试一次或换一个模型。",
      hide: "收起"
    },

    footer: {
      pinned: "已固定",
      pinnedHint: "面板已固定在当前位置",
      explain: "解释",
      explaining: "解释中…",
      explainHint: "请模型补充说明术语、缩写与语气，不影响上方译文",
      copyTranslation: "复制译文"
    },

    menu: {
      home: "回到首页",
      capture: "截图翻译",
      pasteImage: "粘贴图片翻译",
      pin: "固定当前位置",
      unpin: "取消固定位置",
      fullscreen: "放大至全屏",
      restore: "还原窗口大小",
      collapse: "收起到右下角",
      expand: "展开面板",
      copyTranslation: "复制译文",
      settings: "模型配置",
      hidePanel: "隐藏面板"
    },

    mini: {
      expandToFull: "展开为完整面板",
      dockLabel: (appName) => `展开${appName}翻译面板`,
      dockHint: (appName) => `${appName}　·　点一下展开，拖动可移动`
    },

    region: {
      hint: "拖动框选要翻译的画面范围　·　Esc 取消",
      translateSelection: "翻译选中文字",
      searchSelection: "用默认浏览器搜索选中文字"
    },

    hints: {
      pinned: "已固定位置，面板不再跟随选中移动",
      unpinned: "已取消固定，面板会跟随选中位置移动",
      cannotCollapseWhileSettingsOpen: "设置打开时无法收起，请先关闭设置",
      cannotCollapseWhileFullscreen: "全屏时无法收起，请先还原窗口"
    },

    errors: {
      noModelProfile: "请先添加模型配置。",
      copyFailed: (detail) => `复制失败：${detail}`
    },

    update: {
      eyebrow: "UPDATE AVAILABLE",
      bannerEyebrow: "UPDATES",
      banner: (appName, version) => `${appName} ${version} 已可更新`,
      installing: "安装中…",
      install: "更新",
      title: "有新版本可以更新",
      notesLabel: "这次更新的内容",
      notesFor: (version) => `${version} 更新了什么`,
      notesEmpty: "这个版本没有附上更新说明。",
      warning: "更新会关闭当前的翻译窗口并重新启动，进行中的翻译会中断。",
      failed: (detail) => `更新失败：${detail}`,
      downloading: "下载中…",
      restarting: "下载完成，即将重新启动…",
      later: "稍后再说",
      installNow: "立即更新",
      checking: "检查中…",
      check: "检查更新",
      checkFailed: (detail) => `检查失败：${detail}`,
      notChecked: "尚未检查",
      disabled: "此版本没有更新通道，需手动下载新版",
      upToDate: (version) => `没有新版本，当前是 ${version}`,
      available: (version) => `有新版本 ${version} 可以更新`
    },

    settings: {
      eyebrow: "MODEL PROFILE",
      title: "模型配置",
      close: "关闭设置",
      profileName: "配置名称",
      provider: "服务商",
      providerGroupNative: "原生服务商",
      providerGroupGateway: "网关与本地",
      providerOllama: "Ollama（本地）",
      providerFedGpt: "公司内部 API",
      providerCustom: "自定义端点",
      endpoint: "API Base URL",
      endpointPlaceholder: "https://api.example.com",
      deployment: "部署名称",
      modelName: "模型名称",
      modelPlaceholder: "输入模型 ID",
      openRouterPlaceholder: "例如 anthropic/claude-sonnet-4.5",
      apiKeyPlaceholder: "留空则保留现有密钥",
      credentialNote: "密钥只会保存在 Windows 凭据管理器。",
      save: "保存设置",
      saving: "保存中…",
      note: {
        fedgpt: "端点与模型名称请按所属单位提供的配置填写。",
        anthropic: "使用 Anthropic Messages 流式 API。",
        gemini: "使用 Gemini streamGenerateContent API。",
        azure: "模型名称请填写 Azure 的部署名称。",
        custom: "端点需兼容 OpenAI Chat Completions API。",
        openaiCompatible: "使用 OpenAI Chat Completions 兼容接口。"
      },
      defaultProfileName: {
        ollama: "本地 Ollama",
        fedgpt: "公司内部 API",
        custom: "自定义端点"
      }
    },

    prefs: {
      language: "界面语言",
      languageLead: "面板、菜单与托盘要用哪种语言显示。选好后会记住，下次启动直接套用。",
      panel: "面板行为",
      showSource: "翻译后展开原文",
      showSourceHint: "关闭时只显示译文，需要对照再手动展开。",
      autoCollapse: "点面板以外的地方时自动收起",
      autoCollapseHint: "收起成右下角的小图标；翻译或解释进行中不会收起。",
      imageRecognition: "图片识别",
      imageRecognitionLead: "截图翻译与粘贴图片翻译时，用什么把图片里的字读出来。",
      systemOcr: "系统 OCR",
      systemOcrHint:
        "Windows 内置识别，全程在本机，图片不会离开这台电脑。速度快，但对手写字、艺术字、低分辨率画面较弱。",
      modelOcr: "模型识别",
      modelOcrHint: "交给上方选用的模型读图，复杂版面与手写字准确得多。",
      modelOcrHintEmphasis: "图片会发送到模型端点",
      autoOcr: "自动",
      autoOcrHint: "先试模型，模型不支持读图或出错就退回系统 OCR。同样",
      autoOcrHintEmphasis: "会把图片发送到模型端点",
      imagePrivacyNote:
        "截图可能拍到画面上的任何东西。选用模型识别等于把那张图上传到你配置的端点，本地 Ollama 不出这台电脑，云端服务则会离开。",
      selection: "选中取字",
      clipboardFallback: "取不到选中内容时，改用复制取字",
      clipboardFallbackHint:
        "让 Electron、Qt、Java、终端这类不交代文字的程序也能选中即译。只在拖动或连点选词后才会执行，并会原样还原你的剪贴板。",
      updates: "UPDATES"
    }
  }
} as const satisfies Locale;
