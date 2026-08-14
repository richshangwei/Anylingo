export type TranslationPanelStatus =
  | "idle"
  | "streaming"
  | "completed"
  | "cancelled"
  | "failed";

export type TranslationPanelSnapshot = {
  sourceText: string;
  translatedText: string;
  targetLanguage: string;
  status: TranslationPanelStatus;
};

export type TranslationPanelState = {
  start(sourceText: string, targetLanguage: string): void;
  append(delta: string): void;
  complete(): void;
  cancel(): void;
  fail(): void;
  /** 回到起始畫面：丟掉這一輪的原文與譯文，狀態退回 idle。 */
  reset(): void;
  snapshot(): TranslationPanelSnapshot;
};

const initial = (targetLanguage: string): TranslationPanelSnapshot => ({
  sourceText: "",
  translatedText: "",
  targetLanguage,
  status: "idle"
});

export function createTranslationPanelState(): TranslationPanelState {
  let state: TranslationPanelSnapshot = initial("繁體中文");

  return {
    start(sourceText, targetLanguage) {
      state = {
        sourceText,
        translatedText: "",
        targetLanguage,
        status: "streaming"
      };
    },
    append(delta) {
      if (state.status !== "streaming") return;
      state = { ...state, translatedText: state.translatedText + delta };
    },
    complete() {
      if (state.status !== "streaming") return;
      state = { ...state, status: "completed" };
    },
    cancel() {
      if (state.status !== "streaming") return;
      state = { ...state, status: "cancelled" };
    },
    fail() {
      if (state.status !== "streaming") return;
      state = { ...state, status: "failed" };
    },
    reset() {
      // 目標語言是使用者選的設定，不隨內容一起清掉。
      state = initial(state.targetLanguage);
    },
    snapshot() {
      return { ...state };
    }
  };
}
