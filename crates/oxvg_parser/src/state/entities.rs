use std::char::from_u32;

use crate::{
    file_reader::SAXState,
    syntactic_constructs::{name, reference},
};

use super::{
    attributes::{AttributeValueQuoted, AttributeValueUnquoted},
    text::Text,
    State, ID,
};

/// &`amp`; and such
#[derive(Clone)]
pub struct TextEntity;
/// <foo bar=&`quot`;
#[derive(Clone)]
pub struct AttributeValueEntityUnquoted;
/// <foo bar="&`quot`;"
#[derive(Clone)]
pub struct AttributeValueEntityQuoted;

#[derive(Clone)]
enum Entity {
    Text(Box<TextEntity>),
    AttributeValueUnquoted(Box<AttributeValueEntityUnquoted>),
    AttributeValueQuoted(Box<AttributeValueEntityQuoted>),
}

/// Handles the common transitions for all the entity types.
fn handle_entity(sax: &mut SAXState, char: char, current_state: &Entity) -> Box<dyn State> {
    let (return_current_state, return_state): (Box<dyn State>, Box<dyn State>) =
        match &current_state {
            Entity::Text(s) => (s.clone(), Box::new(Text)),
            Entity::AttributeValueUnquoted(s) => (s.clone(), Box::new(AttributeValueUnquoted)),
            Entity::AttributeValueQuoted(s) => (s.clone(), Box::new(AttributeValueQuoted)),
        };
    match char {
        ';' => {
            if let Ok(entity) = parse_entity(sax) {
                apply_entity(sax, current_state, &entity);
            } else if sax.get_options().strict {
                todo!("Handling tag entity not implemented");
            };
            sax.entity = String::new();
            return_state
        }
        c if sax.entity.is_empty() && (name::is_start(c) || c == '#') => {
            sax.entity.push(c);
            return_current_state
        }
        c if !sax.entity.is_empty() && (name::is(c) || c == '#') => {
            sax.entity.push(c);
            return_current_state
        }
        _ => {
            apply_entity(sax, current_state, &format!("&{};", sax.entity));
            sax.entity = String::new();
            return_state
        }
    }
}

/// Will parse `sax.entity` into it's representative string.
///
/// If the parse fails, the string contained in `Err` is the original entity
fn parse_entity(sax: &mut SAXState) -> Result<String, String> {
    // Lazily build the entity map
    if sax.entity_map.is_empty() {
        for &(key, value) in reference::XML_ENTITIES {
            sax.entity_map.insert(key.into(), value);
        }
        if !sax.get_options().strict {
            for &(key, value) in reference::ENTITIES {
                sax.entity_map.insert(key.into(), value);
            }
        }
    }

    if let Some(value) = sax.entity_map.get(&sax.entity) {
        return Ok((*value).into());
    }
    sax.entity = sax.entity.to_lowercase();
    if let Some(value) = sax.entity_map.get(&sax.entity) {
        return Ok((*value).into());
    }
    let num = match &sax.entity {
        e if e.starts_with("#x") => u32::from_str_radix(&e[2..e.len()], 16).map_err(Some),
        e if e.starts_with('#') => e[1..e.len()].parse::<u32>().map_err(Some),
        _ => Err(None),
    };
    if let Ok(num) = num {
        let char = from_u32(num);
        if let Some(char) = char {
            return Ok(char.into());
        }
    }
    sax.error_state("Invalid character entity");
    Err(format!("&{};", sax.entity))
}

fn apply_entity(sax: &mut SAXState, state: &Entity, parsed_entity: &str) {
    match state {
        Entity::Text(_) => sax.text_node.push_str(parsed_entity),
        Entity::AttributeValueUnquoted(_) | Entity::AttributeValueQuoted(_) => {
            sax.attribute_value.push_str(parsed_entity);
        }
    }
}

impl State for TextEntity {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        handle_entity(sax, char, &Entity::Text(self))
    }

    fn id(&self) -> ID {
        ID::TextEntity
    }
}

impl State for AttributeValueEntityUnquoted {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        handle_entity(sax, char, &Entity::AttributeValueUnquoted(self))
    }

    fn id(&self) -> ID {
        ID::AttributeValueEntityUnquoted
    }
}

impl State for AttributeValueEntityQuoted {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        handle_entity(sax, char, &Entity::AttributeValueQuoted(self))
    }

    fn id(&self) -> ID {
        ID::AttributeValueEntityQuoted
    }
}

#[test]
fn test_parse_entity() {
    let sax = &mut SAXState::default();

    sax.entity = "amp".into();
    assert_eq!(parse_entity(sax), Ok(String::from("&")));

    sax.entity = "#38".into();
    assert_eq!(parse_entity(sax), Ok(String::from("&")));

    sax.entity = "fake_entity".into();
    assert_eq!(parse_entity(sax), Err(String::from("&fake_entity;")));
}
