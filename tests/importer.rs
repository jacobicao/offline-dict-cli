use offline_dict_cli::importer::{
    build_persisted_dictionary, load_source_documents_from_directory, summarize_source_documents,
    SourceDictionaryEntry, SourceDocument, SourcePhrase, SourceTranslation,
};
use offline_dict_cli::{DictionaryEntry, PersistedDictionary, Tag};
use std::fs;

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir().join(format!("offline-dict-cli-{nanos}-{name}"))
}

#[test]
fn importer_merges_duplicate_headwords_and_ignores_phrases() {
    let persisted = build_persisted_dictionary(&[
        SourceDocument {
            tag: Some(Tag::Cet4),
            entries: vec![SourceDictionaryEntry {
                word: "Abandon".to_string(),
                translations: vec![SourceTranslation {
                    translation: "放弃；遗弃".to_string(),
                    kind: Some("v".to_string()),
                }],
                phrases: vec![SourcePhrase {
                    phrase: "abandon ship".to_string(),
                    translation: "弃船".to_string(),
                }],
            }],
        },
        SourceDocument {
            tag: Some(Tag::Cet6),
            entries: vec![SourceDictionaryEntry {
                word: "abandon".to_string(),
                translations: vec![SourceTranslation {
                    translation: "遗弃；沉湎于".to_string(),
                    kind: Some("v".to_string()),
                }],
                phrases: Vec::new(),
            }],
        },
    ]);

    assert_eq!(
        persisted,
        PersistedDictionary {
            entries: vec![DictionaryEntry {
                headword: "abandon".to_string(),
                definitions: vec!["放弃".to_string(), "遗弃".to_string(), "沉湎于".to_string()],
                tags: vec![Tag::Cet4, Tag::Cet6],
            }]
        }
    );
}

#[test]
fn importer_strips_common_annotation_noise_from_translations() {
    let persisted = build_persisted_dictionary(&[SourceDocument {
        tag: Some(Tag::Common3500),
        entries: vec![SourceDictionaryEntry {
            word: "Access".to_string(),
            translations: vec![
                SourceTranslation {
                    translation: "[计] 访问；接近".to_string(),
                    kind: Some("v".to_string()),
                },
                SourceTranslation {
                    translation: "n. 入口；通道".to_string(),
                    kind: Some("n".to_string()),
                },
            ],
            phrases: Vec::new(),
        }],
    }]);

    assert_eq!(
        persisted,
        PersistedDictionary {
            entries: vec![DictionaryEntry {
                headword: "access".to_string(),
                definitions: vec![
                    "访问".to_string(),
                    "接近".to_string(),
                    "入口".to_string(),
                    "通道".to_string()
                ],
                tags: vec![Tag::Common3500],
            }]
        }
    );
}

#[test]
fn importer_loads_known_source_files_and_applies_tag_mapping() {
    let root = unique_temp_dir("source-root");
    let json_dir = root.join("json");
    fs::create_dir_all(&json_dir).expect("create json dir");

    fs::write(
        json_dir.join("1-初中-顺序.json"),
        r#"[{"word":"Ability","translations":[{"translation":"能力","type":"n"}]}]"#,
    )
    .expect("write junior file");
    fs::write(
        json_dir.join("3-CET4-顺序.json"),
        r#"[{"word":"Abandon","translations":[{"translation":"放弃","type":"v"}]}]"#,
    )
    .expect("write cet4 file");
    fs::write(
        json_dir.join("5-考研-顺序.json"),
        r#"[{"word":"Abstract","translations":[{"translation":"摘要","type":"n"}]}]"#,
    )
    .expect("write graduate file");

    let documents = load_source_documents_from_directory(&root).expect("load source docs");

    fs::remove_dir_all(&root).ok();

    assert_eq!(documents.len(), 3);
    assert_eq!(documents[0].tag, Some(Tag::Common3500));
    assert_eq!(documents[1].tag, Some(Tag::Cet4));
    assert_eq!(documents[2].tag, None);
    assert_eq!(documents[0].entries[0].word, "Ability");
    assert_eq!(documents[1].entries[0].word, "Abandon");
    assert_eq!(documents[2].entries[0].word, "Abstract");
}

