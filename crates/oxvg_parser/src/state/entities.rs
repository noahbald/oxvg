use std::char::from_u32;

use crate::{
    file_reader::SAXState,
    syntactic_constructs::{names, references},
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

fn handle_entity(sax: &mut SAXState, char: char, current_state: &Entity) -> Box<dyn State> {
    let (return_current_state, return_state): (Box<dyn State>, Box<dyn State>) =
        match &current_state {
            Entity::Text(s) => (s.clone(), Box::new(Text)),
            Entity::AttributeValueUnquoted(s) => (s.clone(), Box::new(AttributeValueUnquoted)),
            Entity::AttributeValueQuoted(s) => (s.clone(), Box::new(AttributeValueQuoted)),
        };
    match char {
        ';' => {
            let (entity, is_tag) = parse_entity(sax);
            if is_tag {
                todo!("Handling tag entity not implemented");
            } else {
                apply_entity(sax, current_state, &entity);
            }
            sax.entity = String::new();
            return_state
        }
        c if sax.entity.is_empty() && (names::is_start(c) || c == '#') => {
            sax.entity.push(c);
            return_current_state
        }
        c if !sax.entity.is_empty() && (names::is(c) || c == '#') => {
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

fn parse_entity(sax: &mut SAXState) -> (String, bool) {
    // Lazily build the entity map
    if sax.entity_map.is_empty() {
        for &(key, value) in references::XML_ENTITIES {
            sax.entity_map.insert(key.into(), value);
        }
        if !sax.get_options().strict {
            for &(key, value) in references::ENTITIES {
                sax.entity_map.insert(key.into(), value);
            }
        }
    }

    if let Some(value) = sax.entity_map.get(&sax.entity) {
        return ((*value).into(), false);
    }
    sax.entity = sax.entity.to_lowercase();
    if let Some(value) = sax.entity_map.get(&sax.entity) {
        return ((*value).into(), false);
    }
    let num = match &sax.entity {
        e if e.starts_with("#x") => u32::from_str_radix(&e[2..e.len()], 16).map_err(Some),
        e if e.starts_with('#') => e[1..e.len()].parse::<u32>().map_err(Some),
        _ => Err(None),
    };
    if let Ok(num) = num {
        let char = from_u32(num);
        if let Some(char) = char {
            return (char.into(), false);
        }
    }
    sax.error_state("Invalid character entity");
    (format!("&{};", sax.entity), true)
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
