//! Interface translations. Command ids, document text and file names remain stable.
//! Untranslated labels fall back to English so coverage can grow incrementally.

mod errors;
mod ja;
use ja::TRANSLATIONS as JAPANESE;

use std::collections::HashMap;
use std::sync::LazyLock;

static JAPANESE_INDEX: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| JAPANESE.iter().copied().collect());

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Ja,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::En, Self::Ja];

    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ja => "日本語",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        match code.to_ascii_lowercase().as_str() {
            "en" => Some(Self::En),
            "ja" => Some(Self::Ja),
            _ => None,
        }
    }

    pub fn tr(self, text: &str) -> &str {
        let translations = match self {
            Self::En => return text,
            Self::Ja => &*JAPANESE_INDEX,
        };
        translations.get(text).copied().unwrap_or(text)
    }

    /// Keep the frame's language available to dialog bodies without passing application state.
    pub fn get(ctx: &egui::Context) -> Self {
        ctx.data(|data| data.get_temp::<Self>(egui::Id::new("printcraft-language"))).unwrap_or_default()
    }

    pub fn store(self, ctx: &egui::Context) {
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("printcraft-language"), self));
    }

    /// Translate known application diagnostics at the UI boundary. Unknown library or OS
    /// details remain intact, and captured filenames/field names are never translated.
    pub fn diagnostic(self, message: &str) -> String {
        if self == Self::En {
            return message.to_owned();
        }
        let translated = self.tr(message);
        if translated != message {
            return translated.to_owned();
        }
        for template in errors::TEMPLATES {
            if self.tr(template) != *template
                && let Some(values) = diagnostic_values(template, message)
            {
                return tr_template(self, template, &values);
            }
        }
        message.to_owned()
    }

    /// Translate the history prefix only on command labels, never on document text.
    /// Translate generated action labels while preserving captured filenames and field names.
    pub fn action_label(self, text: &str) -> String {
        for (prefix, template, field) in [
            ("Insert pages from ", "Insert pages from {name}", "name"),
            ("Fill in ", "Fill in {name}", "name"),
            ("Set the image of ", "Set the image of {name}", "name"),
            ("Edit script of ", "Edit script of {name}", "name"),
            ("Import ", "Import {name}", "name"),
        ] {
            if let Some(value) = text.strip_prefix(prefix) {
                return tr_template(self, template, &[(field, value)]);
            }
        }
        if let Some(key) = text.strip_prefix("Change ")
            && ["Title", "Author", "Subject", "Keywords", "Creator", "Producer"].contains(&key)
        {
            return tr_template(self, "Change {key}", &[("key", self.tr(key))]);
        }
        self.tr(text).to_owned()
    }

    pub fn command_label(self, text: &str) -> String {
        for prefix in ["Undo", "Redo"] {
            if let Some(action) = text.strip_prefix(prefix).and_then(|tail| tail.strip_prefix(' ')) {
                return format!("{} {}", self.tr(prefix), self.action_label(action));
            }
        }
        self.tr(text).to_string()
    }
}

/// Resolve before `widget_info` callbacks: those callbacks run under the context write lock.
pub fn tr<'a>(ui: &egui::Ui, text: &'a str) -> &'a str {
    Language::get(ui.ctx()).tr(text)
}

pub fn tr_fmt(ui: &egui::Ui, template: &str, args: &[(&str, &str)]) -> String {
    tr_template(Language::get(ui.ctx()), template, args)
}

