use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ChunkConfig {
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
    #[serde(default = "default_overlap")]
    pub overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        ChunkConfig {
            max_chars: default_max_chars(),
            overlap: default_overlap(),
        }
    }
}

fn default_max_chars() -> usize { 512 }
fn default_overlap() -> usize { 50 }

pub struct Chunk {
    pub text: String,
    pub index: usize,
}

pub fn chunk_text(text: &str, config: &ChunkConfig) -> Vec<Chunk> {
    if text.trim().is_empty() {
        return vec![];
    }

    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut raw_chunks: Vec<String> = Vec::new();

    for para in &paragraphs {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if para.len() <= config.max_chars {
            raw_chunks.push(para.to_string());
        } else {
            raw_chunks.extend(split_by_sentences(para, config.max_chars));
        }
    }

    if raw_chunks.is_empty() {
        raw_chunks.push(text.to_string());
    }

    merge_small_chunks(&raw_chunks, config.max_chars, config.overlap)
        .into_iter()
        .enumerate()
        .map(|(i, text)| Chunk { text, index: i })
        .collect()
}

fn split_by_sentences(text: &str, max_chars: usize) -> Vec<String> {
    let sentence_endings: &[char] = &['。', '！', '？', '；', '.', '!', '?', ';', '\n'];
    let mut chunks = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();

    while start < total {
        let remaining = total - start;
        if remaining <= max_chars {
            chunks.push(chars[start..].iter().collect());
            break;
        }

        let mut end = start + max_chars;
        let mut found = false;
        for i in (start..end).rev() {
            if sentence_endings.contains(&chars[i]) {
                end = i + 1;
                found = true;
                break;
            }
        }
        if !found {
            end = start + max_chars;
        }

        chunks.push(chars[start..end].iter().collect());
        start = end;
    }

    chunks
}

fn merge_small_chunks(chunks: &[String], max_chars: usize, overlap: usize) -> Vec<String> {
    if chunks.is_empty() {
        return vec![];
    }

    let mut result = Vec::new();
    let mut current = chunks[0].clone();

    for i in 1..chunks.len() {
        let next = &chunks[i];
        if current.len() + next.len() + 1 <= max_chars {
            current.push('\n');
            current.push_str(next);
        } else {
            if overlap > 0 && !current.is_empty() {
                let overlap_text = take_suffix(&current, overlap);
                result.push(current);
                current = overlap_text;
                current.push('\n');
                current.push_str(next);
            } else {
                result.push(current);
                current = next.clone();
            }
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

fn take_suffix(text: &str, n: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= n {
        text.to_string()
    } else {
        chars[chars.len() - n..].iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text() {
        let chunks = chunk_text("Hello world", &ChunkConfig::default());
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello world");
    }

    #[test]
    fn test_empty_text() {
        let chunks = chunk_text("", &ChunkConfig::default());
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_paragraph_split() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let config = ChunkConfig { max_chars: 100, overlap: 0 };
        let chunks = chunk_text(text, &config);
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn test_long_paragraph_split() {
        let long_text = "A".repeat(600);
        let config = ChunkConfig { max_chars: 200, overlap: 0 };
        let chunks = chunk_text(&long_text, &config);
        assert!(chunks.len() >= 3);
        for chunk in &chunks {
            assert!(chunk.text.len() <= 200);
        }
    }
}
