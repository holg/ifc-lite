// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Fast entity scanner using SIMD-accelerated byte searching
//!
//! Scans IFC files to discover entities without full parsing.

use memchr::memchr;
use rustc_hash::FxHashMap;

/// Entity index mapping ID to byte offsets
pub type EntityIndex = FxHashMap<u32, (usize, usize)>;

/// Fast entity scanner for IFC files
///
/// Uses memchr for SIMD-accelerated scanning to quickly find entity
/// boundaries without full parsing.
pub struct EntityScanner<'a> {
    content: &'a str,
    pos: usize,
}

impl<'a> EntityScanner<'a> {
    /// Create a new scanner for the given content
    pub fn new(content: &'a str) -> Self {
        // Skip header section (find DATA; line)
        let pos = content
            .find("DATA;")
            .map(|p| p + 5)
            .unwrap_or(0);

        Self { content, pos }
    }

    /// Scan to find the next entity
    ///
    /// Returns (id, type_name, start_byte, end_byte)
    pub fn next_entity(&mut self) -> Option<(u32, &'a str, usize, usize)> {
        let bytes = self.content.as_bytes();

        // Find next # (entity start)
        while self.pos < bytes.len() {
            // Use memchr for fast # search
            let hash_pos = memchr(b'#', &bytes[self.pos..])?;
            self.pos += hash_pos;

            // Check if this is an entity definition (not a reference in attributes)
            // Entity definitions are at the start of a line (after whitespace/newline)
            let is_entity_start = self.pos == 0
                || bytes[self.pos - 1] == b'\n'
                || bytes[self.pos - 1] == b'\r'
                || bytes[self.pos - 1] == b';';

            if !is_entity_start {
                self.pos += 1;
                continue;
            }

            let start = self.pos;

            // Parse entity ID
            self.pos += 1; // Skip #
            let id_start = self.pos;

            while self.pos < bytes.len() && bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }

            if self.pos == id_start {
                // No digits found
                continue;
            }

            let id: u32 = self.content[id_start..self.pos].parse().ok()?;

            // Skip whitespace and =
            while self.pos < bytes.len() && (bytes[self.pos] == b' ' || bytes[self.pos] == b'\t') {
                self.pos += 1;
            }

            if self.pos >= bytes.len() || bytes[self.pos] != b'=' {
                continue;
            }
            self.pos += 1; // Skip =

            // Skip whitespace
            while self.pos < bytes.len() && (bytes[self.pos] == b' ' || bytes[self.pos] == b'\t') {
                self.pos += 1;
            }

            // Parse type name
            let type_start = self.pos;
            while self.pos < bytes.len()
                && (bytes[self.pos].is_ascii_alphanumeric() || bytes[self.pos] == b'_')
            {
                self.pos += 1;
            }

            if self.pos == type_start {
                continue;
            }

            let type_name = &self.content[type_start..self.pos];

            // Find end of entity (semicolon, but handle strings)
            let end = self.find_entity_end()?;

            return Some((id, type_name, start, end));
        }

        None
    }

    /// Find the end of an entity (semicolon), handling quoted strings
    fn find_entity_end(&mut self) -> Option<usize> {
        let bytes = self.content.as_bytes();
        let mut in_string = false;

        while self.pos < bytes.len() {
            match bytes[self.pos] {
                b'\'' => {
                    // Check for escaped quote ''
                    if in_string && self.pos + 1 < bytes.len() && bytes[self.pos + 1] == b'\'' {
                        self.pos += 2;
                        continue;
                    }
                    in_string = !in_string;
                }
                b';' if !in_string => {
                    self.pos += 1;
                    return Some(self.pos);
                }
                _ => {}
            }
            self.pos += 1;
        }

        None
    }

    /// Build an index of all entities (ID -> byte offsets)
    pub fn build_index(content: &'a str) -> EntityIndex {
        let mut scanner = Self::new(content);
        let mut index = FxHashMap::default();

        while let Some((id, _, start, end)) = scanner.next_entity() {
            index.insert(id, (start, end));
        }

        index
    }

    /// Count entities by type
    pub fn count_by_type(content: &'a str) -> FxHashMap<String, usize> {
        let mut scanner = Self::new(content);
        let mut counts: FxHashMap<String, usize> = FxHashMap::default();

        while let Some((_, type_name, _, _)) = scanner.next_entity() {
            *counts.entry(type_name.to_uppercase()).or_insert(0) += 1;
        }

        counts
    }

    /// Find all entities of a specific type
    pub fn find_by_type(content: &'a str, target_type: &str) -> Vec<(u32, usize, usize)> {
        let mut scanner = Self::new(content);
        let mut results = Vec::new();
        let target_upper = target_type.to_uppercase();

        while let Some((id, type_name, start, end)) = scanner.next_entity() {
            if type_name.eq_ignore_ascii_case(&target_upper) {
                results.push((id, start, end));
            }
        }

        results
    }

    /// Get total entity count
    pub fn entity_count(content: &'a str) -> usize {
        let mut scanner = Self::new(content);
        let mut count = 0;

        while scanner.next_entity().is_some() {
            count += 1;
        }

        count
    }
}