/// Substitute once in the translated template. Inserted filenames, URLs and other values
/// are opaque, even when they contain text resembling another placeholder.
pub fn tr_template(language: Language, template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut chars = language.tr(template).chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next();
            out.push('{');
        } else if c == '}' && chars.peek() == Some(&'}') {
            chars.next();
            out.push('}');
        } else if c == '{' {
            let mut field = String::new();
            while chars.peek().is_some_and(|c| *c != '}') {
                if let Some(c) = chars.next() {
                    field.push(c);
                }
            }
            let closed = chars.next().is_some();
            if closed && let Some((_, value)) = args.iter().find(|(key, _)| *key == field) {
                if field == "e" {
                    out.push_str(&language.diagnostic(value));
                } else {
                    out.push_str(value);
                }
            } else {
                out.push('{');
                out.push_str(&field);
                if closed {
                    out.push('}');
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn diagnostic_values<'a>(template: &'a str, message: &'a str) -> Option<Vec<(&'a str, &'a str)>> {
    let mut values = Vec::new();
    let mut template = template;
    let mut message = message;
    while let Some((prefix, tail)) = template.split_once('{') {
        message = message.strip_prefix(prefix)?;
        let (field, tail) = tail.split_once('}')?;
        let delimiter = tail.split('{').next().unwrap_or_default();
        let (value, remainder) = if delimiter.is_empty() {
            if !tail.is_empty() {
                return None;
            }
            (message, "")
        } else {
            let end = if tail.contains('{') { message.find(delimiter)? } else { message.len().checked_sub(delimiter.len())? };
            (message.get(..end)?, message.get(end..)?)
        };
        values.push((field, value));
        template = tail;
        message = remainder;
    }
    (template == message).then_some(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_preserve_opaque_values_reordering_and_literal_braces() {
        let args = [("name", "報告 {e}.pdf"), ("e", "the password is incorrect")];
        assert_eq!(tr_template(Language::Ja, "Couldn't open {name}: {e}", &args), "報告 {e}.pdf を開けませんでした: パスワードが正しくありません");
        assert_eq!(tr_template(Language::En, "Couldn't open {name}: {e}", &args), "Couldn't open 報告 {e}.pdf: the password is incorrect");
        assert_eq!(tr_template(Language::En, "{e}: {name}", &args), "the password is incorrect: 報告 {e}.pdf");
        assert_eq!(tr_template(Language::En, "{{name}} {name} {unknown} {", &args), "{name} 報告 {e}.pdf {unknown} {");
    }

    #[test]
    fn diagnostics_preserve_field_names_unknown_details_and_english() {
        let input = "there is no field named \"File {e}\"";
        assert_eq!(Language::Ja.diagnostic(input), "\"File {e}\" という名前のフィールドはありません");
        assert_eq!(Language::En.diagnostic(input), input);
        assert_eq!(Language::Ja.diagnostic("page 17 does not exist"), "ページ 17 はありません");
        assert_eq!(Language::Ja.diagnostic("page 17 does not exists"), "page 17 does not exists");
        assert_eq!(Language::Ja.diagnostic("unknown OS diagnostic: /tmp/File.pdf"), "unknown OS diagnostic: /tmp/File.pdf");
        assert_eq!(Language::Ja.diagnostic("there is no comment 4 on page 2"), "ページ 2 に注釈 4 はありません");
    }

    #[test]
    fn japanese_templates_keep_the_same_placeholders() {
        fn fields(text: &str) -> Vec<&str> {
            let mut found = Vec::new();
            let mut tail = text;
            while let Some((_, rest)) = tail.split_once('{') {
                let Some((key, rest)) = rest.split_once('}') else { break };
                found.push(key);
                tail = rest;
            }
            found.sort_unstable();
            found
        }
        for (english, translated) in JAPANESE {
            assert_eq!(fields(english), fields(translated), "template changed its arguments: {english}");
        }
        for template in errors::TEMPLATES {
            assert_ne!(Language::Ja.tr(template), *template, "missing diagnostic: {template}");
        }
    }

    #[test]
    fn history_labels_preserve_opaque_document_names() {
        assert_eq!(Language::Ja.action_label("Insert pages from Save {e}.pdf"), "Save {e}.pdf からページを挿入");
        assert_eq!(Language::Ja.command_label("Undo Insert pages from Save {e}.pdf"), "取り消し Save {e}.pdf からページを挿入");
        assert_eq!(Language::Ja.action_label("Untranslated custom action"), "Untranslated custom action");
        assert_eq!(Language::En.command_label("Undo Fill in Save {e}"), "Undo Fill in Save {e}");
    }

    #[test]
    fn japanese_labels_have_glyphs_with_the_web_font_subset() {
        let craft = printcraft_fonts::ui_japanese_fonts();
        let Some(face) = craft.iter().find(|f| f.family == "BIZ UDPGothic" && f.style == "Regular") else {
            eprintln!("skipping web glyph checks: build with CRAFT_FONTS_DIR");
            return;
        };
        let keep = face.name();
        let mut definitions = crate::theme::font_definitions();
        for family in definitions.families.values_mut() {
            family.retain(|name| !craft.iter().any(|face| face.name() == *name) || *name == keep);
        }
        use egui::epaint::text::{Fonts, TextOptions};
        let mut fonts = Fonts::new(TextOptions::default(), definitions);
        let labels: String = JAPANESE.iter().flat_map(|(_, text)| text.chars()).filter(|c| !c.is_control()).collect();
        for id in [egui::FontId::proportional(13.0), egui::FontId::monospace(13.0), crate::theme::medium(13.0), crate::theme::semibold(17.0)] {
            assert!(fonts.has_glyphs(&id, &labels), "{id:?} lacks a Japanese glyph with the web font subset");
        }
    }

    #[test]
    fn translations_are_unique_and_preserve_unknown_text() {
        for translations in [JAPANESE] {
            for (i, (en, translated)) in translations.iter().enumerate() {
                assert!(!translated.is_empty());
                assert!(translations.iter().take(i).all(|(other, _)| en != other), "duplicate: {en}");
                assert_eq!(Language::En.tr(en), *en);
            }
        }
        assert_eq!(Language::Ja.tr("File"), "ファイル");
        assert_eq!(Language::Ja.tr("日本語の文書.pdf"), "日本語の文書.pdf");
        assert_eq!(Language::Ja.tr("File"), "ファイル");
        assert_eq!(Language::Ja.tr("年度報告.pdf"), "年度報告.pdf");
        assert_eq!(Language::Ja.tr("Untranslated label"), "Untranslated label");
        assert_eq!(Language::Ja.command_label("Undo Rotate pages"), "取り消し ページを回転");
        assert_eq!(Language::En.command_label("Undo Rotate pages"), "Undo Rotate pages");
        assert_eq!(Language::parse("xx"), None);
    }

    #[test]
    fn japanese_covers_commands_and_catalogue() {
        let contains = |label: &str| JAPANESE.iter().any(|(en, _)| *en == label);
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
    fn japanese_labels_have_glyphs_in_desktop_builds() {
        if printcraft_fonts::ui_japanese_fonts().is_empty() {
            eprintln!("skipping glyph checks: build with CRAFT_FONTS_DIR to run them");
            return;
        }
        use egui::epaint::text::{Fonts, TextOptions};
        let mut fonts = Fonts::new(TextOptions::default(), crate::theme::font_definitions());
        let labels = JAPANESE.iter().map(|(_, translated)| *translated).chain([Language::Ja.name()]).collect::<String>();
        let labels: String = labels.chars().filter(|c| !c.is_control()).collect();
        for id in [egui::FontId::proportional(13.0), egui::FontId::monospace(13.0), crate::theme::medium(13.0), crate::theme::semibold(17.0)] {
            assert!(fonts.has_glyphs(&id, &labels), "{id:?} lacks a Japanese label glyph");
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
