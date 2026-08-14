<#
.SYNOPSIS
    產生隨譯自動更新用的簽章金鑰對（一次性作業）。

.DESCRIPTION
    更新包的信任來源是這組金鑰，不是 HTTPS。私鑰外流等於任何人都能對安裝版
    推送任意程式，因此本腳本拒絕把私鑰寫進工作區。

.EXAMPLE
    .\scripts\new-update-key.ps1 -KeyPath C:\secure\floatrans-beta.key
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$KeyPath,

    # 私鑰密碼。不給就會停在互動提示，在 CI 上等同卡死，所以這裡強制要求明確指定。
    # 要產生無密碼金鑰請傳空字串（僅建議用於本機測試）。
    [Parameter(Mandatory = $true)]
    [AllowEmptyString()]
    [string]$Password
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path $PSScriptRoot -Parent

$full = [System.IO.Path]::GetFullPath($KeyPath)
$rootFull = [System.IO.Path]::GetFullPath($repoRoot)
if ($full.StartsWith($rootFull, [StringComparison]::OrdinalIgnoreCase)) {
    throw "私鑰不可放在專案內（$full）。請改放到工作區之外，例如 C:\secure\。"
}
if (Test-Path $full) {
    throw "$full 已存在。請改用其他路徑，覆蓋舊金鑰會讓已安裝的版本再也收不到更新。"
}

$keyDir = Split-Path $full -Parent
if (-not (Test-Path $keyDir)) { New-Item -ItemType Directory -Path $keyDir -Force | Out-Null }

Push-Location (Join-Path $repoRoot 'apps\desktop')
try {
    # --ci 關掉互動提示，--password 明確帶入，兩者缺一在無終端機環境會卡住
    npx tauri signer generate --ci --password $Password -w $full
    if ($LASTEXITCODE -ne 0) { throw "tauri signer generate 失敗（exit $LASTEXITCODE）" }
}
finally {
    Pop-Location
}

$pub = "$full.pub"
Write-Host ''
Write-Host '金鑰已產生：' -ForegroundColor Green
Write-Host "  私鑰：$full        <- 絕不可進版控，請放進 CI secret"
Write-Host "  公鑰：$pub"
Write-Host ''
Write-Host '建置時需要的環境變數：' -ForegroundColor Cyan
Write-Host '  FLOATRANS_UPDATE_PUBKEY          = 上面公鑰檔的內容'
Write-Host '  FLOATRANS_UPDATE_ENDPOINT        = latest.json 的 HTTPS 網址'
Write-Host '  TAURI_SIGNING_PRIVATE_KEY        = 私鑰檔的內容（或路徑）'
Write-Host '  TAURI_SIGNING_PRIVATE_KEY_PASSWORD = 產生時輸入的密碼'
