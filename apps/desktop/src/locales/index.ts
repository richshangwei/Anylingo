import type { Locale, LocaleId, Strings } from "./types";
import { zhTW } from "./zh-TW";
import { zhCN } from "./zh-CN";
import { en } from "./en";
import { ja } from "./ja";

export type { Locale, LocaleId, Strings };

/**
 * 介面語言的登錄表。**加語言只要在這裡加一行**，語言選單、驗證、回退全部跟著長出來，
 * 不必再去翻哪裡還漏了一處。
 *
 * 順序就是設定裡語言選單的顯示順序，刻意不做排序：字母排序在多文字系統下沒有意義
 * （中文按什麼排？日文按五十音還是 Unicode？），不如維持一份人為決定的固定順序。
 */
export const LOCALES: readonly Locale[] = [zhTW, en, ja, zhCN];

/**
 * 預設語言。這是產品的母語版本，也是 `types.ts` 的基準。
 *
 * 同時是 Rust 那側 `DEFAULT_UI_LOCALE` 的值——兩邊對不上的話，第一次啟動時
 * 前端會顯示一種語言、系統匣顯示另一種。
 */
export const DEFAULT_LOCALE_ID: LocaleId = "zh-TW";

const BY_ID = new Map<string, Locale>(LOCALES.map((locale) => [locale.id, locale]));

/**
 * 把任意字串收斂成一個確定存在的語言。
 *
 * 認不得就回預設，不丟例外：這個值來自資料庫，而資料庫可能是舊版寫的、可能被
 * 手動改過、也可能是某個語言檔被移除後留下的孤兒。介面語言讀不出來時該退回預設
 * 繼續跑，不該讓整個面板開不起來。
 */
export function resolveLocale(id: string | null | undefined): Locale {
  if (!id) return BY_ID.get(DEFAULT_LOCALE_ID)!;
  const exact = BY_ID.get(id);
  if (exact) return exact;
  // `zh-Hant-TW`、`en-US` 這類帶地區的標籤取主語言再試一次，讓 `en-GB` 也能落在 `en`。
  const base = id.split("-")[0];
  const byBase = LOCALES.find((locale) => locale.id === base || locale.id.startsWith(`${base}-`));
  return byBase ?? BY_ID.get(DEFAULT_LOCALE_ID)!;
}

/** 取字串表。畫面上一律走這裡，不要直接 import 某一個語言檔。 */
export function stringsFor(id: string | null | undefined): Strings {
  return resolveLocale(id).strings;
}
