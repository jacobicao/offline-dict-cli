use offline_dict_cli::{
    format_result, Dictionary, DictionaryEntry, LookupError, PersistedDictionary, QueryKind, Tag,
};

fn sample_dictionary() -> Dictionary {
    Dictionary::from_entries(vec![
        DictionaryEntry {
            headword: "abandon".to_string(),
            definitions: vec!["放弃".to_string(), "遗弃".to_string(), "沉湎于".to_string()],
            tags: vec![Tag::Cet4, Tag::Cet6],
        },
        DictionaryEntry {
            headword: "forsake".to_string(),
            definitions: vec!["放弃".to_string()],
            tags: vec![Tag::Tem4],
        },
        DictionaryEntry {
            headword: "quit".to_string(),
            definitions: vec!["放弃".to_string()],
            tags: vec![Tag::Common3500],
        },
        DictionaryEntry {
            headword: "renounce".to_string(),
            definitions: vec!["放弃".to_string()],
            tags: vec![Tag::Gre],
        },
        DictionaryEntry {
            headword: "relinquish".to_string(),
            definitions: vec!["放弃".to_string()],
            tags: vec![Tag::Tem8],
        },
        DictionaryEntry {
            headword: "surrender".to_string(),
            definitions: vec!["放弃".to_string()],
            tags: vec![Tag::Cet6],
        },
        DictionaryEntry {
            headword: "give up".to_string(),
            definitions: vec!["放弃".to_string()],
            tags: vec![],
        },
    ])
}

#[test]
fn english_lookup_is_case_insensitive_and_formats_tags() {
    let dictionary = sample_dictionary();

    let result = dictionary
        .lookup("Abandon", false)
        .expect("lookup should work");

    assert_eq!(result.kind, QueryKind::English);
    assert_eq!(result.displayed_query, "abandon");
    assert_eq!(result.tags, vec![Tag::Cet4, Tag::Cet6]);
    assert_eq!(result.display_tag, Some(Tag::Cet4));
    assert_eq!(
        result.results,
        vec!["放弃".to_string(), "遗弃".to_string(), "沉湎于".to_string()]
    );
    assert_eq!(result.total_results, 3);

    assert_eq!(
        format_result(&result, false),
        "abandon\ntags: CET4\n1. 放弃\n2. 遗弃\n3. 沉湎于"
    );
}

#[test]
fn english_phrase_lookup_is_exact() {
    let dictionary = sample_dictionary();

    let result = dictionary
        .lookup("give up", false)
        .expect("phrase should match");

    assert_eq!(result.kind, QueryKind::English);
    assert_eq!(result.displayed_query, "give up");
    assert_eq!(result.display_tag, None);
    assert!(result.tags.is_empty());
    assert_eq!(result.results, vec!["放弃".to_string()]);
}

#[test]
fn common_3500_is_the_lowest_display_tag() {
    let dictionary = sample_dictionary();

    let result = dictionary
        .lookup("quit", false)
        .expect("lookup should work");

    assert_eq!(result.tags, vec![Tag::Common3500]);
    assert_eq!(result.display_tag, Some(Tag::Common3500));
    assert_eq!(
        format_result(&result, false),
        "quit\ntags: COMMON_3500\n1. 放弃"
    );
}

#[test]
fn chinese_lookup_is_ranked_and_truncated_by_default() {
    let dictionary = sample_dictionary();

    let result = dictionary
        .lookup("放弃", false)
        .expect("reverse lookup should work");

    assert_eq!(result.kind, QueryKind::Chinese);
    assert_eq!(result.displayed_query, "放弃");
    assert_eq!(result.display_tag, None);
    assert!(result.tags.is_empty());
    assert_eq!(
        result.results,
        vec![
            "quit".to_string(),
            "abandon".to_string(),
            "surrender".to_string(),
            "forsake".to_string(),
            "relinquish".to_string()
        ]
    );
    assert_eq!(result.total_results, 7);

    assert_eq!(
        format_result(&result, false),
        "放弃\n1. quit\n2. abandon\n3. surrender\n4. forsake\n5. relinquish\n\n5 of 7 results, use --all to show more"
    );
}

#[test]
fn chinese_lookup_can_show_all_results() {
    let dictionary = sample_dictionary();

    let result = dictionary
        .lookup("放弃", true)
        .expect("reverse lookup should work");

    assert_eq!(
        result.results,
        vec![
            "quit".to_string(),
            "abandon".to_string(),
            "surrender".to_string(),
            "forsake".to_string(),
            "relinquish".to_string(),
            "renounce".to_string(),
            "give up".to_string()
        ]
    );

    assert_eq!(
        format_result(&result, true),
        "放弃\n1. quit\n2. abandon\n3. surrender\n4. forsake\n5. relinquish\n6. renounce\n7. give up"
    );
}

#[test]
fn missing_query_returns_not_found() {
    let dictionary = sample_dictionary();

    let error = dictionary
        .lookup("missing", false)
        .expect_err("should miss");

    assert_eq!(
        error,
        LookupError::NotFound {
            query: "missing".to_string()
        }
    );
}

#[test]
fn persisted_dictionary_shape_is_serializable() {
    let persisted = PersistedDictionary {
        entries: vec![DictionaryEntry {
            headword: "abandon".to_string(),
            definitions: vec!["放弃".to_string()],
            tags: vec![Tag::Cet4],
        }],
    };

    let json = serde_json::to_string(&persisted).expect("serialize");
    let decoded: PersistedDictionary = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(decoded, persisted);
}
