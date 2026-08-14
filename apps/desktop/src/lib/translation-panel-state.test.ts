import { describe, expect, it } from "vitest";
import { createTranslationPanelState } from "./translation-panel-state";

describe("translation panel state", () => {
  it("keeps partial translation and ignores late deltas after cancellation", () => {
    const panel = createTranslationPanelState();

    panel.start("hello", "繁體中文");
    panel.append("你");
    panel.cancel();
    panel.append("好");

    expect(panel.snapshot()).toEqual({
      sourceText: "hello",
      translatedText: "你",
      targetLanguage: "繁體中文",
      status: "cancelled"
    });
  });

  it("returns to the empty state on reset but keeps the chosen target language", () => {
    const panel = createTranslationPanelState();

    panel.start("hello", "日本語");
    panel.append("こんにちは");
    panel.complete();
    panel.reset();

    expect(panel.snapshot()).toEqual({
      sourceText: "",
      translatedText: "",
      targetLanguage: "日本語",
      status: "idle"
    });
  });
});
