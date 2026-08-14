<#
.SYNOPSIS
    建置隨譯安裝版，可選擇是否帶自動更新能力。

.DESCRIPTION
    兩種模式：

    -Local（預設）
        不需要任何金鑰，產出可安裝但不會自動更新的版本。
        程式內 configured_updater() 讀不到端點與公鑰，更新檢查會回報 Disabled。

    -WithUpdater
        需要四個環境變數，產出安裝檔 + .sig，並可接著產生 latest.json。
        缺任何一個就直接中止，不會做出「看似能更新、實際上不能」的版本。

        更新說明不由參數帶入，一律從 CHANGELOG.md 讀該版本的條列。沒寫就不給建置：
        使用者按下「立即更新」之前唯一看得到的資訊就是那幾行字。

.EXAMPLE
    .\scripts\build-release.ps1
    .\scripts\build-release.ps1 -WithUpdater
#>
[CmdletBinding(DefaultParameterSetName = 'Local')]
param(
    [Parameter(ParameterSetName = 'Updater', Mandatory = $true)]
    [switch]$WithUpdater,

    # 存放 Release 資產的目錄網址。省略時由 FLOATRANS_UPDATE_ENDPOINT 推算，
    # 因為安裝檔與 latest.json 本來就該放在同一個 Release 底下。
    [Parameter(ParameterSetName = 'Updater')]
    [string]$BaseUrl
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release-common.ps1')
$repoRoot = Split-Path $PSScriptRoot -Parent
$desktop = Join-Path $repoRoot 'apps\desktop'

# vite build 在 Node 24.12.0 會無訊息崩潰，需要 >=22 <24 或 >=24.14
$nodeVersion = (& node --version) -replace '^v', ''
$parsed = [version]($nodeVersion -split '-')[0]
$ok = ($parsed -ge [version]'22.0.0' -and $parsed -lt [version]'24.0.0') -or ($parsed -ge [version]'24.14.0')
if (-not $ok) {
    throw "Node $nodeVersion 不符需求（>=22 <24 或 >=24.14）。24.12.0 會讓 vite build 無訊息崩潰。"
}
Write-Host "Node $nodeVersion" -ForegroundColor DarkGray

# 版本與更新說明要在動手建置之前就驗完。兩分鐘的建置跑完才發現沒寫更新說明，
# 只會讓人想把檢查拿掉。
$version = Get-ProjectVersion
Write-Host "版本 $version" -ForegroundColor DarkGray
if ($WithUpdater) {
    $Notes = Get-ReleaseNotes -Version $version
    Write-Host '更新說明（來自 CHANGELOG.md）：' -ForegroundColor DarkGray
    $Notes -split "`n" | ForEach-Object { Write-Host "  $_" -ForegroundColor DarkGray }
}

# 每次建置都從空的產物目錄開始。殘留檔案的危害是實際發生過的：
#   - 上次簽章建置的 .sig 配上這次重建的安裝檔 → 用戶端一律驗章失敗
#   - 改產品名後舊檔名的安裝檔仍在 → 兩個安裝檔並存，可能發布到錯的那個
$bundleDir = Join-Path $repoRoot 'target\release\bundle\nsis'
if (Test-Path $bundleDir) {
    Get-ChildItem $bundleDir -File | ForEach-Object {
        Write-Host "  清除舊產物：$($_.Name)" -ForegroundColor DarkYellow
        Remove-Item $_.FullName -Force
    }
}

Push-Location $desktop
try {
    if ($WithUpdater) {
        $missing = @(
            'FLOATRANS_UPDATE_ENDPOINT'
            'FLOATRANS_UPDATE_PUBKEY'
            'TAURI_SIGNING_PRIVATE_KEY'
            'TAURI_SIGNING_PRIVATE_KEY_PASSWORD'
        ) | Where-Object { -not (Get-Item "env:$_" -ErrorAction SilentlyContinue) }

        if ($missing.Count -gt 0) {
            throw @"
缺少建置自動更新版所需的環境變數：
  $($missing -join "`n  ")
請先執行 scripts\new-update-key.ps1 產生金鑰，並參考 docs\SPEC.md 的「更新機制」。
"@
        }
        # 安裝檔與 latest.json 放在同一個 GitHub Release 底下，所以資產目錄就是端點
        # 去掉檔名的那一段。要放到別處時才需要自己指定 -BaseUrl。
        #
        # 這裡刻意用「這一版的」Release 網址，而不是 releases/latest/download。
        # latest.json 走 latest 是為了讓端點永遠固定；安裝檔走版本號則是為了讓網址
        # 與它的簽章永遠成對——指向 latest 的安裝檔網址會在下一版發布的瞬間改指新檔，
        # 舊的簽章立刻對不上。
        if (-not $BaseUrl) {
            $endpoint = $env:FLOATRANS_UPDATE_ENDPOINT
            if ($endpoint -notmatch '^(?<repo>https://github\.com/[^/]+/[^/]+)/releases/latest/download/latest\.json$') {
                throw @"
無法從端點推算安裝檔的存放位置：$endpoint
預期格式為 https://github.com/<擁有者>/<repo>/releases/latest/download/latest.json
若更新檔不是放在 GitHub Release，請自行指定 -BaseUrl。
"@
            }
            $BaseUrl = "$($Matches['repo'])/releases/download/v$version"
        }

        Write-Host '建置模式：附自動更新' -ForegroundColor Cyan
        Write-Host "  端點：$env:FLOATRANS_UPDATE_ENDPOINT"
        Write-Host "  安裝檔：$BaseUrl"

        # 打包器在簽更新包時會驗證 tauri.conf.json 的 plugins.updater.pubkey，
        # 而該欄位在版控裡刻意留空（公鑰屬於發布環境，不該進版控）。
        # 執行期的 configured_updater() 覆寫來得太晚，所以這裡用 --config 補上。
        #
        # 只補 pubkey，不要補 endpoints：endpoints 一旦寫進設定，就會在外掛初始化時
        # 被驗證，格式不合（例如非 https）會讓程式「啟動就 panic」而不是單純停用更新。
        # 端點的唯一來源維持編譯期的 FLOATRANS_UPDATE_ENDPOINT，在執行期驗證即可。
        $updaterConf = [ordered]@{
            plugins = [ordered]@{
                updater = [ordered]@{
                    pubkey = $env:FLOATRANS_UPDATE_PUBKEY
                }
            }
        }
        $confPath = Join-Path ([System.IO.Path]::GetTempPath()) 'floatrans-updater.json'
        $updaterConf | ConvertTo-Json -Depth 6 | Set-Content -Path $confPath -Encoding UTF8
        try {
            npm run tauri build -- --config $confPath
            if ($LASTEXITCODE -ne 0) { throw "tauri build 失敗（exit $LASTEXITCODE）" }
        }
        finally {
            # 這個檔含公鑰，不留在暫存目錄
            Remove-Item $confPath -ErrorAction SilentlyContinue
        }

        & (Join-Path $PSScriptRoot 'make-latest-json.ps1') `
            -BaseUrl $BaseUrl `
            -Notes $Notes `
            -AssetName (Get-ReleaseAssetName -Version $version)
    }
    else {
        Write-Host '建置模式：純安裝版（不會自動更新）' -ForegroundColor Cyan
        # 沒有私鑰卻要求更新產物會讓建置失敗，所以這裡明確關掉
        $override = Join-Path ([System.IO.Path]::GetTempPath()) 'floatrans-no-updater.json'
        '{"bundle":{"createUpdaterArtifacts":false}}' | Set-Content -Path $override -Encoding UTF8
        npm run tauri build -- --config $override
        if ($LASTEXITCODE -ne 0) { throw "tauri build 失敗（exit $LASTEXITCODE）" }
        Remove-Item $override -ErrorAction SilentlyContinue
    }
}
finally {
    Pop-Location
}

$bundle = Join-Path $repoRoot 'target\release\bundle\nsis'
Write-Host ''
Write-Host '產物：' -ForegroundColor Green
Get-ChildItem $bundle -File | ForEach-Object {
    Write-Host ("  {0}  ({1:N2} MB)" -f $_.Name, ($_.Length / 1MB))
}
