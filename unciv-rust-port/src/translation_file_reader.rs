use std::collections::{HashMap, LinkedHashMap};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub struct TranslationFileReader;

impl TranslationFileReader {
    pub const PERCENTAGES_FILE_LOCATION: &'static str = "jsons/translations/completionPercentages.properties";
    const CHARSET: &'static str = "UTF-8";

    pub fn read<P: AsRef<Path>>(file_path: P) -> io::Result<LinkedHashMap<String, String>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut translations = LinkedHashMap::new();

        for line in reader.lines() {
            let line = line?;
            if !line.contains(" = ") {
                continue;
            }

            let parts: Vec<&str> = line.split(" = ").collect();
            if parts.len() != 2 || parts[1].is_empty() {
                continue;
            }

            let key = parts[0].replace("\\n", "\n");
            let value = parts[1].replace("\\n", "\n");
            translations.insert(key, value);
        }

        Ok(translations)
    }

    pub fn read_language_percentages() -> HashMap<String, i32> {
        let mut percentages = HashMap::new();

        if let Ok(file) = File::open(Self::PERCENTAGES_FILE_LOCATION) {
            let reader = BufReader::new(file);

            for line in reader.lines() {
                if let Ok(line) = line {
                    let parts: Vec<&str> = line.split(" = ").collect();
                    if parts.len() == 2 {
                        if let Ok(percentage) = parts[1].parse::<i32>() {
                            percentages.insert(parts[0].to_string(), percentage);
                        }
                    }
                }
            }
        }

        percentages
    }
}