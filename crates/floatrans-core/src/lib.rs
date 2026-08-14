/// 同一段原文可以要兩種輸出。放在請求上而非另開一條路徑，
/// 是因為所有供應商都共用同一個提示詞組裝函式，加在這裡它們就全部支援。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TranslationMode {
    /// 純譯文：只要翻譯，不夾帶說明。
    #[default]
    Translate,
    /// 解釋：術語、縮寫、語氣等純譯文會流失的補充說明。
    Explain,
    /// 抄錄：把圖片裡的文字原樣抄出來，不翻譯也不描述。
    ///
    /// 這是拿模型當 OCR 用。抄完的文字接著走一般的翻譯路徑，所以面板上的
    /// 原文欄位、修改後重譯、解釋都照常運作——把「取字」和「翻譯」分開兩步，
    /// 而不是直接叫模型看圖說譯文。
    Transcribe,
}

/// 隨請求送出的圖片。
///
/// `base64` 是不含 `data:` 前綴的裸資料：各家供應商包裝方式不同
///（OpenAI 要 data URL、Anthropic 與 Gemini 要裸的 base64），
/// 統一存最小共同格式，由各供應商自己加工。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestImage {
    pub media_type: String,
    pub base64: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationRequest {
    pub source_text: String,
    pub target_language: String,
    pub mode: TranslationMode,
    /// 有圖片時一併送給模型。只有 `Transcribe` 會用到，
    /// 但放在請求上而不是另開型別，供應商就不必為它多一條路徑。
    pub image: Option<RequestImage>,
}

impl TranslationRequest {
    pub fn new(source_text: impl Into<String>, target_language: impl Into<String>) -> Self {
        Self {
            source_text: source_text.into(),
            target_language: target_language.into(),
            mode: TranslationMode::Translate,
            image: None,
        }
    }

    pub fn explaining(source_text: impl Into<String>, target_language: impl Into<String>) -> Self {
        Self {
            mode: TranslationMode::Explain,
            ..Self::new(source_text, target_language)
        }
    }

    /// 請模型把圖片裡的文字抄出來。不帶目標語言——抄錄不涉及翻譯。
    pub fn transcribing(image: RequestImage) -> Self {
        Self {
            mode: TranslationMode::Transcribe,
            image: Some(image),
            ..Self::new(String::new(), String::new())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TranslationEvent {
    Delta(String),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TranslationStatus {
    #[default]
    Idle,
    Streaming,
    Cancelled,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TranslationSnapshot {
    pub source_text: String,
    pub translated_text: String,
    pub status: TranslationStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranslationId(u64);

#[derive(Debug, Default)]
pub struct TranslationSession {
    next_id: u64,
    active_id: Option<TranslationId>,
    snapshot: TranslationSnapshot,
}

impl TranslationSession {
    pub fn start(&mut self, request: TranslationRequest) -> TranslationId {
        self.next_id += 1;
        let id = TranslationId(self.next_id);
        self.active_id = Some(id);
        self.snapshot = TranslationSnapshot {
            source_text: request.source_text,
            translated_text: String::new(),
            status: TranslationStatus::Streaming,
        };
        id
    }

    pub fn apply(&mut self, id: TranslationId, event: TranslationEvent) {
        if self.active_id != Some(id) {
            return;
        }

        match event {
            TranslationEvent::Delta(text) => self.snapshot.translated_text.push_str(&text),
        }
    }

    pub fn snapshot(&self) -> &TranslationSnapshot {
        &self.snapshot
    }

    pub fn cancel(&mut self, id: TranslationId) -> bool {
        if self.active_id != Some(id) || self.snapshot.status != TranslationStatus::Streaming {
            return false;
        }

        self.active_id = None;
        self.snapshot.status = TranslationStatus::Cancelled;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_translation_request_replaces_the_previous_stream() {
        let mut session = TranslationSession::default();
        let first = session.start(TranslationRequest::new("hello", "zh-TW"));
        session.apply(first, TranslationEvent::Delta("你".into()));

        let second = session.start(TranslationRequest::new("goodbye", "zh-TW"));
        session.apply(first, TranslationEvent::Delta("好".into()));
        session.apply(second, TranslationEvent::Delta("再見".into()));

        assert_eq!(session.snapshot().source_text, "goodbye");
        assert_eq!(session.snapshot().translated_text, "再見");
        assert_eq!(session.snapshot().status, TranslationStatus::Streaming);
    }

    #[test]
    fn cancelled_translation_ignores_late_stream_events() {
        let mut session = TranslationSession::default();
        let id = session.start(TranslationRequest::new("hello", "zh-TW"));
        session.apply(id, TranslationEvent::Delta("你".into()));

        assert!(session.cancel(id));
        session.apply(id, TranslationEvent::Delta("好".into()));

        assert_eq!(session.snapshot().translated_text, "你");
        assert_eq!(session.snapshot().status, TranslationStatus::Cancelled);
    }
}
