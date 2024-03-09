use std::char::from_u32;

use crate::{
    file_reader::SAXState,
    references::{ENTITIES, XML_ENTITIES},
    syntactic_constructs::Name,
};

use super::{
    attributes::{AttributeValueQuoted, AttributeValueUnquoted},
    text::Text,
    FileReaderState, State,
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

fn handle_entity(
    file_reader: &mut SAXState,
    char: &char,
    current_state: Entity,
) -> Box<dyn FileReaderState> {
    let (return_current_state, return_state): (Box<dyn FileReaderState>, Box<dyn FileReaderState>) =
        match &current_state {
            Entity::Text(s) => (s.clone(), Box::new(Text)),
            Entity::AttributeValueUnquoted(s) => (s.clone(), Box::new(AttributeValueUnquoted)),
            Entity::AttributeValueQuoted(s) => (s.clone(), Box::new(AttributeValueQuoted)),
        };
    match char {
        ';' => {
            let (entity, is_tag) = parse_entity(file_reader);
            if !is_tag {
                apply_entity(file_reader, current_state, &entity);
            } else {
                todo!("Handling tag entity not implemented");
            }
            file_reader.entity = String::new();
            return_state
        }
        c if file_reader.entity.is_empty() && (Name::is_name_start_char(c) || c == &'#') => {
            file_reader.entity.push(*c);
            return_current_state
        }
        c if !file_reader.entity.is_empty() && (Name::is_name_char(c) || c == &'#') => {
            file_reader.entity.push(*c);
            return_current_state
        }
        _ => {
            apply_entity(
                file_reader,
                current_state,
                &format!("&{};", file_reader.entity),
            );
            file_reader.entity = String::new();
            return_state
        }
    }
}

fn parse_entity(file_reader: &mut SAXState) -> (String, bool) {
    // Lazily build the entity map
    if file_reader.entity_map.is_empty() {
        for &(key, value) in XML_ENTITIES {
            file_reader.entity_map.insert(key.into(), value);
        }
        if file_reader.get_options().strict {
            for &(key, value) in ENTITIES {
                file_reader.entity_map.insert(key.into(), value);
            }
        }
    }

    if let Some(value) = file_reader.entity_map.get(&file_reader.entity) {
        return ((*value).into(), false);
    }
    file_reader.entity = file_reader.entity.to_lowercase();
    if let Some(value) = file_reader.entity_map.get(&file_reader.entity) {
        return ((*value).into(), false);
    }
    let num = match &file_reader.entity {
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
    file_reader.error_state("Invalid character entity");
    (format!("&{};", file_reader.entity), true)
}

fn apply_entity(file_reader: &mut SAXState, state: Entity, parsed_entity: &str) {
    match state {
        Entity::Text(_) => file_reader.text_node.push_str(parsed_entity),
        Entity::AttributeValueUnquoted(_) | Entity::AttributeValueQuoted(_) => {
            file_reader.attribute_value.push_str(parsed_entity)
        }
    }
}

impl FileReaderState for TextEntity {
    fn next(
        self: Box<Self>,
        file_reader: &mut crate::file_reader::SAXState,
        char: &char,
    ) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        handle_entity(file_reader, char, Entity::Text(self))
    }

    fn id(&self) -> State {
        State::TextEntity
    }
}

impl FileReaderState for AttributeValueEntityUnquoted {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        handle_entity(file_reader, char, Entity::AttributeValueUnquoted(self))
    }

    fn id(&self) -> State {
        State::AttributeValueEntityUnquoted
    }
}

impl FileReaderState for AttributeValueEntityQuoted {
    fn next(self: Box<Self>, file_reader: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        handle_entity(file_reader, char, Entity::AttributeValueQuoted(self))
    }

    fn id(&self) -> State {
        State::AttributeValueEntityQuoted
    }
}
