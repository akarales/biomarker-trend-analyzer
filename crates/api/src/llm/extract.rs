//! Incremental extraction of string fields from a *partial* JSON document.
//!
//! A schema-constrained model streams one JSON object in arbitrary
//! fragments (`{"clinician":{"explan` … `ation":"Fluco` …). The extractor
//! is fed those fragments and emits, as soon as characters arrive, the
//! decoded text of every string VALUE with its dotted path
//! (`clinician.explanation`). Escapes (incl. `\uXXXX` and surrogate pairs)
//! may be split across fragments. It never validates — the complete text is
//! parsed strictly with the schema at the end; this only drives progressive
//! rendering.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    pub field: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    Key,
    Colon,
    Value,
    CommaOrEnd,
}

#[derive(Debug, Default)]
pub struct FieldExtractor {
    /// object keys from the root to the current container
    path: Vec<String>,
    /// per open container: what comes next (arrays expect values)
    expect: Vec<Expect>,
    in_key: bool,
    in_value: bool,
    key: String,
    /// pending escape: "\\" seen, or "\\u" + up to 4 hex digits
    escape: Option<String>,
    /// high surrogate waiting for its low half
    high_surrogate: Option<u16>,
    /// output buffered during one `feed`
    out: Vec<Delta>,
}

impl FieldExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed the next fragment; returns the text deltas it completed
    /// (adjacent text of the same field is merged).
    pub fn feed(&mut self, fragment: &str) -> Vec<Delta> {
        for c in fragment.chars() {
            self.step(c);
        }
        std::mem::take(&mut self.out)
    }

    fn field(&self) -> String {
        let mut parts = self.path.clone();
        parts.push(self.key.clone());
        parts.join(".")
    }

    fn emit(&mut self, c: char) {
        let field = self.field();
        match self.out.last_mut() {
            Some(last) if last.field == field => last.text.push(c),
            _ => self.out.push(Delta {
                field,
                text: c.to_string(),
            }),
        }
    }

    fn set_expect(&mut self, e: Expect) {
        if let Some(top) = self.expect.last_mut() {
            *top = e;
        }
    }

    fn step(&mut self, c: char) {
        if self.in_key || self.in_value {
            return self.string_char(c);
        }
        match c {
            '{' => {
                // the key that opened this object becomes a path segment
                if !self.expect.is_empty() {
                    self.path.push(std::mem::take(&mut self.key));
                }
                self.expect.push(Expect::Key);
            }
            '[' => self.expect.push(Expect::Value),
            '}' | ']' => {
                self.expect.pop();
                if c == '}' && !self.expect.is_empty() {
                    self.path.pop();
                }
                self.set_expect(Expect::CommaOrEnd);
            }
            '"' => match self.expect.last() {
                Some(Expect::Key) => {
                    self.in_key = true;
                    self.key.clear();
                }
                Some(Expect::Value) => self.in_value = true,
                _ => {}
            },
            ':' => self.set_expect(Expect::Value),
            ',' => {
                let in_object = self.expect.len() > self.path.len();
                self.set_expect(if in_object {
                    Expect::Key
                } else {
                    Expect::Value
                });
            }
            _ => {
                // numbers / true / false / null: no text to stream
                if !c.is_whitespace() && self.expect.last() == Some(&Expect::Value) {
                    self.set_expect(Expect::CommaOrEnd);
                }
            }
        }
    }

    fn string_char(&mut self, c: char) {
        if let Some(esc) = self.escape.as_mut() {
            esc.push(c);
            let done = match esc.as_str() {
                "\\u" => false,
                e if e.starts_with("\\u") => e.len() == 6,
                _ => true,
            };
            if done {
                let esc = self.escape.take().unwrap_or_default();
                if let Some(decoded) = self.decode_escape(&esc) {
                    self.push_string_char(decoded);
                }
            }
            return;
        }
        match c {
            '\\' => self.escape = Some("\\".to_string()),
            '"' => {
                if self.in_key {
                    self.in_key = false;
                    self.set_expect(Expect::Colon);
                } else {
                    self.in_value = false;
                    self.set_expect(Expect::CommaOrEnd);
                }
            }
            _ => self.push_string_char(c),
        }
    }

    fn decode_escape(&mut self, esc: &str) -> Option<char> {
        let simple = match esc {
            "\\n" => Some('\n'),
            "\\t" => Some('\t'),
            "\\r" => Some('\r'),
            "\\b" => Some('\u{8}'),
            "\\f" => Some('\u{c}'),
            "\\\"" => Some('"'),
            "\\\\" => Some('\\'),
            "\\/" => Some('/'),
            _ => None,
        };
        if simple.is_some() {
            return simple;
        }
        let unit = u16::from_str_radix(esc.get(2..)?, 16).ok()?;
        match (self.high_surrogate.take(), unit) {
            (None, 0xD800..=0xDBFF) => {
                self.high_surrogate = Some(unit);
                None
            }
            (Some(high), 0xDC00..=0xDFFF) => char::from_u32(
                0x10000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(unit) - 0xDC00),
            ),
            (_, unit) => char::from_u32(u32::from(unit)),
        }
    }

    fn push_string_char(&mut self, c: char) {
        if self.in_key {
            self.key.push(c);
        } else {
            self.emit(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    const DOC: &str = r#"{"clinician": {"explanation": "Fluconazole \"inhibits\" CYP2C9\nso warfarin rises — monitor INR \u00b1 1 \ud83d\udc8a",
      "severity": "moderate", "mechanism": "CYP2C9 inhibition", "recommendation": "Check INR in 3\u20135 days"},
      "patient": {"explanation": "Your blood thinner may get stronger.", "severity": "moderate",
      "mechanism": "One medicine slows how the body clears the other.", "recommendation": "Ask about an INR test \\ a/b"}}"#;

    fn collect(chunks: impl IntoIterator<Item = String>) -> BTreeMap<String, String> {
        let mut x = FieldExtractor::new();
        let mut out: BTreeMap<String, String> = BTreeMap::new();
        for chunk in chunks {
            for d in x.feed(&chunk) {
                out.entry(d.field).or_default().push_str(&d.text);
            }
        }
        out
    }

    fn expected() -> BTreeMap<String, String> {
        let v: serde_json::Value = serde_json::from_str(DOC).expect("valid doc");
        let mut out = BTreeMap::new();
        for (section, fields) in v.as_object().expect("object") {
            for (k, val) in fields.as_object().expect("object") {
                out.insert(
                    format!("{section}.{k}"),
                    val.as_str().expect("string").to_string(),
                );
            }
        }
        out
    }

    #[test]
    fn whole_document_yields_every_field_decoded() {
        assert_eq!(collect([DOC.to_string()]), expected());
    }

    #[test]
    fn any_chunking_yields_the_same_fields() {
        let chars: Vec<char> = DOC.chars().collect();
        // deterministic pseudo-random chunk sizes (xorshift), many seeds,
        // plus the extremes: one char at a time and every split point
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..300 {
            let mut chunks = Vec::new();
            let mut i = 0;
            while i < chars.len() {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let n = 1 + (seed % 9) as usize;
                chunks.push(
                    chars[i..(i + n).min(chars.len())]
                        .iter()
                        .collect::<String>(),
                );
                i += n;
            }
            assert_eq!(collect(chunks), expected());
        }
        assert_eq!(collect(chars.iter().map(|c| c.to_string())), expected());
        for split in 0..chars.len() {
            let (a, b) = chars.split_at(split);
            assert_eq!(
                collect([a.iter().collect(), b.iter().collect()]),
                expected(),
                "split at {split}"
            );
        }
    }

    #[test]
    fn single_section_documents_use_plain_field_names() {
        let doc = r#"{"explanation":"e","severity":"mild","mechanism":"m","recommendation":"r"}"#;
        let got = collect([doc.to_string()]);
        assert_eq!(got.get("explanation").map(String::as_str), Some("e"));
        assert_eq!(got.len(), 4);
    }

    #[test]
    fn deltas_arrive_before_the_value_is_complete() {
        let mut x = FieldExtractor::new();
        assert!(x.feed(r#"{"clinician":{"expl"#).is_empty());
        let d = x.feed(r#"anation":"Fluco"#);
        assert_eq!(
            d,
            vec![Delta {
                field: "clinician.explanation".into(),
                text: "Fluco".into()
            }]
        );
    }
}
