use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const DEFAULT_CHINESE_RESULTS: usize = 5;
const EMBEDDED_DATASET: &str = include_str!("../data/generated/dictionary.json");

pub mod importer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tag {
    #[serde(rename = "COMMON_3500")]
    Common3500,
    #[serde(rename = "CET4")]
    Cet4,
    #[serde(rename = "CET6")]
    Cet6,
    #[serde(rename = "TEM4")]
    Tem4,
    #[serde(rename = "TEM8")]
    Tem8,
    #[serde(rename = "GRE")]
    Gre,
}

impl Tag {
    fn priority(self) -> usize {
        match self {
            Self::Common3500 => 0,
            Self::Cet4 => 1,
            Self::Cet6 => 2,
            Self::Tem4 => 3,
            Self::Tem8 => 4,
            Self::Gre => 5,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Common3500 => "COMMON_3500",
            Self::Cet4 => "CET4",
            Self::Cet6 => "CET6",
            Self::Tem4 => "TEM4",
            Self::Tem8 => "TEM8",
            Self::Gre => "GRE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub headword: String,
    pub definitions: Vec<String>,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedDictionary {
    pub entries: Vec<DictionaryEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryKind {
    English,
    Chinese,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupResult {
    pub kind: QueryKind,
    pub query: String,
    pub displayed_query: String,
    pub tags: Vec<Tag>,
    pub results: Vec<String>,
    pub total_results: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    EmptyQuery,
    NotFound { query: String },
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyQuery => write!(f, "empty query"),
            Self::NotFound { query } => write!(f, "未找到精确匹配: {query}"),
        }
    }
}

#[derive(Debug, Clone)]
struct EntryRecord {
    headword: String,
    normalized_headword: String,
    definitions: Vec<String>,
    tags: Vec<Tag>,
    best_priority: usize,
}

pub struct Dictionary {
    entries: Vec<EntryRecord>,
    english_index: HashMap<String, usize>,
    chinese_index: HashMap<String, Vec<usize>>,
}

impl Dictionary {
    pub fn from_entries(entries: Vec<DictionaryEntry>) -> Self {
        let mut records = Vec::with_capacity(entries.len());
        let mut english_index = HashMap::with_capacity(entries.len());
        let mut chinese_index: HashMap<String, Vec<usize>> = HashMap::new();

        for entry in entries {
            let normalized_headword = normalize_english(&entry.headword);
            let definitions = dedupe_preserve_order(entry.definitions);
            let tags = normalize_tags(entry.tags);
            let best_priority = best_tag_priority(&tags);
            let record_index = records.len();

            for definition in &definitions {
                chinese_index
                    .entry(normalize_chinese(definition))
                    .or_default()
                    .push(record_index);
            }

            english_index.insert(normalized_headword.clone(), record_index);
            records.push(EntryRecord {
                headword: entry.headword,
                normalized_headword,
                definitions,
                tags,
                best_priority,
            });
        }

        Self {
            entries: records,
            english_index,
            chinese_index,
        }
    }

    pub fn from_persisted(persisted: PersistedDictionary) -> Self {
        Self::from_entries(persisted.entries)
    }

    pub fn from_json_str(json: &str) -> Result<Self, String> {
        let persisted: PersistedDictionary = serde_json::from_str(json)
            .map_err(|error| format!("failed to parse dataset: {error}"))?;
        Ok(Self::from_persisted(persisted))
    }

    pub fn from_json_path(path: &Path) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|error| format!("failed to read dataset {}: {error}", path.display()))?;
        Self::from_json_str(&contents)
    }

    pub fn embedded() -> Result<Self, String> {
        Self::from_json_str(EMBEDDED_DATASET)
    }

    pub fn lookup(&self, query: &str, show_all: bool) -> Result<LookupResult, LookupError> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return Err(LookupError::EmptyQuery);
        }

