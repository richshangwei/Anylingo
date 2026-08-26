import { describe, expect, it } from "vitest";
import { DEFAULT_LOCALE_ID, LOCALES, resolveLocale, stringsFor } from "./index";
import { zhTW } from "./zh-TW";

/**
 * 這幾個測試在補「建置不會幫忙檢查」的洞。
 *
 * `npm run build` 只跑 vite，沒有 `tsc`——所以 `satisfies Locale` 漏掉一個鍵、
 * 或某個語言把帶變數的字串寫成了普通字串，打包一樣會過，錯誤要等到使用者切到
 * 那個語言、走到那個畫面才會冒出來（而且多半是「畫面上出現 undefined」這種
 * 最難回報的樣子）。這裡把它擋在 `npm test`。
 */

/** 把巢狀物件壓成 `a.b.c` 形式的鍵清單，順便記下每個葉節點是字串還是函式。 */
function flatten(value: unknown, prefix = ""): Map<string, "string" | "function"> {
  const found = new Map<string, "string" | "function">();
  if (typeof value === "function") {
    found.set(prefix, "function");
    return found;
  }
  if (typeof value === "string") {
    found.set(prefix, "string");
    return found;
  }
  if (value && typeof value === "object") {
    for (const [key, child] of Object.entries(value)) {
      const path = prefix ? `${prefix}.${key}` : key;
      for (const [innerPath, kind] of flatten(child, path)) found.set(innerPath, kind);
    }
  }
  return found;
}

const reference = flatten(zhTW.strings);

describe("介面語言檔", () => {
  it("每個語言都有一模一樣的鍵，型別也一致", () => {
    for (const locale of LOCALES) {
      const actual = flatten(locale.strings);

      const missing = [...reference.keys()].filter((key) => !actual.has(key));
      const extra = [...actual.keys()].filter((key) => !reference.has(key));
      expect({ locale: locale.id, missing, extra }).toEqual({
        locale: locale.id,
        missing: [],
        extra: []
      });

      // 字串與函式不能互換：把 `upToDate(version)` 寫成固定字串，畫面上會少掉版號；
      // 反過來把純字串寫成函式，畫面上會直接印出函式原始碼。
      const wrongKind = [...reference].filter(([key, kind]) => actual.get(key) !== kind);
      expect({ locale: locale.id, wrongKind }).toEqual({ locale: locale.id, wrongKind: [] });
    }
  });

  it("帶變數的字串在每個語言都收一樣多的變數", () => {
    const functionKeys = [...reference].filter(([, kind]) => kind === "function").map(([key]) => key);
    // 這份表本身要有東西，否則上面那條斷言會在沒有任何函式時空轉而看起來是綠的。
    expect(functionKeys.length).toBeGreaterThan(0);

    function at(strings: unknown, path: string): unknown {
      return path.split(".").reduce<unknown>((node, key) => (node as Record<string, unknown>)[key], strings);
    }

    for (const locale of LOCALES) {
      for (const key of functionKeys) {
        const expected = (at(zhTW.strings, key) as (...args: never[]) => string).length;
        const actual = (at(locale.strings, key) as (...args: never[]) => string).length;
        expect({ locale: locale.id, key, arity: actual }).toEqual({
          locale: locale.id,
          key,
          arity: expected
        });
      }
    }
  });

  it("沒有空字串——漏翻會變成畫面上的空白，比留著原文更難發現", () => {
    for (const locale of LOCALES) {
      const blanks = [...flatten(locale.strings)]
        .filter(([, kind]) => kind === "string")
        .map(([key]) => key)
        .filter((key) => {
          const value = key
            .split(".")
            .reduce<unknown>((node, part) => (node as Record<string, unknown>)[part], locale.strings);
          return typeof value === "string" && value.trim() === "";
        });
      expect({ locale: locale.id, blanks }).toEqual({ locale: locale.id, blanks: [] });
    }
  });

  it("語言 id 與母語名稱都不重複", () => {
    const ids = LOCALES.map((locale) => locale.id);
    const names = LOCALES.map((locale) => locale.nativeName);
    expect(new Set(ids).size).toBe(ids.length);
    expect(new Set(names).size).toBe(names.length);
  });

  it("預設語言真的在登錄表裡", () => {
    expect(LOCALES.some((locale) => locale.id === DEFAULT_LOCALE_ID)).toBe(true);
  });

  it("認不得的語言退回預設，不丟例外", () => {
    // 資料庫裡可能留著舊版寫入的值、或某個被移除的語言檔。
    expect(resolveLocale("kl-GL").id).toBe(DEFAULT_LOCALE_ID);
    expect(resolveLocale("").id).toBe(DEFAULT_LOCALE_ID);
    expect(resolveLocale(null).id).toBe(DEFAULT_LOCALE_ID);
    expect(resolveLocale(undefined).id).toBe(DEFAULT_LOCALE_ID);
  });

  it("帶地區的標籤落到主語言", () => {
    expect(resolveLocale("en-US").id).toBe("en");
    expect(resolveLocale("ja-JP").id).toBe("ja");
    // zh 開頭的沒有裸 `zh`，取第一個 zh-* ——總比退回一個完全不相干的語言好。
    expect(resolveLocale("zh-Hant").id.startsWith("zh")).toBe(true);
  });

  it("stringsFor 直接給得出可用的字串", () => {
    expect(stringsFor("en").common.translate).toBe("Translate");
    expect(stringsFor("ja").common.translate).toBe("翻訳");
    expect(stringsFor("zh-CN").common.translate).toBe("翻译");
    expect(stringsFor("zh-TW").common.translate).toBe("翻譯");
  });
});
