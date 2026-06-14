#![cfg(target_os = "windows")]

mod logging {
    pub(crate) fn log_message(_msg: &str) {}
}

#[allow(dead_code)]
#[path = "..\\..\\..\\App\\Win\\src\\dict.rs"]
mod dict;

mod dict_config_tests {
    use super::dict::{parse_dictionary_package_config, DictionaryPackageConfig};

    #[test]
    fn parses_dictionary_package_config() {
        let raw = "[Dictionary]\nEnglishPrefix=word_en_es\nTargetPrefix=word_es\n";
        assert_eq!(
            parse_dictionary_package_config(raw),
            Some(DictionaryPackageConfig {
                english_prefix: "word_en_es".to_string(),
                target_prefix: "word_es".to_string(),
            })
        );
    }

    #[test]
    fn rejects_missing_dictionary_prefixes() {
        assert_eq!(
            parse_dictionary_package_config("[Dictionary]\nEnglishPrefix=word_en_es\n"),
            None
        );
        assert_eq!(
            parse_dictionary_package_config("[Dictionary]\nTargetPrefix=word_es\n"),
            None
        );
    }

    #[test]
    fn rejects_invalid_dictionary_prefix_characters() {
        assert_eq!(
            parse_dictionary_package_config(
                "[Dictionary]\nEnglishPrefix=word-en-es\nTargetPrefix=word_es\n"
            ),
            None
        );
    }
}
