# 隨譯 Anylingo

**所見所選，皆可譯。**

Windows 10/11 即時翻譯工具。翻譯不該綁在瀏覽器裡——隨譯在整台電腦通用，不限應用程式：

- **選得到的字** → 反白後點滑鼠旁的「譯」，或按 `Ctrl+Alt+T`。
- **選不到的畫面** → 按 `Ctrl+Alt+R` 框選，辨識文字後翻譯。PDF、掃描件、影像報告、影片字幕、遠端桌面、自繪介面都適用。

譯文透過 Ollama（地端）或 OpenAI 相容 API 串流回傳，出現在游標旁而不打斷手上的工作。

## 目前功能

- UI Automation 優先讀取選取文字，密碼欄位直接排除。
- 選取後顯示不搶焦點的小型動作按鈕。
- 截圖翻譯：框選螢幕範圍，用 Windows 內建 OCR 辨識文字後送出翻譯。
- `Ctrl+Alt+T` 翻譯選取文字、`Ctrl+Alt+R` 截圖翻譯兩組全域快捷鍵；被其他程式佔用時只會提示，不影響啟動。
- Ollama `/api/chat` 與 OpenAI-compatible `/v1/chat/completions` 串流介面。
- 模型設定存 SQLite；API Key 存 Windows Credential Manager。
- 譯文面板會開在游標旁邊，放不下時自動翻到另一側；收合中的面板收到譯文會自動展開。
- 面板恆常置頂；「◇」改為釘選位置，釘選後就不再跟著選取移動。
- 常駐系統匣、右下角收合、關閉時隱藏。
- NSIS 簽章更新接縫；啟動時自動檢查，使用者確認後安裝。

UI Automation 問不到選取內容時（Electron、Qt、Java、終端機這類自繪介面），會改用模擬 `Ctrl+C` 取字。只在拖曳或連點圈字之後才會執行，單純點按鈕不會觸發；剪貼簿上原有的每個格式都會先整份保存、取完原樣放回。不需要時可在設定裡關閉。

## 更新

安裝版從 0.2.1 起內建自動更新，來源是本 repo 的 [GitHub Releases](https://github.com/richshangwei/Anylingo/releases)。啟動時檢查一次，有新版會列出更新說明並詢問。0.2.1 之前的版本沒有更新能力，需手動安裝。

**這個 repo 必須維持公開**，否則已安裝的版本會全部斷更 —— 私有 repo 的 Release 資產要帶 token 才下載得到，而使用者端沒有 token。

發版流程與其強制檢查見 [docs/SPEC.md](docs/SPEC.md) 的「更新機制」，每版的更新說明見 [CHANGELOG.md](CHANGELOG.md)。

## 截圖翻譯

按 `Ctrl+Alt+R`、面板右上角的「▣」按鈕，或系統匣選單的「截圖翻譯」，畫面會壓暗並進入框選模式；拖曳出範圍後放開即開始辨識，按 `Esc` 或按右鍵取消。框選前隨譯會先收起自己的視窗，辨識完成再帶著結果回來。

辨識使用 Windows 內建的 `Windows.Media.Ocr`，截圖只留在記憶體中，不落地也不會送到模型端點；只有辨識出來的文字會照一般翻譯流程送出。小範圍會自動放大再辨識以提高準確度，中日韓文字之間多餘的斷詞空白也會接回來。

辨識語言取自系統的慣用語言清單。若出現「沒有可用的 OCR 語言套件」，請到 Windows 設定的「時間與語言 → 語言與地區」，在該語言的「語言選項」中安裝「光學字元辨識」選用功能。

## 本機開發

需求：Rust MSVC toolchain、Visual Studio Build Tools（Desktop development with C++）、Windows SDK，以及 Node `>=22 <24` 或 `>=24.14`。

```powershell
cd apps\desktop
npm install
npm run build
npm test
cd ..\..
cargo test --workspace
cargo run -p floatrans-desktop
```

預設模型設定為：

- 端點：`http://127.0.0.1:11434`
- 模型：`qwen3:8b`

可在右上角齒輪改成其他 Ollama 模型，或設定 OpenAI 相容服務。

## 規格文件

[docs/SPEC.md](docs/SPEC.md) 說明整體架構、翻譯流程、更新機制如何運作與發動，以及送給模型的提示詞怎麼組成。

## 發布與更新

```powershell
.\scripts\build-release.ps1                     # 純安裝版，不需金鑰，不會自動更新
.\scripts\build-release.ps1 -WithUpdater ...    # 附自動更新，需要簽章金鑰
```

發布前請依 [Beta 發布說明](docs/BETA-RELEASE.md) 建立更新簽章金鑰及 HTTPS 更新端點。私鑰不可加入版本控制。
