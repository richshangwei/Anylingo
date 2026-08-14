// Node 24.0–24.13 會讓 vite build 在 "rendering chunks" 階段直接崩潰，
// 結束碼 0xC0000409（STATUS_STACK_BUFFER_OVERRUN），而且不印任何錯誤訊息。
// 症狀看起來就像「編譯不出來」，很難查。這裡先擋下來並講清楚原因。
const [major, minor] = process.versions.node.split(".").map(Number);
const supported = major >= 22 && !(major === 24 && minor < 14);

if (!supported) {
  console.error(`
✖ Node ${process.versions.node} 無法建置本專案。

  需求：>=22 <24 或 >=24.14（見 package.json 的 engines）
  ${major === 24 && minor < 14
      ? "Node 24.0–24.13 會讓 vite build 無訊息崩潰（結束碼 0xC0000409），不是程式碼有問題。"
      : "版本過舊。"}

  解法（擇一）：
    nvm use 24.14.1
    nvm install 24.14.1 && nvm use 24.14.1
`);
  process.exit(1);
}
