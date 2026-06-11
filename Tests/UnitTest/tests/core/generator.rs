#[allow(dead_code)]
#[path = "..\\..\\..\\..\\Core\\src\\bin\\generator.rs"]
mod generator_source;

mod generator_tests {
    use super::generator_source::{
        english_freq_rank, extract_cn_terms, is_clean_word, is_inflection_of_other,
        parse_inflections, parse_translation, split_pos, strip_brackets,
    };

    #[test]
    fn clean_word_filters_shape() {
        assert!(is_clean_word("indicator"));
        assert!(is_clean_word("i"));
        assert!(!is_clean_word("e"));
        assert!(!is_clean_word("two words"));
        assert!(!is_clean_word("co-op"));
        assert!(!is_clean_word("3d"));
    }

    #[test]
    fn inflection_of_other_detected() {
        assert!(is_inflection_of_other("0:run/1:i", "running"));
        assert!(!is_inflection_of_other("i:running/3:runs", "run"));
        assert!(!is_inflection_of_other("", "run"));
    }

    #[test]
    fn split_pos_extracts_known_pos() {
        assert_eq!(
            split_pos("n. 苹果, 家伙"),
            ("n.".to_string(), "苹果, 家伙".to_string())
        );
        assert_eq!(
            split_pos("vt. 申请"),
            ("vt.".to_string(), "申请".to_string())
        );
        assert_eq!(
            split_pos("[计] 指示器"),
            (String::new(), "[计] 指示器".to_string())
        );
    }

    #[test]
    fn parse_translation_builds_major_and_defs() {
        let (major, defs) = parse_translation("n. 指示器, 指示剂\n[计] 指示器");
        assert_eq!(major.as_deref(), Some("指示器, 指示剂"));
        let defs = defs.unwrap();
        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].pos, "n.");
        assert_eq!(defs[0].meanings, vec!["指示器, 指示剂".to_string()]);
        assert_eq!(defs[1].pos, "");
    }

    #[test]
    fn parse_inflections_maps_exchange() {
        let inf = parse_inflections("d:perceived/p:perceived/3:perceives/i:perceiving").unwrap();
        assert_eq!(inf.past_tense.as_deref(), Some("perceived"));
        assert_eq!(inf.past_participle.as_deref(), Some("perceived"));
        assert_eq!(inf.third_singular.as_deref(), Some("perceives"));
        assert_eq!(inf.present_participle.as_deref(), Some("perceiving"));
        assert!(inf.plural.is_none());
        assert!(parse_inflections("").is_none());
    }

    #[test]
    fn strip_brackets_removes_tags() {
        assert_eq!(strip_brackets("[计] 指示器"), " 指示器");
        assert_eq!(strip_brackets("（使）弯曲"), "弯曲");
        assert_eq!(strip_brackets("苹果"), "苹果");
    }

    #[test]
    fn extract_cn_terms_splits_and_filters() {
        assert_eq!(extract_cn_terms("n. 苹果, 家伙"), vec!["苹果", "家伙"]);
        assert_eq!(extract_cn_terms("[计] 指示器"), vec!["指示器"]);
        assert_eq!(extract_cn_terms("vt. 把…弄弯, abc, 弯曲"), vec!["弯曲"]);
        assert_eq!(extract_cn_terms("n. 苹果\n[医] 苹果"), vec!["苹果"]);
    }

    #[test]
    fn english_freq_rank_orders_frq_before_bnc() {
        let frq = english_freq_rank(2695, 2446).unwrap();
        let bnc_only = english_freq_rank(0, 3000).unwrap();
        assert!(frq < bnc_only, "frq-ranked must sort before bnc-only");
        assert!(english_freq_rank(0, 0).is_none());
    }
}
