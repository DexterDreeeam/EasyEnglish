//! Multilingual Wiktionary/Kaikki importer for EasyEnglish.
//!
//! Reads a local Kaikki English JSONL gzip file and emits language-specific
//! English → target and target → English datasets under `Dict/`.
//!
//! Usage:
//!
//! ```text
//! cargo run -p ee-core --bin multilingual_generator -- <kaikki-english.jsonl.gz>
//! ```
//!
//! The generated assets are derived from Wiktionary/Kaikki data. Keep the
//! corresponding attribution file with the generated dictionary files.

use flate2::read::GzDecoder;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use ee_core::{Definition, RecordModel, SerializableRecord, WordCn, WordEn};

const MAX_REVERSE_ENGLISH: usize = 10;
const MAX_MAJOR_TERMS: usize = 8;
const MAX_TARGET_TERM_CHARS: usize = 60;

#[derive(Clone, Copy)]
struct LanguageSpec {
    id: &'static str,
    codes: &'static [&'static str],
    english_base: &'static str,
    reverse_base: &'static str,
    traditional_hk: bool,
}

const LANGUAGES: &[LanguageSpec] = &[
    LanguageSpec {
        id: "hk",
        codes: &["yue", "zh", "cmn"],
        english_base: "word_en_hk_v1",
        reverse_base: "word_hk_v1",
        traditional_hk: true,
    },
    LanguageSpec {
        id: "es",
        codes: &["es"],
        english_base: "word_en_es_v1",
        reverse_base: "word_es_v1",
        traditional_hk: false,
    },
    LanguageSpec {
        id: "ja",
        codes: &["ja"],
        english_base: "word_en_ja_v1",
        reverse_base: "word_ja_v1",
        traditional_hk: false,
    },
    LanguageSpec {
        id: "ko",
        codes: &["ko"],
        english_base: "word_en_ko_v1",
        reverse_base: "word_ko_v1",
        traditional_hk: false,
    },
    LanguageSpec {
        id: "pt",
        codes: &["pt"],
        english_base: "word_en_pt_v1",
        reverse_base: "word_pt_v1",
        traditional_hk: false,
    },
    LanguageSpec {
        id: "id",
        codes: &["id"],
        english_base: "word_en_id_v1",
        reverse_base: "word_id_v1",
        traditional_hk: false,
    },
    LanguageSpec {
        id: "ar",
        codes: &["ar"],
        english_base: "word_en_ar_v1",
        reverse_base: "word_ar_v1",
        traditional_hk: false,
    },
    LanguageSpec {
        id: "vi",
        codes: &["vi"],
        english_base: "word_en_vi_v1",
        reverse_base: "word_vi_v1",
        traditional_hk: false,
    },
    LanguageSpec {
        id: "hi",
        codes: &["hi"],
        english_base: "word_en_hi_v1",
        reverse_base: "word_hi_v1",
        traditional_hk: false,
    },
    LanguageSpec {
        id: "fr",
        codes: &["fr"],
        english_base: "word_en_fr_v1",
        reverse_base: "word_fr_v1",
        traditional_hk: false,
    },
];

#[derive(Default)]
struct LanguageData {
    english: BTreeMap<String, EnglishAccumulator>,
    reverse: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Default)]
struct EnglishAccumulator {
    pos_to_terms: BTreeMap<String, BTreeSet<String>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Dict/kaikki.org-dictionary-English.jsonl.gz".to_string());
    let source_path = Path::new(&source);
    if !source_path.exists() {
        return Err(format!(
            "Kaikki English JSONL gzip not found at {:?}. Download \
             https://kaikki.org/dictionary/English/kaikki.org-dictionary-English.jsonl.gz \
             and pass its path as the first argument.",
            source_path
        )
        .into());
    }

    let mut data: HashMap<&'static str, LanguageData> = LANGUAGES
        .iter()
        .map(|lang| (lang.id, LanguageData::default()))
        .collect();

    read_kaikki(source_path, &mut data)?;

    let out_dir = Path::new("Dict");
    for lang in LANGUAGES {
        let lang_data = data
            .get(lang.id)
            .ok_or_else(|| format!("missing accumulator for {}", lang.id))?;
        write_english_dataset(
            &lang_data.english,
            &out_dir.join(lang.english_base),
            &out_dir.join(format!("{}.sqlite", lang.english_base)),
        )?;
        write_reverse_dataset(
            &lang_data.reverse,
            &out_dir.join(lang.reverse_base),
            &out_dir.join(format!("{}.sqlite", lang.reverse_base)),
        )?;
    }
    write_attribution(out_dir)?;
    Ok(())
}

