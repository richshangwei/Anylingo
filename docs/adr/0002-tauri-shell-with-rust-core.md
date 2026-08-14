# 以 Tauri 2 建立桌面殼層並將核心留在 Rust

系統採 Tauri 2 與 Svelte/TypeScript 製作設定頁、選取動作鈕及翻譯面板；Windows UI Automation、全域輸入、剪貼簿、設定、憑證與模型串流全部由 Rust 負責。相較全 Rust UI，這個邊界引入 WebView2 與少量前端技術，但能降低多視窗與浮動介面的開發成本，同時不讓前端直接持有敏感資料或作業系統權限。
