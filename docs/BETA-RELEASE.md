# Beta 發布與自動更新

本文是操作步驟。機制為什麼這樣設計、如何被觸發，見 [SPEC.md 第 3 節](SPEC.md#3-更新機制)。

隨譯使用 Tauri 2 的簽章更新機制。Windows Authenticode 與 Tauri 更新簽章是兩件事；Beta 可先使用 Tauri 更新簽章，但公開擴大發布前仍建議替 NSIS 安裝程式加入 Authenticode。

---

## 先讀：純安裝版永遠收不到自動更新

`scripts\build-release.ps1` 不加參數（`-Local` 模式）產出的安裝檔，**裝到使用者機器上之後，無論你把更新檔發佈到哪裡都不會自動更新**。

原因在 `main.rs` 的 `configured_updater()`：

```rust
let (Some(endpoint), Some(public_key)) = (
    option_env!("FLOATRANS_UPDATE_ENDPOINT"),
    option_env!("FLOATRANS_UPDATE_PUBKEY"),
) else { return Ok(None); };
```

`option_env!` 是**編譯期**巨集——端點網址與公鑰在建置當下就燒進執行檔。`-Local` 建置時這兩個環境變數不存在，編出來的二進位檔裡根本沒有更新端點，更新檢查一律回報 `Disabled`（面板上顯示「此建置沒有更新頻道」）。

這不是設定問題，事後補不上去。**自動更新這條鏈必須從一個帶端點的建置開始，而那一版本身仍然要手動發給使用者安裝。**

---

## 發佈位置

要求只有兩個：

1. `latest.json` 走 **HTTPS**（Tauri 拒絕非安全協定，寫進去程式會啟動失敗而非停用更新）
2. `latest.json` 的網址**永久不變**（它被編譯進執行檔，換網址等於所有舊安裝失聯）

安裝檔本身（`-BaseUrl` 指向的位置）不必在受信任的主機上——用戶端會用公鑰驗簽章，驗不過就不安裝。但 `latest.json` 建議走 HTTPS 以防降版攻擊。

### 選項 A：GitHub Releases

用這個固定網址當端點：

```
https://github.com/<帳號>/<repo>/releases/latest/download/latest.json
```

`releases/latest/download/` 永遠指向最新一個 release 中的同名資產，不必每次改端點。

限制：repo 必須是 **public**。private repo 的資產下載需要 token，Tauri 的 updater 不會帶。

### 選項 B：靜態主機

Cloudflare R2 / Pages、S3 + CloudFront、Netlify、自架 nginx 都可以，把 `latest.json` 與 `*-setup.exe` 放在固定路徑即可。不想公開原始碼就選這個。

---

## 一次性建立金鑰

```powershell
.\scripts\new-update-key.ps1 -KeyPath C:\secure\floatrans-beta.key -Password '<密碼>'
```

腳本會拒絕把私鑰寫進專案目錄，並以 `--ci` 關閉互動提示（少了它會停在密碼提示，在 CI 上等同卡死）。

請把私鑰與密碼放在 CI 的秘密儲存區，不可進版控。公開金鑰作為建置環境變數 `FLOATRANS_UPDATE_PUBKEY`。

---

## 首次啟用自動更新

順序不能顛倒，第 2 步尤其要先想清楚。

1. **產生金鑰**（見上一節）
2. **決定 `latest.json` 的固定 HTTPS 網址**——這個值會寫死進執行檔，之後改不掉
3. **設定四個環境變數**
   - `FLOATRANS_UPDATE_ENDPOINT`：第 2 步決定的 `latest.json` 網址
   - `FLOATRANS_UPDATE_PUBKEY`：`*.key.pub` 的內容
   - `TAURI_SIGNING_PRIVATE_KEY`：私鑰內容或安全路徑
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私鑰密碼
4. **建置**

   ```powershell
   .\scripts\build-release.ps1 -WithUpdater -BaseUrl https://example.com/floatrans -Notes "更新說明"
   ```

   腳本會先檢查四個環境變數與 Node 版本，缺任何一項就中止，不會做出「看似能更新、實際上不能」的版本；建置後自動產生 `latest.json`。

   產物有三個，缺一更新都會失敗：`*-setup.exe`、`*-setup.exe.sig`、`latest.json`。

5. **上傳** `*-setup.exe` 與 `latest.json` 到 `-BaseUrl` 指向的位置，並確認 `latest.json` 真的出現在 `FLOATRANS_UPDATE_ENDPOINT` 那個網址上
6. **手動把這顆 `*-setup.exe` 發給使用者安裝** ← 斷點在這裡。舊的純安裝版收不到通知，只能靠這一次手動散布

---

## 之後每次改版

1. **先 bump 版本號**（`apps\desktop\src-tauri\tauri.conf.json` 的 `version`）
   更新判斷靠版本比較，沒 bump 的話用戶端會認定已是最新版，永遠不提示
2. `-WithUpdater` 重新建置（四個環境變數同上，端點維持不變）
3. 覆蓋上傳 `latest.json`，並上傳新的 `*-setup.exe`
4. 已裝過帶更新版的使用者，下次啟動就會收到提示

---

## 不可逆的決定

這幾項一旦做錯，代價是「所有已安裝的版本失聯，只能重新手動散布一次」：

| 項目 | 後果 |
|---|---|
| 端點網址 | 編譯期寫死，換網址＝舊安裝全部收不到更新 |
| 私鑰 | 遺失或覆蓋＝再也簽不出能被舊版接受的更新包（`new-update-key.ps1` 因此拒絕覆蓋既有金鑰） |
| 版本號未 bump | 用戶端認定已是最新版，不會提示 |
| 簽章比安裝檔舊 | 所有用戶端驗章失敗（`make-latest-json.ps1` 會擋下這種組合） |

---

## 不需要自動更新時

本機自用不必準備任何金鑰：

```powershell
.\scripts\build-release.ps1
```

這種建置的更新檢查會回報「此建置沒有更新頻道」，而不是假裝已是最新版。

---

## 用戶端行為

安裝版會在啟動時自動檢查更新（每 24 小時最多一次），找到新版本後主動跳出確認對話框，列出更新說明；同一版本被關掉後不再打擾，更高版本仍會提示。手動從模型設定對話框點「檢查更新」會跳過 24 小時節流。

Windows 安裝更新時會依 Tauri 的限制結束目前程式再重新啟動。免安裝版維持手動下載更新，不使用原地自動更新。

---

## 待辦

- **Authenticode 簽章**：目前 NSIS 安裝檔未簽章，使用者下載執行會撞到 Windows SmartScreen 警告。Tauri 更新簽章只保證更新包沒被掉包，擋不掉這個警告。
- **單一實例與更新重啟的互動**：更新流程會結束目前行程、由安裝程式重新啟動。理論上舊行程退出後互斥鎖即釋放，但尚未在帶更新的建置上實測過，第一次做 beta 時要特別走一次完整更新流程確認新行程沒有被 `tauri-plugin-single-instance` 擋掉。
