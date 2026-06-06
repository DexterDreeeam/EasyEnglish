//! Database and Word List Generator for EasyEnglish.
//! Processes raw_50k.txt, sorts lists of 5k, 10k, and 20k,
//! and writes SQLite databases with serialized JSON models.

use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

use ee_core::{
    Definition, Example, Inflections, Pronunciation, RecordModel, SerializableRecord, WordEn,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting database and word list generation...");

    let raw_path = Path::new("Dict/raw_50k.txt");
    if !raw_path.exists() {
        return Err(format!("Raw frequency file not found at: {:?}", raw_path).into());
    }

    let file = File::open(raw_path)?;
    let reader = io::BufReader::new(file);

    let mut words = Vec::new();
    let mut seen = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let word = parts[0].to_lowercase().trim().to_string();

        // Standard filter: must be pure lower ASCII a-z and length >= 2 (or 'a'/'i')
        if word.chars().all(|c| c.is_ascii_alphabetic())
            && (word.len() >= 2 || word == "a" || word == "i")
            && seen.insert(word.clone())
        {
            words.push(word);
        }

        if words.len() >= 20000 {
            break;
        }
    }

    println!("Total filtered unique words extracted: {}", words.len());

    // Generate v1 (5,000 words), v2 (10,000 words), and v3 (20,000 words)
    generate_dataset(&words, 5000, "Dict/word_list_v1", "Dict/word_en_v1.sqlite")?;
    generate_dataset(&words, 10000, "Dict/word_list_v2", "Dict/word_en_v2.sqlite")?;
    generate_dataset(&words, 20000, "Dict/word_list_v3", "Dict/word_en_v3.sqlite")?;

    println!("Database and word list generation completed successfully!");
    Ok(())
}

fn generate_dataset(
    words: &[String],
    limit: usize,
    text_path_str: &str,
    db_path_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let count = std::cmp::min(words.len(), limit);
    let mut subset = words[0..count].to_vec();
    subset.sort(); // Alphabetical sort as required

    // Save as sorted text file
    let mut text_file = File::create(text_path_str)?;
    for w in &subset {
        writeln!(text_file, "{}", w)?;
    }
    println!("Saved text list to: {}", text_path_str);

    // Create SQLite DB
    let db_path = Path::new(db_path_str);
    if db_path.exists() {
        std::fs::remove_file(db_path)?;
    }
    let mut conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS storage_entries (
            key        TEXT PRIMARY KEY,
            value      TEXT NOT NULL
        )",
        [],
    )?;

    // Perform database insertion in bulk transaction for 100x speedup
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare("INSERT INTO storage_entries (key, value) VALUES (?, ?)")?;

        for word_str in &subset {
            // Build rich WordEn structures
            let word_en = match word_str.as_str() {
                "apple" => WordEn {
                    word: "apple".to_string(),
                    major: Some("苹果".to_string()),
                    pronunciation: Some(Pronunciation {
                        ipa: "ˈæpəl".to_string(),
                        audio: Some("audio/apple.mp3".to_string()),
                        audio_url: Some("https://cdn.easyenglish.org/audio/apple.mp3".to_string()),
                    }),
                    definitions: Some(vec![Definition {
                        pos: "n.".to_string(),
                        meanings: vec!["苹果".to_string(), "苹果树".to_string()],
                    }]),
                    inflections: Some(Inflections {
                        plural: Some("apples".to_string()),
                        past_tense: None,
                        past_participle: None,
                        present_participle: None,
                        third_singular: None,
                    }),
                    examples: Some(vec![Example {
                        en: "Apple is a sweet fruit.".to_string(),
                        zh: "苹果是一种甜的水果。".to_string(),
                    }]),
                },
                "book" => WordEn {
                    word: "book".to_string(),
                    major: Some("书".to_string()),
                    pronunciation: Some(Pronunciation {
                        ipa: "bʊk".to_string(),
                        audio: Some("audio/book.mp3".to_string()),
                        audio_url: Some("https://cdn.easyenglish.org/audio/book.mp3".to_string()),
                    }),
                    definitions: Some(vec![
                        Definition {
                            pos: "n.".to_string(),
                            meanings: vec!["书".to_string(), "册子".to_string()],
                        },
                        Definition {
                            pos: "v.".to_string(),
                            meanings: vec!["预订".to_string(), "登记".to_string()],
                        },
                    ]),
                    inflections: Some(Inflections {
                        plural: Some("books".to_string()),
                        past_tense: Some("booked".to_string()),
                        past_participle: Some("booked".to_string()),
                        present_participle: Some("booking".to_string()),
                        third_singular: Some("books".to_string()),
                    }),
                    examples: Some(vec![Example {
                        en: "I love reading books.".to_string(),
                        zh: "我爱读书。".to_string(),
                    }]),
                },
                "apply" => WordEn {
                    word: "apply".to_string(),
                    major: Some("申请".to_string()),
                    pronunciation: Some(Pronunciation {
                        ipa: "əˈplaɪ".to_string(),
                        audio: Some("audio/apply.mp3".to_string()),
                        audio_url: Some("https://cdn.easyenglish.org/audio/apply.mp3".to_string()),
                    }),
                    definitions: Some(vec![Definition {
                        pos: "v.".to_string(),
                        meanings: vec!["申请".to_string(), "应用".to_string(), "适用".to_string()],
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
                        zh: "你可以线上申请。".to_string(),
                    }]),
                },
                // Fallback for general vocabulary
                _ => WordEn {
                    word: word_str.clone(),
                    major: None,
                    pronunciation: Some(Pronunciation {
                        ipa: format!("/{}/", word_str),
                        audio: None,
                        audio_url: None,
                    }),
                    definitions: Some(vec![Definition {
                        pos: "n.".to_string(),
                        meanings: vec![format!("[mock meaning of {}]", word_str)],
                    }]),
                    inflections: None,
                    examples: None,
                },
            };

            // Wrap in RecordModel and serialize
            let model = RecordModel::WordEn(word_en);
            let serialized = model.serialize()?;

            stmt.execute(params![word_str, serialized])?;
        }
    }
    tx.commit()?;
    println!(
        "Populated SQLite DB with {} entries at: {}",
        count, db_path_str
    );
    Ok(())
}
