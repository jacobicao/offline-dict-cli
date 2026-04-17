use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{Duration, Local, NaiveDate};

use crate::{contains_chinese, normalize_english};

pub trait Clock {
    fn today_local(&self) -> NaiveDate;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn today_local(&self) -> NaiveDate {
        Local::now().date_naive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryDay {
    pub date: NaiveDate,
    pub words: Vec<String>,
}

pub struct HistoryStore {
    directory: PathBuf,
}

impl HistoryStore {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    pub fn record_lookup(&self, date: NaiveDate, headword: &str) -> Result<(), String> {
        let normalized = normalize_english(headword);
        if normalized.is_empty() {
            return Ok(());
        }

        self.ensure_writable_directory()?;

        let existing = self.read_words_for_date(date)?;
        if existing.iter().any(|word| word == &normalized) {
            return Ok(());
        }

        let path = self.file_path(date);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("failed to open history file {}: {error}", path.display()))?;

        writeln!(file, "{normalized}")
            .map_err(|error| format!("failed to write history file {}: {error}", path.display()))
    }

    pub fn read_range(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<HistoryDay>, String> {
        self.ensure_readable_directory()?;

        let mut days = Vec::new();
        let mut current = to;

        loop {
            days.push(HistoryDay {
                date: current,
                words: self.read_words_for_date(current)?,
            });

            if current == from {
                break;
            }

            current = current
                .checked_sub_signed(Duration::days(1))
                .expect("date subtraction should stay in range");
        }

        Ok(days)
    }

    fn ensure_writable_directory(&self) -> Result<(), String> {
        if self.directory.exists() {
            let metadata = fs::metadata(&self.directory).map_err(|error| {
                format!(
                    "failed to read history directory {}: {error}",
                    self.directory.display()
                )
            })?;
            if !metadata.is_dir() {
                return Err(format!(
                    "history path {} is not a directory",
                    self.directory.display()
                ));
            }
            return Ok(());
        }

        fs::create_dir_all(&self.directory).map_err(|error| {
            format!(
                "failed to create history directory {}: {error}",
                self.directory.display()
            )
        })
    }

    fn ensure_readable_directory(&self) -> Result<(), String> {
        if !self.directory.exists() {
            return Ok(());
        }

        let metadata = fs::metadata(&self.directory).map_err(|error| {
            format!(
                "failed to read history directory {}: {error}",
                self.directory.display()
            )
        })?;
        if !metadata.is_dir() {
            return Err(format!(
                "history path {} is not a directory",
                self.directory.display()
            ));
        }

        Ok(())
    }

    fn read_words_for_date(&self, date: NaiveDate) -> Result<Vec<String>, String> {
        let path = self.file_path(date);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(format!(
                    "failed to read history file {}: {error}",
                    path.display()
                ))
            }
        };

        let mut words = Vec::new();
        for line in contents.lines() {
            let normalized = normalize_english(line.trim());
            if normalized.is_empty() || contains_chinese(&normalized) {
                continue;
            }

            if !words.contains(&normalized) {
                words.push(normalized);
            }
        }

        Ok(words)
    }

    fn file_path(&self, date: NaiveDate) -> PathBuf {
        self.directory
            .join(format!("{}.txt", date.format("%Y-%m-%d")))
    }
}

pub fn resolve_history_dir(
    explicit_directory: Option<&Path>,
    local_app_data: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(directory) = explicit_directory {
        return Ok(directory.to_path_buf());
    }

    let Some(local_app_data) = local_app_data else {
        return Err(
            "error: LOCALAPPDATA is unavailable and OFFLINE_DICT_HISTORY_DIR is not set"
                .to_string(),
        );
    };

    Ok(local_app_data.join("offline-dict-cli").join("log"))
}

pub fn default_date_range(clock: &dyn Clock) -> (NaiveDate, NaiveDate) {
    let today = clock.today_local();
    let from = today
        .checked_sub_signed(Duration::days(6))
        .expect("date subtraction should stay in range");
    (from, today)
}

pub fn parse_date(input: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .map_err(|_| format!("error: invalid date '{input}', expected YYYY-MM-DD"))
}

pub fn format_search_log(days: &[HistoryDay]) -> String {
    let mut blocks = Vec::new();

    for day in days {
        let mut lines = vec![day.date.format("%Y-%m-%d").to_string()];
        if day.words.is_empty() {
            lines.push("(no queries)".to_string());
        } else {
            for (index, word) in day.words.iter().enumerate() {
                lines.push(format!("{}. {word}", index + 1));
            }
        }
        blocks.push(lines.join("\n"));
    }

    format!("{}\n", blocks.join("\n\n"))
}
