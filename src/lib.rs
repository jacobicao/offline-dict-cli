use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::history::{
    default_date_range, format_search_log, parse_date, resolve_history_dir, HistoryStore,
};

const DEFAULT_CHINESE_RESULTS: usize = 5;
const EMBEDDED_DATASET: &str = include_str!(concat!(env!("OUT_DIR"), "/embedded_dictionary.json"));

pub mod history;
pub mod importer;

pub use history::{Clock, SystemClock};

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
    pub display_tag: Option<Tag>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Lookup {
        query: String,
        show_all: bool,
    },
    SearchLog {
        from: Option<String>,
        to: Option<String>,
    },
    Help,
    Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub dataset_path: Option<PathBuf>,
    pub history_dir: Option<PathBuf>,
    pub local_app_data: Option<PathBuf>,
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
            display_tag: lowest_display_tag(&entry.tags),
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
            display_tag: None,
            tags: Vec::new(),
            results,
            total_results,
        })
    }
}

pub fn parse_command<I, S>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let arguments: Vec<String> = args.into_iter().map(Into::into).collect();

    if arguments.iter().any(|argument| argument == "--help") {
        return Ok(Command::Help);
    }

    if arguments.iter().any(|argument| argument == "--version") {
        return Ok(Command::Version);
    }

    let mut show_all = false;
    let mut positionals = Vec::new();
    let mut search_log_mode = false;
    let mut from = None;
    let mut to = None;
    let mut index = 0;

    while index < arguments.len() {
        let argument = &arguments[index];
        match argument.as_str() {
            "--all" => {
                if search_log_mode {
                    return Err(unexpected_argument(argument));
                }
                show_all = true;
                index += 1;
            }
            "--from" => {
                if !search_log_mode {
                    return Err(unexpected_argument(argument));
                }
                index += 1;
                let Some(value) = arguments.get(index).cloned() else {
                    return Err("error: missing value for '--from'".to_string());
                };
                from = Some(value);
                index += 1;
            }
            "--to" => {
                if !search_log_mode {
                    return Err(unexpected_argument(argument));
                }
                index += 1;
                let Some(value) = arguments.get(index).cloned() else {
                    return Err("error: missing value for '--to'".to_string());
                };
                to = Some(value);
                index += 1;
            }
            flag if flag.starts_with('-') => return Err(unexpected_argument(flag)),
            "search-log" if positionals.is_empty() && !show_all && !search_log_mode => {
                search_log_mode = true;
                index += 1;
            }
            value => {
                if search_log_mode {
                    return Err(unexpected_argument(value));
                }
                positionals.push(value.to_string());
                index += 1;
            }
        }
    }

    if search_log_mode {
        if from.is_some() ^ to.is_some() {
            return Err("error: --from and --to must be provided together".to_string());
        }

        return Ok(Command::SearchLog { from, to });
    }

    if positionals.is_empty() {
        return Ok(Command::Help);
    }

    Ok(Command::Lookup {
        query: positionals.join(" "),
        show_all,
    })
}

pub fn execute_command(
    command: Command,
    runtime: &RuntimeConfig,
    clock: &dyn Clock,
) -> Result<CommandOutput, String> {
    match command {
        Command::Help => Ok(CommandOutput {
            stdout: help_text(),
            stderr: String::new(),
            exit_code: 0,
        }),
        Command::Version => Ok(CommandOutput {
            stdout: format!("{}\n", version_text()),
            stderr: String::new(),
            exit_code: 0,
        }),
        Command::Lookup { query, show_all } => execute_lookup(query, show_all, runtime, clock),
        Command::SearchLog { from, to } => execute_search_log(from, to, runtime, clock),
    }
}

pub fn runtime_config_from_env() -> RuntimeConfig {
    RuntimeConfig {
        dataset_path: env::var_os("OFFLINE_DICT_DATASET").map(PathBuf::from),
        history_dir: env::var_os("OFFLINE_DICT_HISTORY_DIR").map(PathBuf::from),
        local_app_data: env::var_os("LOCALAPPDATA").map(PathBuf::from),
    }
}

