import type { Locale } from "./types";

/**
 * 日本語。
 *
 * 製品名は日本語圏でもローマ字の Anylingo を使う。「隨譯」は日本語の常用漢字に
 * 無い字で、読み方が分からない名前は名前として働かないため。印章の「譯」だけは
 * ロゴなのでそのまま残す。
 */
export const ja = {
  id: "ja",
  nativeName: "日本語",
  defaultTargetLanguage: "日本語",
  strings: {
    brand: {
      name: "Anylingo",
      slogan: "見えるものは、すべて訳せる",
      tagline: "ブラウザだけでなく、パソコン上のあらゆる文字を翻訳"
    },

    common: {
      translate: "翻訳",
      stop: "停止",
      cancel: "キャンセル",
      close: "閉じる",
      expand: "展開",
      collapse: "折りたたむ",
      clear: "クリア",
      back: "戻る",
      copy: "コピー",
      copied: "コピーしました",
      pasteImage: "画像を貼り付け",
      submitHint: "Ctrl＋Enter で翻訳",
      apiKey: "API キー",
      keyPlus: "＋",
      panelLabel: "翻訳パネル"
    },

    status: {
      idle: "待機中",
      streaming: "翻訳中",
      cancelled: "停止しました",
      waitingForModel: "モデルの応答を待っています…"
    },

    titlebar: {
      home: "ホームに戻る",
      homeHint: "ホームに戻る：今回の原文と訳文を消去し、ウィンドウを元のサイズに戻します",
      capture: "画面を切り取って翻訳",
      captureHint: "画面を切り取って翻訳（Ctrl＋Alt＋R）",
      settings: "モデル設定",
      pin: "現在の位置に固定",
      unpin: "位置の固定を解除",
      pinHint: "現在の位置に固定し、選択に追従しないようにします",
      unpinHint: "固定を解除し、選択した位置にパネルが移動するようにします",
      fullscreen: "全画面に拡大",
      restore: "ウィンドウサイズに戻す",
      collapseHint: "右下に折りたたむ",
      expandHint: "展開",
      hidePanel: "パネルを閉じる"
    },

    strip: {
      model: "モデル",
      modelProfile: "モデル設定プロファイル",
      noModel: "モデル未設定",
      target: "翻訳先",
      targetLanguage: "翻訳先の言語"
    },

    empty: {
      pasteEyebrow: "テキストを貼り付け",
      textToTranslate: "翻訳するテキスト",
      pastePlaceholder: "翻訳したいテキストを貼り付けるか入力してください。スクリーンショットの貼り付けも可能です",
      selectHint: "任意のアプリで文字を選択し、ショートカットキーを押してください。",
      captureHint: "文字を選択できないときは、範囲を囲んで画面上の文字を読み取ってから翻訳します。",
      capture: "画面を切り取って翻訳",
      pasteText: "テキストを貼り付けて翻訳",
      pasteImage: "画像を貼り付けて翻訳",
      updateAvailable: (version) => `新しいバージョン ${version} があります`,
      upToDate: "最新版です"
    },

    card: {
      source: "原文",
      sourceEditable: "原文（編集して再翻訳できます）",
      sourcePlaceholder: "消去すると、自分でテキストを貼り付けたり入力したりできます",
      translation: "訳文",
      explanation: "解説",
      explanationLoading: "モデルに解説を依頼しています…",
      explanationEmpty: "このモデルからは解説が返りませんでした。もう一度試すか、別のモデルをお試しください。",
      hide: "閉じる"
    },

    footer: {
      pinned: "固定中",
      pinnedHint: "パネルは現在の位置に固定されています",
      explain: "解説",
      explaining: "解説中…",
      explainHint: "専門用語・略語・ニュアンスの補足をモデルに依頼します。上の訳文は変わりません",
      copyTranslation: "訳文をコピー"
    },

    menu: {
      home: "ホームに戻る",
      capture: "画面を切り取って翻訳",
      pasteImage: "画像を貼り付けて翻訳",
      pin: "現在の位置に固定",
      unpin: "位置の固定を解除",
      fullscreen: "全画面に拡大",
      restore: "ウィンドウサイズに戻す",
      collapse: "右下に折りたたむ",
      expand: "パネルを展開",
      copyTranslation: "訳文をコピー",
      settings: "モデル設定",
      hidePanel: "パネルを隠す"
    },

    mini: {
      expandToFull: "通常のパネルに展開",
      dockLabel: (appName) => `${appName}の翻訳パネルを展開`,
      dockHint: (appName) => `${appName}　·　クリックで展開、ドラッグで移動`
    },

    region: {
      hint: "翻訳したい範囲をドラッグして囲んでください　·　Esc でキャンセル",
      translateSelection: "選択した文字を翻訳",
      searchSelection: "既定のブラウザーで選択した文字を検索"
    },

    hints: {
      pinned: "位置を固定しました。パネルは選択に追従しません",
      unpinned: "固定を解除しました。パネルは選択した位置に移動します",
      cannotCollapseWhileSettingsOpen: "設定を開いている間は折りたためません。先に設定を閉じてください",
      cannotCollapseWhileFullscreen: "全画面では折りたためません。先にウィンドウサイズに戻してください"
    },

    errors: {
      noModelProfile: "先にモデル設定を追加してください。",
      copyFailed: (detail) => `コピーに失敗しました：${detail}`
    },

    update: {
      eyebrow: "UPDATE AVAILABLE",
      bannerEyebrow: "UPDATES",
      banner: (appName, version) => `${appName} ${version} に更新できます`,
      installing: "インストール中…",
      install: "更新",
      title: "新しいバージョンがあります",
      notesLabel: "今回の更新内容",
      notesFor: (version) => `${version} の変更点`,
      notesEmpty: "このバージョンには更新内容が添えられていません。",
      warning: "更新すると翻訳ウィンドウを閉じて再起動します。実行中の翻訳は中断されます。",
      failed: (detail) => `更新に失敗しました：${detail}`,
      downloading: "ダウンロード中…",
      restarting: "ダウンロード完了。まもなく再起動します…",
      later: "後で",
      installNow: "今すぐ更新",
      checking: "確認中…",
      check: "更新を確認",
      checkFailed: (detail) => `確認に失敗しました：${detail}`,
      notChecked: "未確認",
      disabled: "このビルドには更新チャンネルがありません。新版は手動でダウンロードしてください",
      upToDate: (version) => `新しいバージョンはありません。現在は ${version} です`,
      available: (version) => `新しいバージョン ${version} に更新できます`
    },

    settings: {
      eyebrow: "MODEL PROFILE",
      title: "モデル設定",
      close: "設定を閉じる",
      profileName: "設定名",
      provider: "プロバイダー",
      providerGroupNative: "ネイティブ対応",
      providerGroupGateway: "ゲートウェイ・ローカル",
      providerOllama: "Ollama（ローカル）",
      providerFedGpt: "社内 API",
      providerCustom: "カスタムエンドポイント",
      endpoint: "API ベース URL",
      endpointPlaceholder: "https://api.example.com",
      deployment: "デプロイ名",
      modelName: "モデル名",
      modelPlaceholder: "モデル ID を入力",
      openRouterPlaceholder: "例：anthropic/claude-sonnet-4.5",
      apiKeyPlaceholder: "空欄にすると既存のキーを保持します",
      credentialNote: "キーは Windows 資格情報マネージャーにのみ保存されます。",
      save: "設定を保存",
      saving: "保存中…",
      note: {
        fedgpt: "エンドポイントとモデル名は、所属組織から提供された設定に従って入力してください。",
        anthropic: "Anthropic Messages ストリーミング API を使用します。",
        gemini: "Gemini streamGenerateContent API を使用します。",
        azure: "モデル名には Azure のデプロイ名を入力してください。",
        custom: "エンドポイントは OpenAI Chat Completions API 互換である必要があります。",
        openaiCompatible: "OpenAI Chat Completions 互換のインターフェースを使用します。"
      },
      defaultProfileName: {
        ollama: "ローカル Ollama",
        fedgpt: "社内 API",
        custom: "カスタムエンドポイント"
      }
    },

    prefs: {
      language: "表示言語",
      languageLead: "パネル・メニュー・通知領域の表示言語です。選ぶと記憶され、次回の起動時にそのまま適用されます。",
      panel: "パネルの動作",
      showSource: "翻訳後に原文を表示する",
      showSourceHint: "オフのときは訳文だけを表示します。見比べたいときは手動で展開してください。",
      autoCollapse: "パネルの外をクリックしたら自動的に折りたたむ",
      autoCollapseHint: "右下の小さなアイコンに折りたたみます。翻訳中・解説中は折りたたみません。",
      imageRecognition: "画像の文字認識",
      imageRecognitionLead: "画面の切り取り翻訳と画像の貼り付け翻訳で、画像内の文字を何で読み取るかを選びます。",
      systemOcr: "システム OCR",
      systemOcrHint:
        "Windows 内蔵の認識機能です。すべてこのパソコン内で完結し、画像は外に出ません。高速ですが、手書き文字・装飾文字・低解像度の画面には弱めです。",
      modelOcr: "モデルによる認識",
      modelOcrHint: "上で選んだモデルに画像を読ませます。複雑なレイアウトや手書き文字の精度が大きく上がります。",
      modelOcrHintEmphasis: "画像はモデルのエンドポイントに送信されます",
      autoOcr: "自動",
      autoOcrHint: "まずモデルを試し、読めない場合やエラー時はシステム OCR に戻します。この場合も",
      autoOcrHintEmphasis: "画像がモデルのエンドポイントに送信されます",
      imagePrivacyNote:
        "切り取った画面には、そのとき表示されていたものが何でも写り込みます。モデルによる認識を選ぶと、その画像を設定したエンドポイントにアップロードすることになります。ローカルの Ollama ならこのパソコンから出ませんが、クラウドサービスの場合は外に出ます。",
      selection: "選択文字の読み取り",
      clipboardFallback: "選択内容を取得できないときはコピーで読み取る",
      clipboardFallbackHint:
        "Electron・Qt・Java・ターミナルなど、選択中の文字を通知しないアプリでも選択即翻訳を使えるようにします。ドラッグまたは連続クリックで選択したときだけ動作し、クリップボードは元どおりに復元します。",
      updates: "UPDATES"
    }
  }
} as const satisfies Locale;
