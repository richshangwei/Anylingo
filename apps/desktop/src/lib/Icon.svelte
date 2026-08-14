<script lang="ts">
  /**
   * 標題列的操作圖示。
   *
   * 原本用 ▣ ⚙ ◇ — × 這些字元充當圖示，有兩個問題：字形交給字型決定，
   * 換一台機器就可能變形或掉成豆腐字；而且語意得用猜的，◇ 看不出是釘選、
   * ▣ 看不出是框選截圖。改成線條圖示後形狀固定，也能畫出「框線＋文字行」
   * 這種一眼認得出用途的組合。
   *
   * 統一 24×24 座標系與 2 的線寬，縮到 14～16px 仍然清楚。
   */
  type IconName =
    | "home"
    | "capture"
    | "settings"
    | "pin"
    | "collapse"
    | "expand"
    | "close"
    | "fullscreen"
    | "restore";

  export let name: IconName;
  export let size = 16;
  /** 釘選這種開關狀態用實心／空心區分。只靠顏色的話，小尺寸下幾乎看不出差別。 */
  export let filled = false;
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#if name === "home"}
    <!-- 屋頂＋門＝回到起始畫面。這是最沒有歧義的一個，換成箭頭就會和「返回」混淆 -->
    <path d="M3 10.5L12 3l9 7.5" />
    <path d="M5 9.8V20a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V9.8" />
    <path d="M10 21v-6h4v6" />
  {:else if name === "capture"}
    <!-- 四角框線＋內部文字行＝框起畫面、辨識裡面的字 -->
    <path d="M3 7V5a2 2 0 0 1 2-2h2" />
    <path d="M17 3h2a2 2 0 0 1 2 2v2" />
    <path d="M21 17v2a2 2 0 0 1-2 2h-2" />
    <path d="M7 21H5a2 2 0 0 1-2-2v-2" />
    <path d="M7 9h10" />
    <path d="M7 13h10" />
    <path d="M7 17h6" />
  {:else if name === "settings"}
    <!-- 滑桿。齒輪在 16px 下會糊成一團，滑桿的輪廓清楚得多 -->
    <path d="M20 7h-9" />
    <path d="M14 17H5" />
    <circle cx="17" cy="17" r="3" />
    <circle cx="7" cy="7" r="3" />
  {:else if name === "pin"}
    <path
      d="M12 17v5"
      fill="none"
    />
    <path
      d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"
      fill={filled ? "currentColor" : "none"}
    />
  {:else if name === "collapse"}
    <!-- 四角向內收＝收合到角落 -->
    <path d="M4 14h6v6" />
    <path d="M20 10h-6V4" />
    <path d="M14 10l7-7" />
    <path d="M3 21l7-7" />
  {:else if name === "expand"}
    <path d="M15 3h6v6" />
    <path d="M9 21H3v-6" />
    <path d="M21 3l-7 7" />
    <path d="M3 21l7-7" />
  {:else if name === "close"}
    <path d="M18 6L6 18" />
    <path d="M6 6l12 12" />
  {:else if name === "fullscreen"}
    <!-- 四角向外撐開＝放大至全螢幕。和 expand 的差別在於這裡是「四個角」，
         expand 是對角兩箭頭，並排時分得出來 -->
    <path d="M8 3H5a2 2 0 0 0-2 2v3" />
    <path d="M21 8V5a2 2 0 0 0-2-2h-3" />
    <path d="M3 16v3a2 2 0 0 0 2 2h3" />
    <path d="M16 21h3a2 2 0 0 0 2-2v-3" />
  {:else if name === "restore"}
    <path d="M8 3v3a2 2 0 0 1-2 2H3" />
    <path d="M21 8h-3a2 2 0 0 1-2-2V3" />
    <path d="M3 16h3a2 2 0 0 1 2 2v3" />
    <path d="M16 21v-3a2 2 0 0 1 2-2h3" />
  {/if}
</svg>

<style>
  /* 圖示不吃滑鼠事件，按鈕才會一直是 event.target——hover 樣式與
     data-tauri-drag-region 的判斷都靠這點。 */
  .icon {
    display: block;
    pointer-events: none;
  }
</style>
