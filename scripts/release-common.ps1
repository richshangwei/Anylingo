<#
.SYNOPSIS
    發布流程共用的版本與更新說明處理。以 dot-source 方式載入。

.DESCRIPTION
    這裡集中兩件在發布時最容易出錯、而且錯了很難發現的事：

    1. 版本號散落在四個檔案裡。tauri.conf.json 決定安裝檔名與更新比對，
       Cargo.toml 決定程式裡 env!("CARGO_PKG_VERSION") 顯示的「目前版本」。
       兩者不同步時，安裝檔是新的、程式卻自稱舊版，使用者會看到自己「永遠沒更新到」。

    2. 更新說明。使用者按下更新前唯一看得到的資訊就是 latest.json 的 notes，
       它的來源是 CHANGELOG.md。沒寫就不給發。
#>

# 這個檔案是被 dot-source 進別的腳本的，所以刻意不設 Set-StrictMode：
# 嚴格模式會連同外洩到呼叫端，把呼叫端原本正確的寫法（例如對單一元素取 .Count）
# 變成執行期錯誤。函式庫不該改變呼叫者的語意。
$script:RepoRoot = Split-Path $PSScriptRoot -Parent

# 版本號的落點。改版本時必須一起動，否則就是上面說的那種不同步。
#
# 每個樣式只換「第一個」符合的位置，所以樣式要能命中該檔案裡最先出現的那個版本欄位。
# package-lock.json 有兩處自己的版本（頂層與 packages[""]），而 packages 底下每個
# 相依套件也都有 6 個空格縮排的 "version"——靠「只換第一個」把它們排除在外，
# packages[""] 在 lockfileVersion 3 裡永遠是第一個。
$script:VersionFiles = @(
    @{ Path = 'Cargo.toml'; Patterns = @('(?m)^(?<pre>version = ")(?<version>[^"]+)(?<post>")') }
    @{ Path = 'apps\desktop\src-tauri\tauri.conf.json'; Patterns = @('(?m)^(?<pre>  "version": ")(?<version>[^"]+)(?<post>")') }
    @{ Path = 'apps\desktop\package.json'; Patterns = @('(?m)^(?<pre>  "version": ")(?<version>[^"]+)(?<post>")') }
    @{
        Path     = 'apps\desktop\package-lock.json'
        Patterns = @(
            '(?m)^(?<pre>  "version": ")(?<version>[^"]+)(?<post>")'
            '(?m)^(?<pre>      "version": ")(?<version>[^"]+)(?<post>")'
        )
    }
)

function Get-VersionIn {
    param([Parameter(Mandatory)][hashtable]$File)

    $full = Join-Path $script:RepoRoot $File.Path
    if (-not (Test-Path $full)) { throw "找不到版本檔：$full" }
    $content = Get-Content $full -Raw
    $match = [regex]::Match($content, $File.Patterns[0])
    if (-not $match.Success) { throw "在 $($File.Path) 找不到版本欄位（樣式：$($File.Patterns[0])）。" }
    $match.Groups['version'].Value
}

<#
.SYNOPSIS
    讀出專案版本，順便確認四個落點一致。
#>
function Get-ProjectVersion {
    [CmdletBinding()]
    param()

    $found = @{}
    foreach ($file in $script:VersionFiles) {
        $found[$file.Path] = Get-VersionIn -File $file
    }

    # 一定要用 @() 包起來：只有一個相異值時管線回傳的是單一字串，
    # 在 StrictMode 下對字串取 .Count 會直接炸掉。
    $distinct = @($found.Values | Select-Object -Unique)
    if ($distinct.Count -ne 1) {
        $detail = ($found.GetEnumerator() | ForEach-Object { "  $($_.Value)  <- $($_.Key)" }) -join "`n"
        throw @"
版本號不一致：
$detail
安裝檔版本（tauri.conf.json）與程式自報版本（Cargo.toml）不同步時，
使用者會裝了新版卻看到舊版號，並且以為更新沒生效。
請用 scripts\set-version.ps1 一次改齊。
"@
    }

    $distinct[0]
}

