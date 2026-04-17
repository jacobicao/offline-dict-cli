use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::{
    dedupe_preserve_order, normalize_english, normalize_tags, DictionaryEntry, PersistedDictionary,
    Tag,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDocument {
    pub tag: Option<Tag>,
    pub entries: Vec<SourceDictionaryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SourceDictionaryEntry {
    pub word: String,
    #[serde(default)]
    pub translations: Vec<SourceTranslation>,
    #[serde(default)]
    pub phrases: Vec<SourcePhrase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SourceTranslation {
    pub translation: String,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SourcePhrase {
    pub phrase: String,
    pub translation: String,
}

#[derive(Debug, Clone, Default)]
struct EntryAccumulator {
    definitions: Vec<String>,
    tags: Vec<Tag>,
}

pub fn build_persisted_dictionary(documents: &[SourceDocument]) -> PersistedDictionary {
    let mut entries_by_headword: BTreeMap<String, EntryAccumulator> = BTreeMap::new();

    for document in documents {
        for entry in &document.entries {
            let headword = normalize_english(&entry.word);
            let definitions = entry
                .translations
                .iter()
                .flat_map(|translation| split_translation(&translation.translation))
                .collect::<Vec<_>>();

            push_entry(
                &mut entries_by_headword,
                headword,
                definitions,
                document.tag.into_iter().collect(),
            );

            for phrase in &entry.phrases {
                push_entry(
                    &mut entries_by_headword,
                    normalize_english(&phrase.phrase),
                    split_translation(&phrase.translation),
                    Vec::new(),
                );
            }
        }
    }

    PersistedDictionary {
        entries: entries_by_headword
            .into_iter()
            .map(|(headword, accumulator)| DictionaryEntry {
                headword,
                definitions: dedupe_preserve_order(accumulator.definitions),
                tags: normalize_tags(accumulator.tags),
            })
            .collect(),
    }
}

pub fn load_source_documents_from_directory(root: &Path) -> Result<Vec<SourceDocument>, String> {
    let json_dir = root.join("json");
    let source_specs = [
        ("1-初中-顺序.json", Some(Tag::Common3500)),
        ("2-高中-顺序.json", None),
        ("3-CET4-顺序.json", Some(Tag::Cet4)),
        ("4-CET6-顺序.json", Some(Tag::Cet6)),
        ("5-考研-顺序.json", None),
        ("6-托福-顺序.json", None),
        ("7-SAT-顺序.json", None),
    ];

    let mut documents = Vec::new();

    for (file_name, tag) in source_specs {
        let path = json_dir.join(file_name);
        if !path.exists() {
            continue;
        }

        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read source file {}: {error}", path.display()))?;
        let entries: Vec<SourceDictionaryEntry> = serde_json::from_str(&contents)
            .map_err(|error| format!("failed to parse source file {}: {error}", path.display()))?;

        documents.push(SourceDocument { tag, entries });
    }

    if documents.is_empty() {
        return Err(format!(
            "no source files found under {}",
            json_dir.display()
        ));
    }

    Ok(documents)
}

fn push_entry(
    entries_by_headword: &mut BTreeMap<String, EntryAccumulator>,
    headword: String,
    definitions: Vec<String>,
    tags: Vec<Tag>,
) {
    if headword.is_empty() {
        return;
    }

    let accumulator = entries_by_headword.entry(headword).or_default();
    accumulator.definitions.extend(definitions);
    accumulator.tags.extend(tags);
}

fn split_translation(raw: &str) -> Vec<String> {
    let cleaned = strip_leading_noise(raw);
    if cleaned.is_empty() {
        return Vec::new();
    }

    cleaned
        .split(['；', ';', '，', ',', '、'])
        .map(strip_leading_noise)
        .map(|part| trim_edge_punctuation(&part))
        .filter(|part| !part.is_empty())
        .collect()
}

fn strip_leading_noise(input: &str) -> String {
    let mut current = input.trim();

    loop {
        let Some(stripped) = strip_leading_bracket_group(current) else {
            break;
        };
        current = stripped.trim();
    }

    loop {
        let next = strip_part_of_speech_prefix(current);
        if next == current {
            break;
        }
        current = next.trim();
    }

    current.to_string()
}

fn strip_leading_bracket_group(input: &str) -> Option<&str> {
    for (open, close) in [('[' as char, ']'), ('(' as char, ')'), ('（', '）')] {
        if input.starts_with(open) {
            let mut seen_open = false;
            for (index, character) in input.char_indices() {
                if character == open {
                    seen_open = true;
                }
                if seen_open && character == close {
                    return Some(&input[index + character.len_utf8()..]);
                }
            }
        }
    }

    None
}

fn strip_part_of_speech_prefix(input: &str) -> &str {
    const PREFIXES: [&str; 13] = [
        "n.", "v.", "vt.", "vi.", "adj.", "adv.", "prep.", "pron.", "conj.", "int.", "num.",
        "art.", "aux.",
    ];

    for prefix in PREFIXES {
        if let Some(remainder) = input.strip_prefix(prefix) {
            return remainder.trim_start();
        }
    }

    input
}

fn trim_edge_punctuation(input: &str) -> String {
    input
        .trim_matches(|character: char| {
            character.is_whitespace()
                || matches!(
                    character,
                    ':' | '：' | '.' | '。' | ' ' | '“' | '”' | '"' | '\''
                )
        })
        .to_string()
}
