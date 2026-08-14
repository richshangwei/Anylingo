import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

const target = document.getElementById("app")!;

/**
 * 掛載成功之後就不再整片蓋掉。
 *
 * 這個診斷畫面是為了「面板全白」而設的，那是掛載期間的問題；但掛載之後任何一個
 * 沒人接的 Promise（最常見的是被拒絕的 invoke，例如選取文字已失效）也會走到這裡，
 * 把一個好端端的面板換成錯誤訊息，而且再也回不去——只能關掉重開。那比原本要
 * 診斷的問題更糟。掛載之後改成只留紀錄。
 */
let mounted = false;

/**
 * 前端一旦在掛載時丟出例外，整個面板會變成全白視窗，畫面上沒有任何線索，
 * 也沒有可開的開發者工具（release 版停用），只能靠重建加診斷碼才查得出來。
 * 這裡把錯誤直接畫在面板上，讓下次一眼就看得到原因。
 */
function showFailure(label: string, detail: unknown) {
  const message = detail instanceof Error ? (detail.stack ?? detail.message) : String(detail);
  if (mounted) {
    console.error(`[隨譯] ${label}:`, detail);
    return;
  }
  target.innerHTML = "";
  const box = document.createElement("pre");
  box.style.cssText =
    "margin:0;padding:12px;color:#9e3328;background:#f5efe4;font:11px/1.5 ui-monospace,monospace;white-space:pre-wrap;height:100%;overflow:auto";
  box.textContent = `介面載入失敗（${label}）\n\n${message}`;
  target.appendChild(box);
}

window.addEventListener("error", (event) => showFailure("error", event.error ?? event.message));
window.addEventListener("unhandledrejection", (event) =>
  showFailure("unhandled rejection", event.reason)
);

try {
  mount(App, { target });
  mounted = true;
} catch (error) {
  showFailure("mount", error);
}
