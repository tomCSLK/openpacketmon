//! Translations of the user interface: the supported languages and the Fluent catalogs in
//! `locales/<code>/*.ftl` (embedded into the binary).
//!
//! German is the source language; every catalog must contain the same messages with the same
//! variables (checked by the tests). Templates get the language as a field `t` and call
//! `t.get("nav-overview")`. Messages whose id starts with `js-` are handed to the browser scripts.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "locales/"]
struct Catalogs;

/// A language of the user interface
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Lang {
    De,
    #[default]
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::De, Lang::En];

    /// ISO 639-1 code, as stored in the settings and used in `<html lang>`
    pub fn code(self) -> &'static str {
        match self {
            Self::De => "de",
            Self::En => "en",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.code() == code)
    }

    /// The language's own name, shown in the language selection
    pub fn native_name(self) -> &'static str {
        match self {
            Self::De => "Deutsch",
            Self::En => "English",
        }
    }

    /// The preferred supported language of an `Accept-Language` header; English if none matches
    pub fn from_accept_language(header: &str) -> Self {
        let mut best: Option<(f32, Lang)> = None;
        for part in header.split(',') {
            let mut fields = part.split(';');
            let tag = fields.next().unwrap_or_default().trim();
            let primary = tag
                .split('-')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let Some(lang) = Self::parse(&primary) else {
                continue;
            };
            let quality = fields
                .find_map(|f| f.trim().strip_prefix("q="))
                .map_or(Some(1.0), |q| q.trim().parse::<f32>().ok())
                .unwrap_or(0.0);
            if quality > 0.0 && best.is_none_or(|(q, _)| quality > q) {
                best = Some((quality, lang));
            }
        }
        best.map_or_else(Self::default, |(_, lang)| lang)
    }

    /// The text of a message without variables; unknown ids come back unchanged
    pub fn get(self, id: &str) -> String {
        self.fmt(id, &[])
    }

    /// A message with one variable, e.g. `t.get1("entries-count", "count", n)` in a template
    pub fn get1(self, id: &str, name: &str, value: impl Into<Arg>) -> String {
        self.fmt(id, &[(name, value.into())])
    }

    /// A message with two variables
    pub fn get2(
        self,
        id: &str,
        name1: &str,
        value1: impl Into<Arg>,
        name2: &str,
        value2: impl Into<Arg>,
    ) -> String {
        self.fmt(id, &[(name1, value1.into()), (name2, value2.into())])
    }

    /// A message with any number of variables
    pub fn fmt(self, id: &str, args: &[(&str, Arg)]) -> String {
        let bundle = bundle(self);
        let Some(pattern) = bundle.get_message(id).and_then(|m| m.value()) else {
            return id.to_owned();
        };
        let mut fluent_args = FluentArgs::new();
        for (name, value) in args {
            fluent_args.set(*name, value.to_fluent());
        }
        let mut errors = Vec::new();
        bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors)
            .into_owned()
    }

    /// Texts of the browser scripts (`js-*` messages, all without variables) as a JSON object for
    /// the page's data block `#i18n`, keyed without the prefix. `<` is escaped so the block cannot
    /// end early.
    pub fn js_texts(self) -> &'static str {
        static TEXTS: LazyLock<Vec<String>> = LazyLock::new(|| Lang::ALL.map(js_catalog).into());
        let index = Lang::ALL.iter().position(|l| *l == self).unwrap_or(0);
        &TEXTS[index]
    }

    /// Decimal separator of numbers written by the program
    pub fn decimal_separator(self) -> char {
        match self {
            Self::De => ',',
            Self::En => '.',
        }
    }

    /// Thousands separator of counts
    pub fn thousands_separator(self) -> char {
        match self {
            Self::De => '.',
            Self::En => ',',
        }
    }
}

/// A value for a message variable
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Text(String),
    Number(f64),
}

impl Arg {
    fn to_fluent(&self) -> FluentValue<'_> {
        match self {
            Self::Text(text) => FluentValue::from(text.as_str()),
            Self::Number(n) => FluentValue::from(*n),
        }
    }
}

