import type { Locale } from "./types";

/**
 * English.
 *
 * The product is called 隨譯 in Chinese; in English it goes by Anylingo, because a
 * name you cannot pronounce is not a name. The seal glyph 譯 stays everywhere —
 * that one is a logo, not a word.
 */
export const en = {
  id: "en",
  nativeName: "English",
  defaultTargetLanguage: "English",
  strings: {
    brand: {
      name: "Anylingo",
      slogan: "See it, select it, translate it",
      tagline: "Not just the browser — translate anything on your PC"
    },

    common: {
      translate: "Translate",
      stop: "Stop",
      cancel: "Cancel",
      close: "Close",
      expand: "Expand",
      collapse: "Collapse",
      clear: "Clear",
      back: "Back",
      copy: "Copy",
      copied: "Copied",
      pasteImage: "Paste image",
      submitHint: "Ctrl+Enter to translate",
      apiKey: "API key",
      keyPlus: "+",
      panelLabel: "Translation panel"
    },

    status: {
      idle: "Ready",
      streaming: "Translating",
      cancelled: "Stopped",
      waitingForModel: "Waiting for the model…"
    },

    titlebar: {
      home: "Home",
      homeHint: "Home: clear this round of source and translation, restore the window",
      capture: "Capture and translate",
      captureHint: "Capture and translate (Ctrl+Alt+R)",
      settings: "Model settings",
      pin: "Pin to this position",
      unpin: "Unpin position",
      pinHint: "Pin here — the panel stops following your selection",
      unpinHint: "Unpin — the panel follows wherever you select",
      fullscreen: "Fill the screen",
      restore: "Restore window size",
      collapseHint: "Collapse to the corner",
      expandHint: "Expand",
      hidePanel: "Close panel"
    },

    strip: {
      model: "Model",
      modelProfile: "Model profile",
      noModel: "No model configured",
      target: "Into",
      targetLanguage: "Target language"
    },

    empty: {
      pasteEyebrow: "PASTE TEXT",
      textToTranslate: "Text to translate",
      pastePlaceholder: "Paste or type the text to translate — a screenshot works too",
      selectHint: "Select text in any application, then press the shortcut.",
      captureHint: "When text cannot be selected, drag a box over it and translate what is on screen.",
      capture: "Capture and translate",
      pasteText: "Paste text",
      pasteImage: "Paste image",
      updateAvailable: (version) => `Version ${version} available`,
      upToDate: "Up to date"
    },

    card: {
      source: "Source",
      sourceEditable: "Source text — edit it and translate again",
      sourcePlaceholder: "Clear this to paste or type your own text",
      translation: "Translation",
      explanation: "Explanation",
      explanationLoading: "Asking the model…",
      explanationEmpty: "This model returned no explanation. Try again, or pick another model.",
      hide: "Hide"
    },

    footer: {
      pinned: "Pinned",
      pinnedHint: "The panel is pinned to this position",
      explain: "Explain",
      explaining: "Explaining…",
      explainHint: "Ask the model about terms, abbreviations and tone. The translation above is untouched.",
      copyTranslation: "Copy translation"
    },

    menu: {
      home: "Home",
      capture: "Capture and translate",
      pasteImage: "Translate pasted image",
      pin: "Pin to this position",
      unpin: "Unpin position",
      fullscreen: "Fill the screen",
      restore: "Restore window size",
      collapse: "Collapse to the corner",
      expand: "Expand panel",
      copyTranslation: "Copy translation",
      settings: "Model settings",
      hidePanel: "Hide panel"
    },

    mini: {
      expandToFull: "Expand to the full panel",
      dockLabel: (appName) => `Expand the ${appName} panel`,
      dockHint: (appName) => `${appName} · click to expand, drag to move`
    },

    region: {
      hint: "Drag a box over what you want translated · Esc to cancel",
      translateSelection: "Translate selection",
      searchSelection: "Search selection in your browser"
    },

    hints: {
      pinned: "Pinned — the panel no longer follows your selection",
      unpinned: "Unpinned — the panel follows wherever you select",
      cannotCollapseWhileSettingsOpen: "Cannot collapse while settings are open. Close them first.",
      cannotCollapseWhileFullscreen: "Cannot collapse in fullscreen. Restore the window first."
    },

    errors: {
      noModelProfile: "Add a model profile first.",
      copyFailed: (detail) => `Copy failed: ${detail}`
    },

    update: {
      eyebrow: "UPDATE AVAILABLE",
      bannerEyebrow: "UPDATES",
      banner: (appName, version) => `${appName} ${version} is available`,
      installing: "Installing…",
      install: "Update",
      title: "An update is available",
      notesLabel: "What is in this update",
      notesFor: (version) => `What changed in ${version}`,
      notesEmpty: "This version shipped without release notes.",
      warning: "Updating closes the panel and restarts the app. Any translation in progress will be cut off.",
      failed: (detail) => `Update failed: ${detail}`,
      downloading: "Downloading…",
      restarting: "Downloaded — restarting…",
      later: "Not now",
      installNow: "Update now",
      checking: "Checking…",
      check: "Check for updates",
      checkFailed: (detail) => `Check failed: ${detail}`,
      notChecked: "Not checked yet",
      disabled: "This build has no update channel — download new versions manually",
      upToDate: (version) => `No new version. You are on ${version}`,
      available: (version) => `Version ${version} is available`
    },

    settings: {
      eyebrow: "MODEL PROFILE",
      title: "Model settings",
      close: "Close settings",
      profileName: "Profile name",
      provider: "Provider",
      providerGroupNative: "Native providers",
      providerGroupGateway: "Gateways and local",
      providerOllama: "Ollama (local)",
      providerFedGpt: "Internal company API",
      providerCustom: "Custom endpoint",
      endpoint: "API base URL",
      endpointPlaceholder: "https://api.example.com",
      deployment: "Deployment name",
      modelName: "Model name",
      modelPlaceholder: "Enter a model ID",
      openRouterPlaceholder: "e.g. anthropic/claude-sonnet-4.5",
      apiKeyPlaceholder: "Leave blank to keep the existing key",
      credentialNote: "Keys are stored only in Windows Credential Manager.",
      save: "Save settings",
      saving: "Saving…",
      note: {
        fedgpt: "Use the endpoint and model name your organisation gave you.",
        anthropic: "Uses the Anthropic Messages streaming API.",
        gemini: "Uses the Gemini streamGenerateContent API.",
        azure: "Enter your Azure deployment name as the model name.",
        custom: "The endpoint must be compatible with the OpenAI Chat Completions API.",
        openaiCompatible: "Uses the OpenAI Chat Completions compatible interface."
      },
      defaultProfileName: {
        ollama: "Local Ollama",
        fedgpt: "Internal company API",
        custom: "Custom endpoint"
      }
    },

    prefs: {
      language: "Interface language",
      languageLead:
        "The language for the panel, menus and tray icon. Your choice is remembered and applied on the next launch.",
      panel: "Panel behaviour",
      showSource: "Show the source after translating",
      showSourceHint: "When off, only the translation is shown — expand it manually to compare.",
      autoCollapse: "Collapse when you click outside the panel",
      autoCollapseHint:
        "Collapses to a small icon in the corner. It will not collapse while translating or explaining.",
      imageRecognition: "Image recognition",
      imageRecognitionLead:
        "What reads the text out of an image, for screen capture and pasted pictures.",
      systemOcr: "System OCR",
      systemOcrHint:
        "Windows built-in recognition. Everything stays on this machine. Fast, but weaker on handwriting, stylised type and low-resolution screens.",
      modelOcr: "Model recognition",
      modelOcrHint: "Let the model above read the image. Far better on complex layouts and handwriting.",
      modelOcrHintEmphasis: "The image is sent to the model endpoint",
      autoOcr: "Automatic",
      autoOcrHint: "Try the model first; fall back to system OCR if it cannot read images or fails. This also",
      autoOcrHintEmphasis: "sends the image to the model endpoint",
      imagePrivacyNote:
        "A screen capture can catch anything that is on screen. Choosing model recognition uploads that image to the endpoint you configured — a local Ollama never leaves this machine, a cloud service does.",
      selection: "Reading the selection",
      clipboardFallback: "Fall back to copying when the selection cannot be read",
      clipboardFallbackHint:
        "Lets Electron, Qt, Java and terminal apps — the ones that do not report their text — work with select-to-translate. It only runs after a drag or a multi-click selection, and restores your clipboard exactly as it was.",
      updates: "UPDATES"
    }
  }
} as const satisfies Locale;
