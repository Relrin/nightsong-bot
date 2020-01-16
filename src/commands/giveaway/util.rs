use lazy_static::lazy_static;
use regex::Regex;

use crate::commands::giveaway::models::ObjectType;

lazy_static! {
    static ref KEY_REGEX: Regex =
        Regex::new(r"^(?P<value>[^\[]+)?(?P<object_info>\[.+\])?\s*->\s*(?P<description>.+)?")
            .unwrap();
}

#[readonly::make]
pub struct ParsedInput {
    pub value: String,
    pub description: Option<String>,
    pub object_info: Option<String>,
    pub object_type: ObjectType,
}

pub fn parse_message(text: &str) -> ParsedInput {
    match text.contains("->") {
        true => {
            let captures = KEY_REGEX.captures(text).unwrap();
            let parsed_value = match captures.name("value") {
                Some(value) => value.as_str().trim().to_string(),
                None => text.to_owned(),
            };
            let parsed_description = match captures.name("description") {
                Some(description) => Some(description.as_str().trim().to_string()),
                None => None,
            };
            let parsed_object_info = match captures.name("object_info") {
                Some(object_info) => Some(object_info.as_str().trim().to_string()),
                None => None,
            };

            ParsedInput {
                value: parsed_value,
                description: parsed_description,
                object_info: parsed_object_info,
                object_type: ObjectType::Key,
            }
        }
        false => ParsedInput {
            value: text.to_owned(),
            description: None,
            object_info: None,
            object_type: ObjectType::Other,
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::giveaway::models::ObjectType;
    use crate::commands::giveaway::util::parse_message;

    #[test]
    fn test_parse_empty_string() {
        let text = "";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, text);
        assert_eq!(parsed_input.description, None);
        assert_eq!(parsed_input.object_info, None);
        assert_eq!(parsed_input.object_type, ObjectType::Other);
    }

    #[test]
    fn test_parse_key_with_info_and_description() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, "AAAAA-BBBBB-CCCCC-DDDD");
        assert_eq!(parsed_input.description, Some(format!("Some game")));
        assert_eq!(parsed_input.object_info, Some(format!("[Store]")));
        assert_eq!(parsed_input.object_type, ObjectType::Key);
    }

    #[test]
    fn test_parse_key_without_info_and_with_description() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD -> Some game";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, "AAAAA-BBBBB-CCCCC-DDDD");
        assert_eq!(parsed_input.description, Some(format!("Some game")));
        assert_eq!(parsed_input.object_info, None);
        assert_eq!(parsed_input.object_type, ObjectType::Key);
    }

    #[test]
    fn test_parse_key_with_info_and_without_description() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] ->";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, "AAAAA-BBBBB-CCCCC-DDDD");
        assert_eq!(parsed_input.description, None);
        assert_eq!(parsed_input.object_info, Some(format!("[Store]")));
        assert_eq!(parsed_input.object_type, ObjectType::Key);
    }

    #[test]
    fn test_parse_key_without_info_and_description() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD ->";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, "AAAAA-BBBBB-CCCCC-DDDD");
        assert_eq!(parsed_input.description, None);
        assert_eq!(parsed_input.object_info, None);
        assert_eq!(parsed_input.object_type, ObjectType::Key);
    }

    #[test]
    fn test_parse_compact_key_with_info_and_description() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD[Store]->Some game";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, "AAAAA-BBBBB-CCCCC-DDDD");
        assert_eq!(parsed_input.description, Some(format!("Some game")));
        assert_eq!(parsed_input.object_info, Some(format!("[Store]")));
        assert_eq!(parsed_input.object_type, ObjectType::Key);
    }

    #[test]
    fn test_parse_compact_key_without_info_and_with_description() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD->Some game";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, "AAAAA-BBBBB-CCCCC-DDDD");
        assert_eq!(parsed_input.description, Some(format!("Some game")));
        assert_eq!(parsed_input.object_info, None);
        assert_eq!(parsed_input.object_type, ObjectType::Key);
    }

    #[test]
    fn test_parse_compact_key_with_info_and_without_description() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD[Store]->";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, "AAAAA-BBBBB-CCCCC-DDDD");
        assert_eq!(parsed_input.description, None);
        assert_eq!(parsed_input.object_info, Some(format!("[Store]")));
        assert_eq!(parsed_input.object_type, ObjectType::Key);
    }

    #[test]
    fn test_parse_compact_key_without_info_and_description() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD->";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, "AAAAA-BBBBB-CCCCC-DDDD");
        assert_eq!(parsed_input.description, None);
        assert_eq!(parsed_input.object_info, None);
        assert_eq!(parsed_input.object_type, ObjectType::Key);
    }

    #[test]
    fn test_parse_key_with_delimiter_only() {
        let text = "->";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, text);
        assert_eq!(parsed_input.description, None);
        assert_eq!(parsed_input.object_info, None);
        assert_eq!(parsed_input.object_type, ObjectType::Key);
    }

    #[test]
    fn test_parse_raw_text() {
        let text = "Not even a key. Just a regular text.";
        let parsed_input = parse_message(text);

        assert_eq!(parsed_input.value, text);
        assert_eq!(parsed_input.description, None);
        assert_eq!(parsed_input.object_info, None);
        assert_eq!(parsed_input.object_type, ObjectType::Other);
    }
}
