<#
.SYNOPSIS
    把建置產物發成 GitHub Release，作為自動更新的來源。

.DESCRIPTION
    走 GitHub REST API，不需要安裝 gh CLI。token 依序從這兩處取：

        1. 環境變數 GITHUB_TOKEN
        2. -TokenPath 指向的檔案（預設 C:\secure\github-token.txt）

    放檔案是為了不必把 token 打進終端機——打過的指令會留在 PowerShell 的歷史紀錄裡。

    目標 repo 必須是**公開**的：私有 repo 的 Release 資產一定要帶 token 才下載得到，
    而使用者電腦上的隨譯沒有 token。發布完會用匿名身分實測一次端點，就是在驗這件事。

    每個版本只能發一次。tag 已存在就中止——重發同一個版本號，已安裝的用戶端不會認為
    有更新（版本沒變大），卻可能拿到不同的檔案與簽章，是最難查的一種故障。

.EXAMPLE
    $env:GITHUB_TOKEN = 'ghp_...'
    .\scripts\publish-github-release.ps1
#>
[CmdletBinding()]
param(
    # 放更新檔的 repo。必須公開，否則使用者端抓不到更新清單。
    [string]$Repo = 'richshangwei/Anylingo',

    # token 檔。內容就是 token 本身，前後空白會被去掉。
    [string]$TokenPath = 'C:\secure\github-token.txt',

    # 只做檢查與顯示，不真的建立 Release。
    [switch]$WhatIfOnly
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release-common.ps1')

$repoRoot = Split-Path $PSScriptRoot -Parent
$bundleDir = Join-Path $repoRoot 'target\release\bundle\nsis'

$token = $env:GITHUB_TOKEN
if (-not $token -and (Test-Path $TokenPath)) {
    $token = (Get-Content $TokenPath -Raw).Trim()
}
if (-not $token -and -not $WhatIfOnly) {
    throw @"
找不到 GitHub token（環境變數 GITHUB_TOKEN 未設定，$TokenPath 也不存在）。

請到 https://github.com/settings/personal-access-tokens 產生一個 fine-grained token：
  Repository access : 只勾 $Repo
  Permissions       : Contents = Read and write
然後把 token 存成檔案（不要打進終端機，指令會留在 PowerShell 歷史紀錄裡）：
  Set-Content -Path '$TokenPath' -Value '<token>' -NoNewline
"@
}

$version = Get-ProjectVersion
$tag = "v$version"
$notes = Get-ReleaseNotes -Version $version
$assetName = Get-ReleaseAssetName -Version $version

$setup = Get-ChildItem $bundleDir -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
$manifest = Join-Path $bundleDir 'latest.json'

if (-not $setup) { throw "$bundleDir 裡沒有 *-setup.exe。請先執行 scripts\build-release.ps1 -WithUpdater。" }
if (-not (Test-Path $manifest)) { throw "找不到 $manifest。請先執行 scripts\build-release.ps1 -WithUpdater。" }

# latest.json 的版本必須就是這次要發的版本。對不上代表產物是上一次建置留下來的，
# 發出去會讓所有用戶端下載到版本與簽章不成對的檔案。
$manifestVersion = (Get-Content $manifest -Raw | ConvertFrom-Json).version
if ($manifestVersion -ne $version) {
    throw "latest.json 寫的是 $manifestVersion，專案版本卻是 $version。產物是舊的，請重新建置。"
}

Write-Host "發布 $tag 到 $Repo" -ForegroundColor Cyan
Write-Host "  安裝檔：$($setup.Name)  ->  $assetName"
Write-Host "  清單　：latest.json"
Write-Host '  更新說明：'
$notes -split "`n" | ForEach-Object { Write-Host "    $_" }

if ($WhatIfOnly) {
    Write-Host ''
    Write-Host '（-WhatIfOnly：以上都只是檢查，沒有真的發布。）' -ForegroundColor Yellow
    return
}

$headers = @{
    Authorization          = "Bearer $token"
    Accept                 = 'application/vnd.github+json'
    'X-GitHub-Api-Version' = '2022-11-28'
}

# 同一個版本不能發第二次
$existing = try {
    Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/tags/$tag" -Headers $headers
}
catch { $null }
if ($existing) {
    throw @"
$Repo already has a release tagged $tag.

同一個版本號發第二次，已安裝的用戶端不會認為有更新（版本沒變大），
卻可能拿到與舊簽章不成對的檔案。請先用 scripts\set-version.ps1 提版本，
並在 CHANGELOG.md 補上該版本的條列。
"@
}

$release = Invoke-RestMethod -Method Post -Uri "https://api.github.com/repos/$Repo/releases" -Headers $headers -ContentType 'application/json' -Body (
    [ordered]@{
        tag_name = $tag
        name     = "隨譯 Anylingo $version"
        body     = $notes
        draft    = $false
        prerelease = $false
    } | ConvertTo-Json
)
Write-Host "已建立 Release：$($release.html_url)" -ForegroundColor Green

function Send-Asset {
    param([string]$Path, [string]$Name, [string]$ContentType)

    $uri = "https://uploads.github.com/repos/$Repo/releases/$($release.id)/assets?name=$([uri]::EscapeDataString($Name))"
    Invoke-RestMethod -Method Post -Uri $uri -Headers $headers -ContentType $ContentType -InFile $Path | Out-Null
    Write-Host "  已上傳 $Name" -ForegroundColor DarkGray
}

# 安裝檔先傳。latest.json 一旦就位，所有用戶端就會開始去抓它指向的安裝檔——
# 反過來的順序會有一段時間所有人的更新都是 404。
Send-Asset -Path $setup.FullName -Name $assetName -ContentType 'application/octet-stream'
Send-Asset -Path $manifest -Name 'latest.json' -ContentType 'application/json'

Write-Host ''
Write-Host '發布完成。' -ForegroundColor Green

# 這一步是整條流程真正的驗收：用**匿名**身分抓一次端點。
#
# 上面每一個 API 呼叫都帶著 token，所以就算 repo 是私有的也會全部成功——
# 而使用者電腦上的隨譯沒有 token。少了這一步，私有 repo 的錯要等到使用者
# 看到「檢查失敗」才會發現，而那時候看起來像是程式壞了。
$endpoint = "https://github.com/$Repo/releases/latest/download/latest.json"
Write-Host "以匿名身分驗證端點：$endpoint" -ForegroundColor Cyan
try {
    $anonymous = Invoke-RestMethod -Uri $endpoint -Headers @{ 'User-Agent' = 'anylingo-release-check' }
    if ($anonymous.version -ne $version) {
        Write-Host "  端點回的是 $($anonymous.version)，不是 $version。GitHub 可能還在處理，稍候再試一次。" -ForegroundColor Yellow
    }
    else {
        Write-Host "  OK：匿名抓得到 $version 的更新清單，自動更新可以運作。" -ForegroundColor Green
    }
}
catch {
    Write-Host ''
    Write-Host "  匿名抓不到（$($_.Exception.Message)）。" -ForegroundColor Red
    Write-Host @"
  Release 本身已經建好了，但外面的人抓不到——最常見的原因是 $Repo 是私有的。
  私有 repo 的 Release 資產一定要帶 token 才下載得到，而使用者電腦上的隨譯沒有 token，
  自動更新會一直失敗。

  請到 https://github.com/$Repo/settings 最下方把它改成 Public，再執行一次：
    Invoke-RestMethod '$endpoint'
"@ -ForegroundColor Yellow
}
