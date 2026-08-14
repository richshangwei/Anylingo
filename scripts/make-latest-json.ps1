<#
.SYNOPSIS
    從建置產物組出更新清單 latest.json。

.DESCRIPTION
    隨譯的更新流程由三個檔案構成：
      1. *-setup.exe      安裝檔，同時也是更新包
      2. *-setup.exe.sig  該安裝檔的簽章，由 TAURI_SIGNING_PRIVATE_KEY 產生
      3. latest.json      指向 (1) 的清單，signature 欄位放 (2) 的內容

    程式啟動時抓 latest.json，比對版本後下載 url，並用建置時燒進二進位檔的
    公鑰驗證 signature。驗不過就不安裝，所以 url 不一定要在受信任的主機上，
    但 latest.json 本身建議走 HTTPS 以免被降版攻擊。

.EXAMPLE
    .\scripts\make-latest-json.ps1 -BaseUrl https://example.com/floatrans -Notes "修正截圖翻譯"
#>
[CmdletBinding()]
param(
    # 存放安裝檔的目錄網址，不含檔名，結尾有無斜線皆可
    [Parameter(Mandatory = $true)]
    [string]$BaseUrl,

    [string]$Notes = '',

    # 安裝檔在發布位置上的檔名。省略就沿用本機的檔名。
    #
    # 產品名是中文，而 GitHub 會把 Release 資產檔名裡的非 ASCII 字元換成點：
    # 「隨譯_0.2.0_x64-setup.exe」上傳後叫「.._0.2.0_x64-setup.exe」，
    # 清單裡的網址就對不上，更新一律 404。所以上傳與清單都用 ASCII 檔名。
    [string]$AssetName,

    [string]$BundleDir,

    [string]$OutFile
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent
if (-not $BundleDir) { $BundleDir = Join-Path $repoRoot 'target\release\bundle\nsis' }
if (-not $OutFile) { $OutFile = Join-Path $BundleDir 'latest.json' }

if (-not (Test-Path $BundleDir)) { throw "找不到建置產物目錄：$BundleDir。請先執行 scripts\build-release.ps1。" }

$setup = Get-ChildItem $BundleDir -Filter '*-setup.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $setup) { throw "在 $BundleDir 找不到 *-setup.exe。" }

$sigFile = "$($setup.FullName).sig"
if (-not (Test-Path $sigFile)) {
    throw @"
找不到簽章檔：$sigFile
代表這次建置沒有產生更新包。請確認：
  - tauri.conf.json 的 bundle.createUpdaterArtifacts 為 true（且未被 --config 覆寫掉）
  - 建置時已設定 TAURI_SIGNING_PRIVATE_KEY 與 TAURI_SIGNING_PRIVATE_KEY_PASSWORD
"@
}

# 簽章比安裝檔舊，代表安裝檔在簽章之後被重建過（例如中間跑了一次無更新版建置）。
# 這種組合上傳出去，所有用戶端都會驗章失敗，而且錯誤訊息很難追。
$sigInfo = Get-Item $sigFile
if ($sigInfo.LastWriteTime -lt $setup.LastWriteTime) {
    throw @"
簽章檔比安裝檔舊，簽章已對不上：
  安裝檔 $($setup.Name)  $($setup.LastWriteTime)
  簽章   $($sigInfo.Name)  $($sigInfo.LastWriteTime)
這通常是安裝檔被重新建置、但沒有重新簽章造成的。
請重新執行 scripts\build-release.ps1 -WithUpdater。
"@
}

# 版本以 tauri.conf.json 為準，檔名只是它的產物
$conf = Get-Content (Join-Path $repoRoot 'apps\desktop\src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$version = $conf.version

$signature = (Get-Content $sigFile -Raw).Trim()
if (-not $AssetName) { $AssetName = $setup.Name }
$url = "$($BaseUrl.TrimEnd('/'))/$([uri]::EscapeDataString($AssetName))"

$manifest = [ordered]@{
    version   = $version
    notes     = $Notes
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = $signature
            url       = $url
        }
    }
}

$json = $manifest | ConvertTo-Json -Depth 6
Set-Content -Path $OutFile -Value $json -Encoding UTF8

Write-Host "latest.json 已產生：$OutFile" -ForegroundColor Green
Write-Host ''
Write-Host $json
Write-Host ''
Write-Host '發布時要一起上傳（兩者缺一，更新都會失敗）：' -ForegroundColor Cyan
Write-Host "  $($setup.FullName)  -> 上傳後須命名為 $AssetName"
Write-Host "  $OutFile"
Write-Host ''
Write-Host "並確認 $url 真的可以下載得到。" -ForegroundColor Yellow
Write-Host '（scripts\publish-github-release.ps1 會把改名與上傳一起做掉。）' -ForegroundColor DarkGray