<#
.SYNOPSIS
    把版本號寫進全部四個落點。
#>
function Set-ProjectVersion {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Version)

    if ($Version -notmatch '^\d+\.\d+\.\d+$') {
        throw "版本號必須是 x.y.z（收到：$Version）。Tauri 以語意化版本比較大小。"
    }

    foreach ($file in $script:VersionFiles) {
        $full = Join-Path $script:RepoRoot $file.Path
        $content = Get-Content $full -Raw
        foreach ($pattern in $file.Patterns) {
            # 一定要用 Regex 執行個體的 Replace 才有「次數」參數。
            # 靜態的 [regex]::Replace(input, pattern, replacement, 1) 第四個參數是
            # RegexOptions 不是次數，1 會被當成 IgnoreCase，然後把**每一個**符合的
            # 位置都換掉——package-lock.json 裡每個相依套件的版本都會被改成專案版本。
            $regex = [regex]::new($pattern)
            # 判斷樣式失效要看「有沒有命中」，不能看「內容有沒有變」：
            # 上一次改到一半失敗時，前幾個檔案已經是目標版本了，替換前後相同是正常的。
            if (-not $regex.IsMatch($content)) {
                throw "$($file.Path) 找不到版本欄位，樣式可能已失效：$pattern"
            }
            $content = $regex.Replace($content, "`${pre}$Version`${post}", 1)
        }
        Set-Content -Path $full -Value $content -NoNewline -Encoding UTF8
        Write-Host "  $($file.Path) -> $Version" -ForegroundColor DarkGray
    }
}

<#
.SYNOPSIS
    從 CHANGELOG.md 取出某個版本的條列更新說明。

.DESCRIPTION
    回傳的字串會原樣進到 latest.json 的 notes，也就是使用者在更新對話框裡看到的內容。
    前端會逐行顯示並去掉 "- " 前綴，所以這裡保留條列符號。
#>
function Get-ReleaseNotes {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Version)

    $path = Join-Path $script:RepoRoot 'CHANGELOG.md'
    if (-not (Test-Path $path)) { throw "找不到 CHANGELOG.md。更新說明的唯一來源就是它。" }

    $lines = Get-Content $path
    # 段落標題長這樣：## 0.2.0 — 2026-08-14
    $start = -1
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match "^##\s+$([regex]::Escape($Version))(\s|$)") { $start = $i; break }
    }
    if ($start -lt 0) {
        throw @"
CHANGELOG.md 裡沒有 $Version 的段落。

每個發出去的版本都必須有更新說明——那是使用者按下「立即更新」之前唯一看得到的東西。
請在 CHANGELOG.md 最上面加一段：

## $Version — $(Get-Date -Format 'yyyy-MM-dd')

- 這個版本改了什麼（寫給使用者看）
"@
    }

    $notes = @()
    for ($i = $start + 1; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -match '^##\s') { break }
        if ($lines[$i] -match '^\s*-\s+\S') { $notes += $lines[$i].Trim() }
    }

    if ($notes.Count -eq 0) {
        throw "CHANGELOG.md 的 $Version 段落裡沒有任何條列（以 '- ' 開頭的行）。空的更新說明等於沒有說明。"
    }

    $notes -join "`n"
}

<#
.SYNOPSIS
    安裝檔上傳到 GitHub 時要用的檔名。

.DESCRIPTION
    產品名是中文，而 GitHub 會把 Release 資產檔名裡的非 ASCII 字元換成點，
    「隨譯_0.2.0_x64-setup.exe」上傳後會變成「.._0.2.0_x64-setup.exe」，
    latest.json 裡寫的網址就對不上，更新會 404。所以上傳前先換成 ASCII 檔名。
#>
function Get-ReleaseAssetName {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Version)

    "Anylingo_${Version}_x64-setup.exe"
}