fn read_kaikki(
    source_path: &Path,
    data: &mut HashMap<&'static str, LanguageData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(source_path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    for (line_no, line) in reader.lines().enumerate() {
        let line = line?;
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Skipping malformed JSON line {}: {}", line_no + 1, err);
                continue;
            }
        };

        let word = match value.get("word").and_then(Value::as_str) {
            Some(word) if is_clean_english_word(word) => word.to_lowercase(),
            _ => continue,
        };
        let pos = value
            .get("pos")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let mut seen_for_entry = HashSet::new();
        collect_translations(&value, &word, &pos, data, &mut seen_for_entry);
    }

    Ok(())
}

fn collect_translations(
    value: &Value,
    english: &str,
    pos: &str,
    data: &mut HashMap<&'static str, LanguageData>,
    seen_for_entry: &mut HashSet<(String, String)>,
) {
    if let Some(translations) = value.get("translations").and_then(Value::as_array) {
        collect_translation_array(translations, english, pos, data, seen_for_entry);
    }

    if let Some(senses) = value.get("senses").and_then(Value::as_array) {
        for sense in senses {
            if let Some(translations) = sense.get("translations").and_then(Value::as_array) {
                let sense_pos = sense.get("pos").and_then(Value::as_str).unwrap_or(pos);
                collect_translation_array(translations, english, sense_pos, data, seen_for_entry);
            }
        }
    }
}

