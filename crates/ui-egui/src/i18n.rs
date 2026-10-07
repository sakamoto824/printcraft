//! Interface translations. Command ids, document text and file names remain stable.
//! Untranslated labels fall back to English so coverage can grow incrementally.

mod zh_tw;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Ja,
    #[serde(rename = "zh-tw", alias = "zh-TW", alias = "zh-Hant", alias = "zh-hant")]
    ZhTw,
}

impl Language {
    pub const ALL: [Self; 3] = [Self::En, Self::Ja, Self::ZhTw];

    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ja => "日本語",
            Self::ZhTw => "繁體中文（台灣）",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        match code.to_ascii_lowercase().as_str() {
            "en" => Some(Self::En),
            "ja" => Some(Self::Ja),
            "zh-tw" | "zh-hant" => Some(Self::ZhTw),
            _ => None,
        }
    }

    pub fn tr(self, text: &str) -> &str {
        let translations = match self {
            Self::En => return text,
            Self::Ja => JAPANESE,
            Self::ZhTw => zh_tw::TRANSLATIONS,
        };
        translations.iter().find(|(english, _)| *english == text).map_or(text, |(_, translated)| *translated)
    }

    /// Translate the history prefix only on command labels, never on document text.
    pub fn command_label(self, text: &str) -> String {
        for prefix in ["Undo", "Redo"] {
            if let Some(action) = text.strip_prefix(prefix).and_then(|tail| tail.strip_prefix(' ')) {
                return format!("{} {}", self.tr(prefix), self.tr(action));
            }
        }
        self.tr(text).to_string()
    }
}