impl From<&str> for Arg {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl From<String> for Arg {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&String> for Arg {
    fn from(text: &String) -> Self {
        Self::Text(text.clone())
    }
}

macro_rules! number_arg {
    ($($t:ty)*) => {$(
        impl From<$t> for Arg {
            #[allow(clippy::cast_precision_loss, clippy::cast_lossless)]
            fn from(n: $t) -> Self {
                Self::Number(n as f64)
            }
        }
        impl From<&$t> for Arg {
            #[allow(clippy::cast_precision_loss, clippy::cast_lossless)]
            fn from(n: &$t) -> Self {
                Self::Number(*n as f64)
            }
        }
    )*};
}
number_arg!(i32 i64 u16 u32 u64 u128 usize f64);

type Bundle = FluentBundle<FluentResource>;

/// All catalogs, one bundle per language in the order of [`Lang::ALL`]
static BUNDLES: LazyLock<Vec<Bundle>> = LazyLock::new(|| Lang::ALL.map(load).into());

fn bundle(lang: Lang) -> &'static Bundle {
    let index = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    &BUNDLES[index]
}

/// The catalog files of a language, by file name
fn sources(lang: Lang) -> BTreeMap<String, String> {
    let prefix = format!("{}/", lang.code());
    Catalogs::iter()
        .filter_map(|path| {
            let name = path.strip_prefix(&prefix)?.to_owned();
            let file = Catalogs::get(&path)?;
            Some((name, String::from_utf8_lossy(&file.data).into_owned()))
        })
        .collect()
}

/// Builds the bundle of a language. Broken entries are skipped (the tests reject them),
/// so they never stop the program.
fn load(lang: Lang) -> Bundle {
    let mut bundle = FluentBundle::new_concurrent(vec![lang.code().parse().unwrap_or_default()]);
    bundle.set_use_isolating(false);
    if lang == Lang::De {
        bundle.set_formatter(Some(decimal_comma));
    }
    for source in sources(lang).into_values() {
        let resource =
            FluentResource::try_new(source).unwrap_or_else(|(resource, _errors)| resource);
        let _ = bundle.add_resource(resource);
    }
    bundle
}

/// Ids of the `js-*` messages of a language
fn js_ids(lang: Lang) -> Vec<String> {
    sources(lang)
        .values()
        .flat_map(|source| source.lines())
        .filter_map(|line| line.split_once(" =").map(|(id, _)| id))
        .filter(|id| id.starts_with("js-"))
        .map(str::to_owned)
        .collect()
}

fn js_catalog(lang: Lang) -> String {
    let texts: BTreeMap<String, String> = js_ids(lang)
        .into_iter()
        .map(|id| (id["js-".len()..].to_owned(), lang.get(&id)))
        .collect();
    serde_json::to_string(&texts)
        .unwrap_or_else(|_| "{}".to_owned())
        .replace('<', "\\u003c")
}

/// Numbers in German texts use a decimal comma
fn decimal_comma<M>(value: &FluentValue<'_>, _: &M) -> Option<String> {
    match value {
        FluentValue::Number(n) => Some(n.as_string().replace('.', ",")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// Message ids and their variables, from a catalog source (one message per unindented `id =` line)
    fn messages(source: &str) -> BTreeMap<String, BTreeSet<String>> {
        let mut result: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut current: Option<String> = None;
        for line in source.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if !line.starts_with(' ')
                && let Some((id, _)) = line.split_once('=')
            {
                let id = id.trim().to_owned();
                assert!(!result.contains_key(&id), "message {id} is defined twice");
                result.insert(id.clone(), BTreeSet::new());
                current = Some(id);
            }
            let Some(id) = &current else { continue };
            let mut rest = line;
            while let Some(pos) = rest.find('$') {
                rest = &rest[pos + 1..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .collect();
                if let Some(vars) = result.get_mut(id) {
                    vars.insert(name);
                }
            }
        }
        result
    }

    fn all_messages(lang: Lang) -> BTreeMap<String, BTreeSet<String>> {
        let mut all = BTreeMap::new();
        for (file, source) in sources(lang) {
            for (id, vars) in messages(&source) {
                assert!(
                    all.insert(id.clone(), vars).is_none(),
                    "{}: message {id} is defined in more than one file ({file})",
                    lang.code()
                );
            }
        }
        all
    }

    #[test]
    fn catalogs_parse_without_errors() {
        for lang in Lang::ALL {
            let files = sources(lang);
            assert!(!files.is_empty(), "no catalog for {}", lang.code());
            for (name, source) in files {
                if let Err((_, errors)) = FluentResource::try_new(source) {
                    panic!("{}/{name}: {errors:?}", lang.code());
                }
            }
        }
    }

    #[test]
    fn every_language_has_the_same_files_messages_and_variables() {
        let files = |lang| sources(lang).into_keys().collect::<Vec<_>>();
        let reference = all_messages(Lang::De);
        for lang in Lang::ALL {
            assert_eq!(files(lang), files(Lang::De), "files of {}", lang.code());
            let other = all_messages(lang);
            for (id, vars) in &reference {
                match other.get(id) {
                    None => panic!("{}: message {id} is missing", lang.code()),
                    Some(v) => assert_eq!(v, vars, "{}: variables of {id}", lang.code()),
                }
            }
            for id in other.keys() {
                assert!(
                    reference.contains_key(id),
                    "{}: message {id} is not in the German catalog",
                    lang.code()
                );
            }
        }
    }

    /// Every id used in a template or in the web code (`t.get("…")`, `t.get1("…"` …) exists
    #[test]
    fn used_messages_exist() {
        fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs" || e == "html") {
                    out.push((
                        path.display().to_string(),
                        std::fs::read_to_string(&path).unwrap(),
                    ));
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        walk(&root.join("templates"), &mut files);
        walk(&root.join("src/web"), &mut files);
        let known = all_messages(Lang::De);
        for (path, text) in files {
            if path.ends_with("i18n.rs") {
                continue;
            }
            for call in [".get(\"", ".get1(\"", ".get2(\"", ".fmt(\""] {
                for (pos, _) in text.match_indices(call) {
                    let rest = &text[pos + call.len()..];
                    let id: String = rest.chars().take_while(|c| *c != '"').collect();
                    if id.is_empty() || id.contains(' ') {
                        continue;
                    }
                    assert!(known.contains_key(&id), "{path}: unknown message {id}");
                }
            }
        }
    }

    #[test]
    fn accept_language_picks_the_best_supported_language() {
        assert_eq!(
            Lang::from_accept_language("de-DE,de;q=0.9,en;q=0.8"),
            Lang::De
        );
        assert_eq!(
            Lang::from_accept_language("fr-FR,en;q=0.5,de;q=0.4"),
            Lang::En
        );
        assert_eq!(Lang::from_accept_language("en;q=0.2, de;q=0.7"), Lang::De);
        assert_eq!(Lang::from_accept_language("fr"), Lang::En);
        assert_eq!(Lang::from_accept_language(""), Lang::En);
    }

    #[test]
    fn variables_are_filled_in_and_numbers_follow_the_language() {
        assert_eq!(Lang::De.get1("latency-ms", "ms", 1.5), "1,5 ms");
        assert_eq!(Lang::En.get1("latency-ms", "ms", 1.5), "1.5 ms");
        assert_eq!(Lang::De.get("no-such-message"), "no-such-message");
        assert!(Lang::De.js_texts().starts_with('{'));
    }
}
