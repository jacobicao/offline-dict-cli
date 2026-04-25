use std::path::Path;

use offline_dict_cli::{Dictionary, QueryKind, Tag};

fn generated_dictionary() -> Dictionary {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("generated")
        .join("dictionary.json");
    Dictionary::from_json_path(&path).unwrap_or_else(|error| {
        panic!(
            "generated dictionary is required for golden query tests at {}: {error}",
            path.display()
        )
    })
}

fn first_results<'a>(dictionary_results: &'a [String], limit: usize) -> Vec<&'a str> {
    dictionary_results
        .iter()
        .take(limit)
        .map(String::as_str)
        .collect()
}

#[test]
fn generated_dataset_preserves_core_english_query_outputs() {
    let dictionary = generated_dictionary();

    let abandon = dictionary.lookup("abandon", false).expect("lookup abandon");
    assert_eq!(abandon.kind, QueryKind::English);
    assert_eq!(abandon.display_tag, Some(Tag::Cet4));
    assert_eq!(
        first_results(&abandon.results, 7),
        vec!["放任", "狂热", "遗弃", "放弃", "丢弃", "抛弃", "离弃"]
    );

    let apple = dictionary.lookup("Apple", false).expect("lookup apple");
    assert_eq!(apple.kind, QueryKind::English);
    assert_eq!(apple.display_tag, Some(Tag::Common3500));
    assert_eq!(
        first_results(&apple.results, 5),
        vec!["苹果", "苹果树", "苹果似的东西", "炸弹", "手榴弹"]
    );

    let ability = dictionary.lookup("ability", false).expect("lookup ability");
    assert_eq!(ability.display_tag, Some(Tag::Common3500));
    assert_eq!(
        first_results(&ability.results, 4),
        vec!["能力", "能耐", "才能", "本领"]
    );
}

#[test]
fn generated_dataset_preserves_core_chinese_reverse_lookup_outputs() {
    let dictionary = generated_dictionary();

    let give_up = dictionary.lookup("放弃", false).expect("reverse lookup");
    assert_eq!(give_up.kind, QueryKind::Chinese);
    assert_eq!(give_up.total_results, 23);
    assert_eq!(
        first_results(&give_up.results, 5),
        vec!["drop", "abandon", "compromise", "desert", "discard"]
    );

    let apple = dictionary.lookup("苹果", false).expect("reverse lookup");
    assert_eq!(apple.total_results, 1);
    assert_eq!(first_results(&apple.results, 1), vec!["apple"]);

    let ability = dictionary.lookup("能力", false).expect("reverse lookup");
    assert_eq!(ability.total_results, 8);
    assert_eq!(
        first_results(&ability.results, 5),
        vec!["ability", "capacity", "facility", "faculty", "power"]
    );
}