#[test]
fn importer_summarizes_headword_and_phrase_counts_separately() {
    let summary = summarize_source_documents(&[
        SourceDocument {
            tag: Some(Tag::Cet4),
            entries: vec![
                SourceDictionaryEntry {
                    word: "Abandon".to_string(),
                    translations: vec![SourceTranslation {
                        translation: "放弃".to_string(),
                        kind: Some("v".to_string()),
                    }],
                    phrases: vec![SourcePhrase {
                        phrase: "abandon ship".to_string(),
                        translation: "弃船".to_string(),
                    }],
                },
                SourceDictionaryEntry {
                    word: "abandon".to_string(),
                    translations: vec![SourceTranslation {
                        translation: "遗弃".to_string(),
                        kind: Some("v".to_string()),
                    }],
                    phrases: vec![SourcePhrase {
                        phrase: "abandon ship".to_string(),
                        translation: "弃船".to_string(),
                    }],
                },
            ],
        },
        SourceDocument {
            tag: None,
            entries: vec![SourceDictionaryEntry {
                word: "give up".to_string(),
                translations: vec![SourceTranslation {
                    translation: "放弃".to_string(),
                    kind: Some("v".to_string()),
                }],
                phrases: vec![SourcePhrase {
                    phrase: "give up on".to_string(),
                    translation: "对…放弃希望".to_string(),
                }],
            }],
        },
    ]);

    assert_eq!(summary.unique_headwords, 1);
    assert_eq!(summary.ignored_unique_phrases, 2);
    assert_eq!(summary.ignored_multi_word_headwords, 1);
    assert_eq!(summary.total_exact_query_keys, 1);
}

#[test]
fn importer_summary_deduplicates_overlap_between_headwords_and_phrases() {
    let summary = summarize_source_documents(&[SourceDocument {
        tag: None,
        entries: vec![
            SourceDictionaryEntry {
                word: "give up".to_string(),
                translations: vec![SourceTranslation {
                    translation: "放弃".to_string(),
                    kind: Some("v".to_string()),
                }],
                phrases: Vec::new(),
            },
            SourceDictionaryEntry {
                word: "abandon".to_string(),
                translations: vec![SourceTranslation {
                    translation: "放弃".to_string(),
                    kind: Some("v".to_string()),
                }],
                phrases: vec![SourcePhrase {
                    phrase: "give up".to_string(),
                    translation: "放弃".to_string(),
                }],
            },
        ],
    }]);

    assert_eq!(summary.unique_headwords, 1);
    assert_eq!(summary.ignored_unique_phrases, 1);
    assert_eq!(summary.ignored_multi_word_headwords, 1);
    assert_eq!(summary.total_exact_query_keys, 1);
}

#[test]
fn importer_ignores_multi_word_headwords() {
    let persisted = build_persisted_dictionary(&[SourceDocument {
        tag: Some(Tag::Cet4),
        entries: vec![
            SourceDictionaryEntry {
                word: "abandon".to_string(),
                translations: vec![SourceTranslation {
                    translation: "放弃".to_string(),
                    kind: Some("v".to_string()),
                }],
                phrases: Vec::new(),
            },
            SourceDictionaryEntry {
                word: "abandon ship".to_string(),
                translations: vec![SourceTranslation {
                    translation: "弃船".to_string(),
                    kind: Some("v".to_string()),
                }],
                phrases: Vec::new(),
            },
        ],
    }]);

    assert_eq!(
        persisted,
        PersistedDictionary {
            entries: vec![DictionaryEntry {
                headword: "abandon".to_string(),
                definitions: vec!["放弃".to_string()],
                tags: vec![Tag::Cet4],
            }]
        }
    );
}