/// Parse the header section to extract metadata
pub fn parse_header(content: &str) -> HeaderInfo {
    let mut info = HeaderInfo::default();

    // Find HEADER section
    let header_start = content.find("HEADER;").unwrap_or(0);
    let header_end = content.find("ENDSEC;").unwrap_or(content.len());
    let header = &content[header_start..header_end];

    // Extract FILE_SCHEMA
    if let Some(schema_start) = header.find("FILE_SCHEMA") {
        if let Some(paren_start) = header[schema_start..].find("((") {
            let start = schema_start + paren_start + 2;
            if let Some(paren_end) = header[start..].find("))") {
                let schema_list = &header[start..start + paren_end];
                // Extract first schema (usually the only one)
                if let Some(quote_start) = schema_list.find('\'') {
                    if let Some(quote_end) = schema_list[quote_start + 1..].find('\'') {
                        info.schema_version =
                            schema_list[quote_start + 1..quote_start + 1 + quote_end].to_string();
                    }
                }
            }
        }
    }

    // Extract FILE_NAME
    if let Some(name_start) = header.find("FILE_NAME") {
        // FILE_NAME(name, timestamp, author, organization, preprocessor, originating_system, authorization)
        if let Some(paren_start) = header[name_start..].find('(') {
            let start = name_start + paren_start + 1;
            // Parse first argument (file name)
            if let Some((file_name, rest)) = parse_header_string(&header[start..]) {
                info.file_name = Some(file_name);

                // Parse timestamp
                if let Some(comma) = rest.find(',') {
                    if let Some((timestamp, rest2)) = parse_header_string(&rest[comma + 1..]) {
                        info.timestamp = Some(timestamp);

                        // Parse author (list)
                        if let Some(comma2) = rest2.find(',') {
                            if let Some((author, rest3)) = parse_header_list(&rest2[comma2 + 1..]) {
                                info.author = author.first().cloned();

                                // Parse organization (list)
                                if let Some(comma3) = rest3.find(',') {
                                    if let Some((org, rest4)) =
                                        parse_header_list(&rest3[comma3 + 1..])
                                    {
                                        info.organization = org.first().cloned();

                                        // Parse preprocessor_version
                                        if let Some(comma4) = rest4.find(',') {
                                            if let Some((preproc, rest5)) =
                                                parse_header_string(&rest4[comma4 + 1..])
                                            {
                                                info.preprocessor_version = Some(preproc);

                                                // Parse originating_system
                                                if let Some(comma5) = rest5.find(',') {
                                                    if let Some((orig_sys, _)) =
                                                        parse_header_string(&rest5[comma5 + 1..])
                                                    {
                                                        info.originating_system = Some(orig_sys);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    info
}

/// Parse a string from header ('value')
fn parse_header_string(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    if !s.starts_with('\'') {
        // Check for empty value
        if s.starts_with('$') {
            return Some((String::new(), &s[1..]));
        }
        return None;
    }

    let mut end = 1;
    let bytes = s.as_bytes();
    while end < bytes.len() {
        if bytes[end] == b'\'' {
            if end + 1 < bytes.len() && bytes[end + 1] == b'\'' {
                end += 2;
                continue;
            }
            break;
        }
        end += 1;
    }

    let value = s[1..end].replace("''", "'");
    Some((value, &s[end + 1..]))
}

/// Parse a list from header (('value1', 'value2'))
fn parse_header_list(s: &str) -> Option<(Vec<String>, &str)> {
    let s = s.trim_start();
    if !s.starts_with('(') {
        return Some((Vec::new(), s));
    }

    let mut items = Vec::new();
    let mut current = &s[1..]; // Skip opening paren

    loop {
        current = current.trim_start();
        if current.starts_with(')') {
            return Some((items, &current[1..]));
        }

        if let Some((item, rest)) = parse_header_string(current) {
            if !item.is_empty() {
                items.push(item);
            }
            current = rest.trim_start();
            if current.starts_with(',') {
                current = &current[1..];
            }
        } else {
            // Skip unknown content
            if let Some(pos) = current.find(|c| c == ',' || c == ')') {
                current = &current[pos..];
                if current.starts_with(',') {
                    current = &current[1..];
                }
            } else {
                break;
            }
        }
    }

    Some((items, current))
}

/// Header information extracted from IFC file
#[derive(Clone, Debug, Default)]
pub struct HeaderInfo {
    pub schema_version: String,
    pub file_name: Option<String>,
    pub timestamp: Option<String>,
    pub author: Option<String>,
    pub organization: Option<String>,
    pub preprocessor_version: Option<String>,
    pub originating_system: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_IFC: &str = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('test.ifc','2024-01-01T00:00:00',('Author'),('Org'),'Preprocessor','App','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCPROJECT('guid',$,'Project',$,$,$,$,$,#2);
#2=IFCUNITASSIGNMENT((#3));
#3=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#4=IFCWALL('guid',$,'Wall 1',$,$,#5,#6,$);
ENDSEC;
END-ISO-10303-21;
"#;

    #[test]
    fn test_scanner_finds_entities() {
        let mut scanner = EntityScanner::new(TEST_IFC);
        let mut entities = Vec::new();

        while let Some((id, type_name, _, _)) = scanner.next_entity() {
            entities.push((id, type_name.to_string()));
        }

        assert_eq!(entities.len(), 4);
        assert_eq!(entities[0], (1, "IFCPROJECT".to_string()));
        assert_eq!(entities[3], (4, "IFCWALL".to_string()));
    }

    #[test]
    fn test_build_index() {
        let index = EntityScanner::build_index(TEST_IFC);
        assert_eq!(index.len(), 4);
        assert!(index.contains_key(&1));
        assert!(index.contains_key(&4));
    }

    #[test]
    fn test_count_by_type() {
        let counts = EntityScanner::count_by_type(TEST_IFC);
        assert_eq!(counts.get("IFCPROJECT"), Some(&1));
        assert_eq!(counts.get("IFCWALL"), Some(&1));
    }

    #[test]
    fn test_parse_header() {
        let info = parse_header(TEST_IFC);
        assert_eq!(info.schema_version, "IFC2X3");
        assert_eq!(info.file_name, Some("test.ifc".to_string()));
    }
}