fn collect_translation_array(
    translations: &[Value],
    english: &str,
    pos: &str,
    data: &mut HashMap<&'static str, LanguageData>,
    seen_for_entry: &mut HashSet<(String, String)>,
) {
    for tr in translations {
        let code = tr
            .get("lang_code")
            .or_else(|| tr.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let Some(lang) = LANGUAGES.iter().find(|lang| lang.codes.contains(&code)) else {
            continue;
        };
        let Some(lang_data) = data.get_mut(lang.id) else {
            continue;
        };

        let Some(term) = tr.get("word").and_then(Value::as_str) else {
            continue;
        };
        let Some(term) = normalize_target_term(term, lang.traditional_hk) else {
            continue;
        };
        if !is_valid_target_term(&term) {
            continue;
        }

        if !seen_for_entry.insert((code.to_string(), term.clone())) {
            continue;
        }

        lang_data
            .english
            .entry(english.to_string())
            .or_default()
            .pos_to_terms
            .entry(pos.to_string())
            .or_default()
            .insert(term.clone());
        lang_data
            .reverse
            .entry(term)
            .or_default()
            .insert(english.to_string());
    }
}

fn write_english_dataset(
    entries: &BTreeMap<String, EnglishAccumulator>,
    list_path: &Path,
    db_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    write_word_list(entries.keys(), list_path)?;

    recreate_db(db_path)?;
    let mut conn = Connection::open(db_path)?;
    let tx = conn.transaction()?;
    {
        let mut stmt =
            tx.prepare("INSERT OR REPLACE INTO storage_entries (key, value) VALUES (?, ?)")?;
        for (key, acc) in entries {
            let word = WordEn {
                word: key.clone(),
                major: Some(major_terms(acc).join(", ")),
                pronunciation: None,
                definitions: Some(definitions(acc)),
                inflections: None,
                examples: None,
            };
            let serialized = RecordModel::WordEn(word).serialize()?;
            stmt.execute(params![key, serialized])?;
        }
    }
    tx.commit()?;
    println!("Wrote {} entries to {}", entries.len(), db_path.display());
    Ok(())
}

fn write_reverse_dataset(
    entries: &BTreeMap<String, BTreeSet<String>>,
    list_path: &Path,
    db_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    write_word_list(entries.keys(), list_path)?;

    recreate_db(db_path)?;
    let mut conn = Connection::open(db_path)?;
    let tx = conn.transaction()?;
    {
        let mut stmt =
            tx.prepare("INSERT OR REPLACE INTO storage_entries (key, value) VALUES (?, ?)")?;
        for (key, english_words) in entries {
            let english = english_words
                .iter()
                .take(MAX_REVERSE_ENGLISH)
                .cloned()
                .collect::<Vec<_>>();
            let serialized = RecordModel::WordCn(WordCn {
                word: key.clone(),
                english,
            })
            .serialize()?;
            stmt.execute(params![key, serialized])?;
        }
    }
    tx.commit()?;
    println!("Wrote {} entries to {}", entries.len(), db_path.display());
    Ok(())
}

fn recreate_db(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let conn = Connection::open(path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS storage_entries (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn write_word_list<'a>(
    keys: impl Iterator<Item = &'a String>,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(path)?;
    let mut count = 0usize;
    for key in keys {
        writeln!(file, "{}", key)?;
        count += 1;
    }
    println!("Wrote {} words to {}", count, path.display());
    Ok(())
}

fn major_terms(acc: &EnglishAccumulator) -> Vec<String> {
    let mut terms = Vec::new();
    for term_set in acc.pos_to_terms.values() {
        for term in term_set {
            terms.push(term.clone());
            if terms.len() >= MAX_MAJOR_TERMS {
                return terms;
            }
        }
    }
    terms
}

fn definitions(acc: &EnglishAccumulator) -> Vec<Definition> {
    acc.pos_to_terms
        .iter()
        .map(|(pos, terms)| Definition {
            pos: pos.clone(),
            meanings: terms.iter().cloned().collect(),
        })
        .collect()
}

fn is_clean_english_word(word: &str) -> bool {
    let n = word.chars().count();
    ((2..=32).contains(&n) && word.chars().all(|c| c.is_ascii_alphabetic()))
        || word == "a"
        || word == "i"
}

fn normalize_target_term(term: &str, traditional_hk: bool) -> Option<String> {
    if traditional_hk {
        normalize_traditional_hk_term(term)
    } else {
        Some(term.trim().to_lowercase())
    }
}

fn normalize_traditional_hk_term(term: &str) -> Option<String> {
    let mut candidates = Vec::new();
    for piece in term.split('/') {
        let candidate = piece.trim();
        if candidate.is_empty() {
            continue;
        }
        candidates.push(candidate);
    }
    if candidates.is_empty() {
        return None;
    }

    for candidate in &candidates {
        if contains_traditional_specific_char(candidate) {
            return Some((*candidate).to_string());
        }
    }
    candidates.first().map(|candidate| (*candidate).to_string())
}

fn contains_traditional_specific_char(value: &str) -> bool {
    const TRADITIONAL_HINTS: &str = "萬與專業東絲兩嚴個豐臨為麗舉義烏樂喬鄉書買亂爭於雲亞產畝親億僅從倉儀們價眾優會傘偉傳傷倫偽體餘傭債傾償儲兒兌黨蘭關興養獸內岡冊寫軍農馮決況凍淨涼減鳳憑凱擊鑿劃劉創刪劊劍劑勁動務勝勞勢勳勵勸區醫華協單賣盧衛卻廠廳歷厲壓厭縣參雙發變葉號嘆後嚇嗎聽啟員響問啞嘔喚嗇嘩嘮嘯嘰噴噸嚐嚮團園國圍圖圓聖場壞塊堅壇壩墳聲壺處備複夠頭誇奪奮奧婦媽嫵妝娛媧嫻嬰學孫寧寶實寵審寬對尋導將爾塵嘗堯層屬島峽幣帥師帳帶幫幹庫廁廂廚廟廢廣慶張強彈彌彎彙彥徑徵徹憶懷態憂慮慘懲應懇惡惱愛願憑憤憫憲懶懸懼戀戰戲戶拋挾捨掃揚換揮損搖搶撈撐撓撥撫撲撿擁擇擔據擠擬擴擺擾攔攜攝攤攪敗敵數斂斃斬斷無舊時曠曆曉暈暉暢暫曖會朧棄棗棟棧棲樣樁標樸樹橋機橫檔檢檯櫃櫻權歡歐殼毀氣漢湯溝淚潔淺漿澆濁測濟瀏渾濃濤塗濫瀉灣濕滄滅滌滯滲滾滿漁漣漬潰潤潑澤濱濺濾瀟灑灘災烏煉煙煥煩熱燈燒燙營燦燭爐爛牆獨狹獅獎獄獵獻現瑣瑤環瓊產畢畫異當疇疊瘋瘡療癒發盜盞盡監盤眾睜矚礎礙禮稅種稱穀積穩窩窮竄竅競筆筍節範築篩簡籃籌類糾紀約紅紋納紐純紙級紛紡細紳紹終組結絕絡給統絲經綠綱網綴維綜緊緒線緝緞締緣編緩緯練縣縫縮縱總績織繞繡繩繪繫繼續纏纖";
    value.chars().any(|c| TRADITIONAL_HINTS.contains(c))
}

fn is_valid_target_term(term: &str) -> bool {
    let n = term.chars().count();
    (1..=MAX_TARGET_TERM_CHARS).contains(&n)
        && !term.chars().any(char::is_control)
        && term.chars().any(|c| c.is_alphabetic())
}

fn write_attribution(out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = PathBuf::from(out_dir);
    path.push("WIKTIONARY-KAIKKI-LICENSE.txt");
    let mut file = File::create(path)?;
    file.write_all(
        b"EasyEnglish multilingual dictionary assets are derived from Wiktionary data extracted by Kaikki/Wiktextract.\n\nSource: https://kaikki.org/dictionary/English/\nRaw download: https://kaikki.org/dictionary/English/kaikki.org-dictionary-English.jsonl.gz\nUpstream license: Creative Commons Attribution-ShareAlike 3.0 Unported (CC BY-SA 3.0)\nLicense URL: https://creativecommons.org/licenses/by-sa/3.0/\n\nPlease cite: Tatu Ylonen, Wiktextract: Wiktionary as Machine-Readable Structured Data, Proceedings of LREC 2022.\n",
    )?;
    Ok(())
}
