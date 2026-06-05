//! Integration tests for polymorphic `record` serialization.

use ee_core::{Record, RecordSerialized, RecordType, SerializableRecord, WordEn, Pronunciation, Definition, Inflections, Example, Note, History};

#[test]
fn test_word_en_serialization_and_deserialization() {
    let word = WordEn {
        word: "apply".to_string(),
        pronunciation: Some(Pronunciation {
            ipa: "əˈplaɪ".to_string(),
            audio: Some("audio/apply.mp3".to_string()),
            audio_url: Some("https://cdn.easyenglish.org/audio/apply.mp3".to_string()),
        }),
        definitions: Some(vec![
            Definition {
                pos: "v.".to_string(),
                meanings: vec!["申请".to_string(), "应用".to_string()],
            }
        ]),
        inflections: Some(Inflections {
            plural: None,
            past_tense: Some("applied".to_string()),
            past_participle: Some("applied".to_string()),
            present_participle: Some("applying".to_string()),
            third_singular: Some("applies".to_string()),
        }),
        examples: Some(vec![
            Example {
                en: "You can apply online.".to_string(),
                zh: "你可以线上的申请。".to_string(),
            }
        ]),
    };

    // Serialize
    let serialized_str = word.serialize().expect("serialize word");
    
    // Low-level record wrap
    let record = Record::new("dict", "apply", serialized_str);
    
    // Verify type
    assert_eq!(record.record_type(), RecordType::WordEn);
    
    // Deserialize polymorphicly
    let model = record.deserialize_to_model().expect("deserialize");
    
    if let RecordSerialized::WordEn(deserialized_word) = model {
        assert_eq!(deserialized_word.word, "apply");
        assert_eq!(deserialized_word.pronunciation.unwrap().ipa, "əˈplaɪ");
        assert_eq!(deserialized_word.inflections.unwrap().past_tense.unwrap(), "applied");
    } else {
        panic!("Expected WordEn variant!");
    }
}

#[test]
fn test_note_and_history_plain_serialization() {
    // 1. Notes
    let note_record = Record::new("notes", "apply", "\"user note content\"");
    assert_eq!(note_record.record_type(), RecordType::Note);
    
    let note_model = note_record.deserialize_to_model().expect("deserialize note");
    assert_eq!(note_model, RecordSerialized::Note(Note { content: "user note content".to_string() }));
    
    // 2. History
    let history_record = Record::new("history", "apply", "\"1780672001108\"");
    assert_eq!(history_record.record_type(), RecordType::History);
    
    let history_model = history_record.deserialize_to_model().expect("deserialize history");
    assert_eq!(history_model, RecordSerialized::History(History { content: "1780672001108".to_string() }));
}