        if contains_chinese(trimmed_query) {
            self.lookup_chinese(trimmed_query, show_all)
        } else {
            self.lookup_english(trimmed_query)
        }
    }

    fn lookup_english(&self, query: &str) -> Result<LookupResult, LookupError> {
        let normalized_query = normalize_english(query);
        let Some(index) = self.english_index.get(&normalized_query).copied() else {
            return Err(LookupError::NotFound {
                query: query.to_string(),
            });
        };

        let entry = &self.entries[index];
        Ok(LookupResult {
            kind: QueryKind::English,
            query: query.to_string(),
            displayed_query: entry.headword.clone(),
            tags: entry.tags.clone(),
            results: entry.definitions.clone(),
            total_results: entry.definitions.len(),
        })
    }

    fn lookup_chinese(&self, query: &str, show_all: bool) -> Result<LookupResult, LookupError> {
        let normalized_query = normalize_chinese(query);
        let Some(indices) = self.chinese_index.get(&normalized_query) else {
            return Err(LookupError::NotFound {
                query: query.to_string(),
            });
        };

        let mut matched_entries: Vec<&EntryRecord> =
            indices.iter().map(|index| &self.entries[*index]).collect();
        matched_entries.sort_by(|left, right| {
            left.best_priority
                .cmp(&right.best_priority)
                .then_with(|| left.normalized_headword.cmp(&right.normalized_headword))
        });

        let total_results = matched_entries.len();
        let limit = if show_all {
            total_results
        } else {
            total_results.min(DEFAULT_CHINESE_RESULTS)
        };
        let results = matched_entries
            .into_iter()
            .take(limit)
            .map(|entry| entry.headword.clone())
            .collect();

        Ok(LookupResult {
            kind: QueryKind::Chinese,
            query: query.to_string(),
            displayed_query: query.to_string(),
            tags: Vec::new(),
            results,
            total_results,
        })
    }
}

pub fn format_result(result: &LookupResult, show_all: bool) -> String {
    let mut lines = Vec::new();
    lines.push(result.displayed_query.clone());

    if matches!(result.kind, QueryKind::English) && !result.tags.is_empty() {
        let tags = result
            .tags
            .iter()
            .map(|tag| tag.label())
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!("tags: {tags}"));
    }

    for (index, item) in result.results.iter().enumerate() {
        lines.push(format!("{}. {item}", index + 1));
    }

    if matches!(result.kind, QueryKind::Chinese)
        && !show_all
        && result.total_results > result.results.len()
    {
        lines.push(String::new());
        lines.push(format!(
            "{} of {} results, use --all to show more",
            result.results.len(),
            result.total_results
        ));
    }

    lines.join("\n")
}

pub fn help_text() -> String {
    format!(
        "Usage:\n  dict [--all] <query>\n  dict --help\n  dict --version\n\nVersion:\n  {}\n",
        env!("CARGO_PKG_VERSION")
    )
}

pub fn version_text() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn best_tag_priority(tags: &[Tag]) -> usize {
    tags.iter()
        .map(|tag| tag.priority())
        .min()
        .unwrap_or(usize::MAX)
}

pub(crate) fn normalize_tags(tags: Vec<Tag>) -> Vec<Tag> {
    let mut deduped = Vec::new();

    for tag in tags {
        if !deduped.contains(&tag) {
            deduped.push(tag);
        }
    }

    deduped.sort_by_key(|tag| tag.priority());
    deduped
}

pub(crate) fn dedupe_preserve_order(items: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for item in items {
        let normalized = item.trim();
        if normalized.is_empty() {
            continue;
        }

        if deduped
            .iter()
            .all(|existing: &String| existing != normalized)
        {
            deduped.push(normalized.to_string());
        }
    }
    deduped
}

pub(crate) fn normalize_english(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn normalize_chinese(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join("")
}

fn contains_chinese(input: &str) -> bool {
    input.chars().any(is_cjk_unified_ideograph)
}

fn is_cjk_unified_ideograph(character: char) -> bool {
    matches!(
        character as u32,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0xF900..=0xFAFF
    )
}
