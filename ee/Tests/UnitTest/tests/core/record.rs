//! Integration tests for polymorphic `record` serialization.

use ee_core::{
    Definition, Example, History, Inflections, Note, Pronunciation, Record, RecordModel,
    SerializableRecord, WordCn, WordEn,
};

#[test]
fn test_word_en_serialization_and_deserialization() {
    let word = WordEn {
        word: "apply".to_string(),
        major: Some("申请".to_string()),
        pronunciation: Some(Pronunciation {
            ipa: "əˈplaɪ".to_string(),
            audio: Some("audio/apply.mp3".to_string()),
            audio_url: Some("https://cdn.easyenglish.org/audio/apply.mp3".to_string()),
        }),
        definitions: Some(vec![Definition {
            pos: "v.".to_string(),
            meanings: vec!["申请".to_string(), "应用".to_string()],
        }]),
        inflections: Some(Inflections {
            plural: None,
            past_tense: Some("applied".to_string()),
            past_participle: Some("applied".to_string()),
            present_participle: Some("applying".to_string()),
            third_singular: Some("applies".to_string()),
        }),
        examples: Some(vec![Example {
            en: "You can apply online.".to_string(),
            zh: "你可以线上的申请。".to_string(),
        }]),
    };

    // Serialize
    let _serialized_str = word.serialize().expect("serialize word");

    // Wrap directly as a RecordModel to maintain internally tagged serializations
    let record_model = RecordModel::WordEn(word);
    let record_model_str = record_model.serialize().expect("serialize RecordModel");

    // Low-level record wrap
    let record = Record::new("apply", record_model_str);

    // Deserialize polymorphicly
    let model = record.deserialize().expect("deserialize");

    if let RecordModel::WordEn(deserialized_word) = model {
        assert_eq!(deserialized_word.word, "apply");
        assert_eq!(deserialized_word.pronunciation.unwrap().ipa, "əˈplaɪ");
        assert_eq!(
            deserialized_word.inflections.unwrap().past_tense.unwrap(),
            "applied"
        );
    } else {
        panic!("Expected WordEn variant!");
    }
}

#[test]
fn test_word_cn_serialization_and_deserialization() {
    let word = WordCn {
        word: "苹果".to_string(),
        english: vec!["apple".to_string(), "apples".to_string()],
    };

    let record_model = RecordModel::WordCn(word);
    let record_model_str = record_model.serialize().expect("serialize RecordModel");

    // The tag must identify the Chinese record variant.
    assert!(record_model_str.contains("\"record_type\":\"word_cn\""));

    let record = Record::new("苹果", record_model_str);
    let model = record.deserialize().expect("deserialize");

    if let RecordModel::WordCn(deserialized) = model {
        assert_eq!(deserialized.word, "苹果");
        assert_eq!(deserialized.english, vec!["apple", "apples"]);
    } else {
        panic!("Expected WordCn variant!");
    }
}

#[test]
fn test_note_and_history_plain_serialization() {
    // 1. Notes
    let note_model = RecordModel::Note(Note {
        content: "user note content".to_string(),
    });
    let note_str = note_model.serialize().expect("serialize note model");
    let note_record = Record::new("apply", note_str);

    let deserialized_note = note_record.deserialize().expect("deserialize note");
    assert_eq!(
        deserialized_note,
        RecordModel::Note(Note {
            content: "user note content".to_string()
        })
    );

    // 2. History
    let history_model = RecordModel::History(History {
        content: "1780672001108".to_string(),
    });
    let history_str = history_model.serialize().expect("serialize history model");
    let history_record = Record::new("apply", history_str);

    let deserialized_history = history_record.deserialize().expect("deserialize history");
    assert_eq!(
        deserialized_history,
        RecordModel::History(History {
            content: "1780672001108".to_string()
        })
    );
}
