// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! STEP file tokenizer using nom combinators
//!
//! Parses STEP/IFC entity definitions into tokens.

use ifc_lite_model::{AttributeValue, DecodedEntity, EntityId, IfcType};
use nom::{
    branch::alt,
    bytes::complete::{take_while, take_while1},
    character::complete::{char, multispace0},
    combinator::{opt, recognize},
    multi::separated_list0,
    sequence::{delimited, pair},
    IResult, Parser,
};

/// Raw token from STEP file (before conversion to AttributeValue)
#[derive(Clone, Debug, PartialEq)]
pub enum Token<'a> {
    /// Entity reference (#123)
    EntityRef(u32),
    /// String value ('text')
    String(&'a str),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Enumeration (.VALUE.)
    Enum(&'a str),
    /// List of tokens
    List(Vec<Token<'a>>),
    /// Typed value like IFCLABEL('text')
    TypedValue(&'a str, Vec<Token<'a>>),
    /// Null value ($)
    Null,
    /// Derived value (*)
    Derived,
}

impl<'a> Token<'a> {
    /// Convert token to owned AttributeValue
    pub fn to_attribute_value(&self) -> AttributeValue {
        match self {
            Token::EntityRef(id) => AttributeValue::EntityRef(EntityId(*id)),
            Token::String(s) => AttributeValue::String((*s).to_string()),
            Token::Integer(i) => AttributeValue::Integer(*i),
            Token::Float(f) => AttributeValue::Float(*f),
            Token::Enum(s) => AttributeValue::Enum((*s).to_string()),
            Token::List(items) => {
                AttributeValue::List(items.iter().map(|t| t.to_attribute_value()).collect())
            }
            Token::TypedValue(name, args) => AttributeValue::TypedValue(
                (*name).to_string(),
                args.iter().map(|t| t.to_attribute_value()).collect(),
            ),
            Token::Null => AttributeValue::Null,
            Token::Derived => AttributeValue::Derived,
        }
    }
}

// ============================================================================
// Parsing Primitives
// ============================================================================

/// Parse whitespace (including comments)
fn ws(input: &str) -> IResult<&str, ()> {
    let (input, _) = multispace0(input)?;
    Ok((input, ()))
}

/// Parse an entity reference (#123)
fn entity_ref(input: &str) -> IResult<&str, Token> {
    let (input, _) = char('#')(input)?;
    let (input, digits) = take_while1(|c: char| c.is_ascii_digit())(input)?;
    let id = digits.parse::<u32>().unwrap_or(0);
    Ok((input, Token::EntityRef(id)))
}

/// Parse a STEP string ('text' with '' for escaped quotes)
fn step_string(input: &str) -> IResult<&str, Token> {
    let (input, _) = char('\'')(input)?;

    // Find the end of the string, handling escaped quotes ('')
    let mut end = 0;
    let bytes = input.as_bytes();
    while end < bytes.len() {
        if bytes[end] == b'\'' {
            // Check for escaped quote
            if end + 1 < bytes.len() && bytes[end + 1] == b'\'' {
                end += 2;
                continue;
            }
            break;
        }
        end += 1;
    }

    let content = &input[..end];
    let remaining = &input[end + 1..]; // Skip closing quote

    Ok((remaining, Token::String(content)))
}

/// Parse a number (integer or float)
fn number(input: &str) -> IResult<&str, Token> {
    let (input, num_str) = recognize((
        opt(char('-')),
        take_while1(|c: char| c.is_ascii_digit()),
        opt(pair(char('.'), take_while(|c: char| c.is_ascii_digit()))),
        opt((
            alt((char('e'), char('E'))),
            opt(alt((char('+'), char('-')))),
            take_while1(|c: char| c.is_ascii_digit()),
        )),
    ))
    .parse(input)?;

    // Use lexical-core for fast parsing
    if num_str.contains('.') || num_str.contains('e') || num_str.contains('E') {
        let f: f64 = lexical_core::parse(num_str.as_bytes()).unwrap_or(0.0);
        Ok((input, Token::Float(f)))
    } else {
        let i: i64 = lexical_core::parse(num_str.as_bytes()).unwrap_or(0);
        Ok((input, Token::Integer(i)))
    }
}

/// Parse an enumeration (.VALUE.)
fn enumeration(input: &str) -> IResult<&str, Token> {
    let (input, _) = char('.')(input)?;
    let (input, name) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let (input, _) = char('.')(input)?;
    Ok((input, Token::Enum(name)))
}

/// Parse null ($)
fn null_value(input: &str) -> IResult<&str, Token> {
    let (input, _) = char('$')(input)?;
    Ok((input, Token::Null))
}

/// Parse derived (*)
fn derived_value(input: &str) -> IResult<&str, Token> {
    let (input, _) = char('*')(input)?;
    Ok((input, Token::Derived))
}

/// Parse a list of tokens
fn list(input: &str) -> IResult<&str, Token> {
    let (input, items) = delimited(
        pair(char('('), ws),
        separated_list0((ws, char(','), ws), token),
        pair(ws, char(')')),
    )
    .parse(input)?;
    Ok((input, Token::List(items)))
}

/// Parse a typed value like IFCLABEL('text')
fn typed_value(input: &str) -> IResult<&str, Token> {
    let (input, type_name) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let (input, _) = ws(input)?;
    let (input, args) = delimited(
        pair(char('('), ws),
        separated_list0((ws, char(','), ws), token),
        pair(ws, char(')')),
    )
    .parse(input)?;
    Ok((input, Token::TypedValue(type_name, args)))
}

/// Parse any token
fn token(input: &str) -> IResult<&str, Token> {
    alt((
        entity_ref,
        step_string,
        null_value,
        derived_value,
        enumeration,
        number,
        list,
        typed_value,
    ))
    .parse(input)
}

/// Parse entity attribute list
fn attribute_list(input: &str) -> IResult<&str, Vec<Token>> {
    delimited(
        pair(char('('), ws),
        separated_list0((ws, char(','), ws), token),
        pair(ws, char(')')),
    )
    .parse(input)
}

// ============================================================================
// Entity Parsing
// ============================================================================

/// Parse a complete entity definition
///
/// Format: `#123=IFCWALL(attr1,attr2,...);`
pub fn parse_entity(input: &str) -> Result<DecodedEntity, String> {
    // Skip leading whitespace
    let input = input.trim_start();

    // Parse entity ID
    let (input, _) = char::<&str, nom::error::Error<&str>>('#')
        .parse(input)
        .map_err(|_| "Expected # at start of entity")?;

    let (input, id_str) = take_while1::<_, &str, nom::error::Error<&str>>(|c: char| {
        c.is_ascii_digit()
    })
    .parse(input)
    .map_err(|_| "Expected entity ID")?;

    let id: u32 = id_str.parse().map_err(|_| "Invalid entity ID")?;

    // Skip =
    let (input, _) = (ws, char('='), ws)
        .parse(input)
        .map_err(|_: nom::Err<nom::error::Error<&str>>| "Expected = after entity ID")?;

    // Parse type name
    let (input, type_name) =
        take_while1::<_, &str, nom::error::Error<&str>>(|c: char| c.is_alphanumeric() || c == '_')
            .parse(input)
            .map_err(|_| "Expected type name")?;

    // Parse attributes
    let (input, _) = ws(input).unwrap_or((input, ()));

    let (_, tokens) =
        attribute_list(input).map_err(|e| format!("Failed to parse attributes: {:?}", e))?;

    // Convert tokens to attribute values
    let attributes: Vec<AttributeValue> = tokens.iter().map(|t| t.to_attribute_value()).collect();

    Ok(DecodedEntity {
        id: EntityId(id),
        ifc_type: IfcType::parse(type_name),
        attributes,
    })
}

/// Parse entity from raw bytes at given position
pub fn parse_entity_at(content: &str, start: usize, end: usize) -> Result<DecodedEntity, String> {
    let slice = &content[start..end];
    parse_entity(slice)
}

// ============================================================================
// Fast Path Parsers (for coordinate extraction)
// ============================================================================

/// Fast parse coordinate list from IfcCartesianPointList3D
/// Returns flattened [x0,y0,z0, x1,y1,z1, ...]
pub fn parse_coordinate_list_3d_fast(content: &str) -> Option<Vec<f64>> {
    // Find the coordinates list - typically attribute 0 after CoordList
    let start = content.find("((")?;
    let end = content.rfind("))")?;
    let list_content = &content[start + 1..end + 1];

    let mut coords = Vec::new();
    let mut current = list_content;

    while let Some(paren_start) = current.find('(') {
        let paren_end = current[paren_start..].find(')')? + paren_start;
        let point_str = &current[paren_start + 1..paren_end];

        // Parse x, y, z
        for num_str in point_str.split(',') {
            let num_str = num_str.trim();
            if !num_str.is_empty() {
                let val: f64 = lexical_core::parse(num_str.as_bytes()).ok()?;
                coords.push(val);
            }
        }

        current = &current[paren_end + 1..];
    }

    if coords.is_empty() {
        None
    } else {
        Some(coords)
    }
}

/// Fast parse index list from IfcTriangulatedFaceSet
/// Converts from 1-based IFC indices to 0-based
pub fn parse_index_list_fast(content: &str) -> Option<Vec<u32>> {
    let start = content.find("((")?;
    let end = content.rfind("))")?;
    let list_content = &content[start + 1..end + 1];

    let mut indices = Vec::new();
    let mut current = list_content;

    while let Some(paren_start) = current.find('(') {
        let paren_end = current[paren_start..].find(')')? + paren_start;
        let index_str = &current[paren_start + 1..paren_end];

        for num_str in index_str.split(',') {
            let num_str = num_str.trim();
            if !num_str.is_empty() {
                let val: u32 = lexical_core::parse(num_str.as_bytes()).ok()?;
                // Convert from 1-based to 0-based
                indices.push(val.saturating_sub(1));
            }
        }

        current = &current[paren_end + 1..];
    }

    if indices.is_empty() {
        None
    } else {
        Some(indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entity_ref() {
        let (remaining, token) = entity_ref("#123").unwrap();
        assert_eq!(remaining, "");
        assert_eq!(token, Token::EntityRef(123));
    }

    #[test]
    fn test_parse_string() {
        let (remaining, token) = step_string("'hello world'").unwrap();
        assert_eq!(remaining, "");
        assert_eq!(token, Token::String("hello world"));
    }

    #[test]
    fn test_parse_string_with_escaped_quote() {
        let (remaining, token) = step_string("'it''s a test'").unwrap();
        assert_eq!(remaining, "");
        assert_eq!(token, Token::String("it''s a test"));
    }

    #[test]
    fn test_parse_number_integer() {
        let (remaining, token) = number("42").unwrap();
        assert_eq!(remaining, "");
        assert_eq!(token, Token::Integer(42));
    }

    #[test]
    fn test_parse_number_float() {
        let (remaining, token) = number("3.14159").unwrap();
        assert_eq!(remaining, "");
        if let Token::Float(f) = token {
            assert!((f - 3.14159).abs() < 1e-10);
        } else {
            panic!("Expected float");
        }
    }

    #[test]
    fn test_parse_number_scientific() {
        let (remaining, token) = number("1.5E-3").unwrap();
        assert_eq!(remaining, "");
        if let Token::Float(f) = token {
            assert!((f - 0.0015).abs() < 1e-10);
        } else {
            panic!("Expected float");
        }
    }

    #[test]
    fn test_parse_enum() {
        let (remaining, token) = enumeration(".TRUE.").unwrap();
        assert_eq!(remaining, "");
        assert_eq!(token, Token::Enum("TRUE"));
    }

    #[test]
    fn test_parse_list() {
        let (remaining, token) = list("(1, 2, 3)").unwrap();
        assert_eq!(remaining, "");
        if let Token::List(items) = token {
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_entity() {
        let entity = parse_entity("#1=IFCWALL('abc',$,#2);").unwrap();
        assert_eq!(entity.id, EntityId(1));
        assert_eq!(entity.ifc_type, IfcType::IfcWall);
        assert_eq!(entity.attributes.len(), 3);
    }
}
