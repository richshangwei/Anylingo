<#
.SYNOPSIS
    改版本號，一次改齊四個落點，並確認 CHANGELOG 有對應段落。

.DESCRIPTION
    版本號散在 Cargo.toml、tauri.conf.json、package.json、package-lock.json。
    手動改容易漏掉 Cargo.toml，結果是安裝檔叫 0.2.0、程式卻自報 0.1.0，
    使用者更新完看到版號沒動，會以為更新失敗。

    改完會檢查 CHANGELOG.md 是否已經有這個版本的條列。沒有的話只是警告不是錯誤——
    你通常會先改版號再寫說明；真正擋下來的是建置那一關。

.EXAMPLE
    .\scripts\set-version.ps1 0.2.1
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Version
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release-common.ps1')

$current = try { Get-ProjectVersion } catch { $null }
if ($current) {
    Write-Host "目前版本：$current" -ForegroundColor DarkGray
    if ([version]$Version -le [version]$current) {
        throw "新版本 $Version 沒有比目前的 $current 大。已安裝的舊版只會在版本更大時才提示更新。"
    }
}

Write-Host "改為 $Version" -ForegroundColor Cyan
Set-ProjectVersion -Version $Version

try {
    Get-ReleaseNotes -Version $Version | Out-Null
    Write-Host "CHANGELOG.md 已有 $Version 的更新說明。" -ForegroundColor Green
}
catch {
    Write-Host ''
    Write-Host "提醒：CHANGELOG.md 還沒有 $Version 的更新說明，建置時會被擋下來。" -ForegroundColor Yellow
    Write-Host '請在 CHANGELOG.md 最上面補一段：' -ForegroundColor Yellow
    Write-Host ''
    Write-Host "## $Version — $(Get-Date -Format 'yyyy-MM-dd')"
    Write-Host ''
    Write-Host '- 這個版本改了什麼'
}