const JAPANESE: &[(&str, &str)] = &[
    ("Menu", "メニュー"),
    ("File", "ファイル"),
    ("Edit", "編集"),
    ("Pages", "ページ"),
    ("View", "表示"),
    ("Help", "ヘルプ"),
    ("Preferences", "環境設定"),
    ("Preferences…", "環境設定…"),
    ("Interface language", "表示言語"),
    ("Identity", "個人情報"),
    ("Name on new comments", "新しい注釈の作成者名"),
    ("Open…", "開く…"),
    ("New blank PDF", "空白の PDF を作成"),
    ("Create PDF from file…", "ファイルから PDF を作成…"),
    ("Create PDF from images…", "画像から PDF を作成…"),
    ("Create PDF from clipboard", "クリップボードから PDF を作成"),
    ("Combine files…", "ファイルを結合…"),
    ("Save", "保存"),
    ("Save as…", "別名で保存…"),
    ("Close file", "ファイルを閉じる"),
    ("Close all", "すべて閉じる"),
    ("Revert", "保存済みの状態に戻す"),
    ("Print…", "印刷…"),
    ("Document properties…", "文書のプロパティ…"),
    ("Undo", "取り消し"),
    ("Redo", "やり直し"),
    ("Find…", "検索…"),
    ("Advanced search…", "高度な検索…"),
    ("Copy pages", "ページをコピー"),
    ("Cut pages", "ページを切り取り"),
    ("Paste pages", "ページを貼り付け"),
    ("Fit visible", "表示範囲に合わせる"),
    ("Marquee zoom", "範囲指定ズーム"),
    ("Take a snapshot", "スナップショットを作成"),
    ("Full screen mode", "全画面表示"),
    ("Read mode", "閲覧モード"),
    ("Switch light / dark theme", "明るい／暗いテーマを切り替え"),
    ("Comments panel", "コメントパネル"),
    ("Form fields panel", "フォームフィールドパネル"),
    ("Clear form", "フォームをクリア"),
    ("Find tools and commands…", "ツールとコマンドを検索…"),
    ("Zoom", "ズーム"),
    ("Actual size", "実際のサイズ"),
    ("Zoom to page level", "ページ全体を表示"),
    ("Fit to width", "幅に合わせる"),
    ("Display theme", "表示テーマ"),
    ("Side panels", "サイドパネル"),
    ("Enable Acrobat JavaScript", "Acrobat JavaScript を有効にする"),
    ("OK", "OK"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translations_are_unique_and_preserve_unknown_text() {
        for translations in [JAPANESE, zh_tw::TRANSLATIONS] {
            for (i, (en, translated)) in translations.iter().enumerate() {
                assert!(!translated.is_empty());
                assert!(translations.iter().take(i).all(|(other, _)| en != other), "duplicate: {en}");
                assert_eq!(Language::En.tr(en), *en);
            }
        }
        assert_eq!(Language::Ja.tr("File"), "ファイル");
        assert_eq!(Language::Ja.tr("日本語の文書.pdf"), "日本語の文書.pdf");
        assert_eq!(Language::ZhTw.tr("File"), "檔案");
        assert_eq!(Language::ZhTw.tr("年度報告.pdf"), "年度報告.pdf");
        assert_eq!(Language::ZhTw.tr("Untranslated label"), "Untranslated label");
        assert_eq!(Language::ZhTw.command_label("Undo Rotate pages"), "復原 旋轉頁面");
        assert_eq!(Language::En.command_label("Undo Rotate pages"), "Undo Rotate pages");
        assert_eq!(Language::parse("xx"), None);
    }

    #[test]
    fn traditional_chinese_covers_commands_and_catalogue() {
        let contains = |label: &str| zh_tw::TRANSLATIONS.iter().any(|(en, _)| *en == label);
        for command in printcraft_engine::commands::COMMANDS {
            assert!(contains(command.label), "missing command: {}", command.label);
        }
        for group in printcraft_engine::catalog::TOOL_GROUPS {
            assert!(contains(group.label), "missing group: {}", group.label);
            for section in group.sections {
                assert!(contains(section.title), "missing section: {}", section.title);
                for item in section.items {
                    assert!(contains(item.label), "missing item: {}", item.label);
                }
            }
        }
    }

    #[test]
    fn traditional_chinese_labels_have_glyphs_in_desktop_builds() {
        if printcraft_fonts::ui_japanese_fonts().is_empty() {
            eprintln!("skipping glyph checks: build with CRAFT_FONTS_DIR to run them");
            return;
        }
        use egui::epaint::text::{Fonts, TextOptions};
        let mut fonts = Fonts::new(TextOptions::default(), crate::theme::font_definitions());
        let labels = zh_tw::TRANSLATIONS.iter().map(|(_, translated)| *translated).chain([Language::ZhTw.name()]).collect::<String>();
        for id in [egui::FontId::proportional(13.0), egui::FontId::monospace(13.0), crate::theme::medium(13.0), crate::theme::semibold(17.0)] {
            assert!(fonts.has_glyphs(&id, &labels), "{id:?} lacks a Traditional Chinese label glyph");
        }
    }

    #[test]
    fn traditional_chinese_codes_and_settings_round_trip() {
        for code in ["zh-tw", "zh-TW", "zh-hant", "zh-Hant"] {
            assert_eq!(Language::parse(code), Some(Language::ZhTw));
            assert_eq!(serde_json::from_value::<Language>(serde_json::json!(code)).unwrap(), Language::ZhTw);
            let mut app = crate::PrintCraftApp::default();
            app.set_option("language", code).unwrap();
            let saved = app.persist();
            assert_eq!(serde_json::from_str::<serde_json::Value>(&saved).unwrap()["language"], "zh-tw");
            let mut restored = crate::PrintCraftApp::default();
            restored.restore(&saved);
            assert_eq!(restored.language, Language::ZhTw);
            assert!(restored.set_option("language", "zh").is_err());
            restored.restore(r#"{"language":"xx"}"#);
            assert_eq!(restored.language, Language::ZhTw);
            restored.set_option("language", "en").unwrap();
            assert_eq!(restored.language, Language::En);
        }
    }

    #[test]
    fn language_persists_and_invalid_input_keeps_current_language() {
        let mut app = crate::PrintCraftApp::default();
        app.set_option("language", "ja").unwrap();
        assert_eq!(app.language, Language::Ja);
        assert!(app.set_option("language", "xx").is_err());
        assert_eq!(app.language, Language::Ja);
        let mut restored = crate::PrintCraftApp::default();
        restored.restore(&app.persist());
        assert_eq!(restored.language, Language::Ja);
        restored.restore(r#"{"language":"xx"}"#);
        assert_eq!(restored.language, Language::Ja);
        let mut legacy = crate::PrintCraftApp::default();
        legacy.restore("{}");
        assert_eq!(legacy.language, Language::En);
    }
}