pub fn format_result(result: &LookupResult, show_all: bool) -> String {
    let mut lines = Vec::new();
    lines.push(result.displayed_query.clone());

    if matches!(result.kind, QueryKind::English) {
        if let Some(tag) = result.display_tag {
            lines.push(format!("tags: {}", tag.label()));
        }
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
        "Usage:\n  dict [--all] <query>\n  dict search-log\n  dict search-log --from YYYY-MM-DD --to YYYY-MM-DD\n  dict --help\n  dict --version\n\nVersion:\n  {}\n",
        env!("CARGO_PKG_VERSION")
    )
}

pub fn version_text() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn execute_lookup(
    query: String,
    show_all: bool,
    runtime: &RuntimeConfig,
    clock: &dyn Clock,
) -> Result<CommandOutput, String> {
    let dictionary = load_dictionary(runtime.dataset_path.as_deref())?;

    match dictionary.lookup(&query, show_all) {
        Ok(result) => {
            let mut stderr = String::new();

            if matches!(result.kind, QueryKind::English) {
                if let Err(error) = record_lookup_history(runtime, clock, &result.displayed_query) {
                    stderr = format!("warning: failed to record history: {error}\n");
                }
            }

            Ok(CommandOutput {
                stdout: format!("{}\n", format_result(&result, show_all)),
                stderr,
                exit_code: 0,
            })
        }
        Err(LookupError::EmptyQuery) => Ok(CommandOutput {
            stdout: help_text(),
            stderr: String::new(),
            exit_code: 0,
        }),
        Err(LookupError::NotFound { .. }) => Ok(CommandOutput {
            stdout: format!("未找到精确匹配: {query}\n"),
            stderr: String::new(),
            exit_code: 1,
        }),
    }
}

fn execute_search_log(
    from: Option<String>,
    to: Option<String>,
    runtime: &RuntimeConfig,
    clock: &dyn Clock,
) -> Result<CommandOutput, String> {
    let history_store = history_store_from_config(runtime)?;
    let (from_date, to_date) = resolve_search_log_range(from.as_deref(), to.as_deref(), clock)?;
    let days = history_store.read_range(from_date, to_date)?;

    Ok(CommandOutput {
        stdout: format_search_log(&days),
        stderr: String::new(),
        exit_code: 0,
    })
}

fn history_store_from_config(runtime: &RuntimeConfig) -> Result<HistoryStore, String> {
    let directory = resolve_history_dir(
        runtime.history_dir.as_deref(),
        runtime.local_app_data.as_deref(),
    )?;
    Ok(HistoryStore::new(directory))
}

fn record_lookup_history(
    runtime: &RuntimeConfig,
    clock: &dyn Clock,
    headword: &str,
) -> Result<(), String> {
    let history_store = history_store_from_config(runtime)?;
    history_store.record_lookup(clock.today_local(), headword)
}

fn resolve_search_log_range(
    from: Option<&str>,
    to: Option<&str>,
    clock: &dyn Clock,
) -> Result<(chrono::NaiveDate, chrono::NaiveDate), String> {
    match (from, to) {
        (None, None) => Ok(default_date_range(clock)),
        (Some(from), Some(to)) => {
            let from_date = parse_date(from)?;
            let to_date = parse_date(to)?;
            if from_date > to_date {
                return Err("error: from date must be on or before to date".to_string());
            }
            Ok((from_date, to_date))
        }
        _ => Err("error: --from and --to must be provided together".to_string()),
    }
}

fn load_dictionary(dataset_path: Option<&Path>) -> Result<Dictionary, String> {
    if let Some(path) = dataset_path {
        return Dictionary::from_json_path(path);
    }

    Dictionary::embedded()
}

fn unexpected_argument(argument: &str) -> String {
    format!("error: unexpected argument '{argument}' found")
}

fn best_tag_priority(tags: &[Tag]) -> usize {
    tags.iter()
        .map(|tag| tag.priority())
        .min()
        .unwrap_or(usize::MAX)
}

fn lowest_display_tag(tags: &[Tag]) -> Option<Tag> {
    tags.iter().copied().min_by_key(|tag| tag.priority())
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

pub(crate) fn contains_chinese(input: &str) -> bool {
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
