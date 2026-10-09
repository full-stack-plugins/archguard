//! Streaming preflight for the opt-in integration YAML profile.
//! Never construct a YAML tree or resolve aliases here.
use super::{IntegrationError, MAX_ARTIFACT_BYTES, fail};
use yaml_rust2::parser::{Event, Parser};

const MAX_DEPTH: usize = 64;
const MAX_EVENTS: usize = 65_536;
const MAX_SCALAR_BYTES: usize = 262_144;
const MAX_TOTAL_SCALAR_BYTES: usize = 8_388_608;

pub(super) fn validate_contract_yaml(bytes: &[u8]) -> Result<(), IntegrationError> {
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return fail("artifact size exceeded");
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| IntegrationError("integration YAML must be UTF-8".into()))?;
    let mut parser = Parser::new_from_str(text);
    let mut depth = 0usize;
    let mut documents = 0usize;
    let mut scalar_bytes = 0usize;
    for _ in 0..MAX_EVENTS {
        let (event, _) = parser
            .next_token()
            .map_err(|e| IntegrationError(format!("invalid integration YAML: {e}")))?;
        match event {
            Event::Alias(_) => return fail("YAML anchors and aliases are unsupported"),
            Event::Scalar(_, _, anchor, _)
            | Event::SequenceStart(anchor, _)
            | Event::MappingStart(anchor, _)
                if anchor != 0 =>
            {
                return fail("YAML anchors and aliases are unsupported");
            }
            Event::Scalar(_, _, _, Some(_))
            | Event::SequenceStart(_, Some(_))
            | Event::MappingStart(_, Some(_)) => {
                return fail("YAML explicit tags are unsupported");
            }
            Event::Scalar(value, _, _, _) => {
                scalar_bytes = scalar_bytes.saturating_add(value.len());
                if value.len() > MAX_SCALAR_BYTES || scalar_bytes > MAX_TOTAL_SCALAR_BYTES {
                    return fail("YAML scalar budget exceeded");
                }
            }
            Event::SequenceStart(_, _) | Event::MappingStart(_, _) => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return fail("YAML depth budget exceeded");
                }
            }
            Event::SequenceEnd | Event::MappingEnd => depth = depth.saturating_sub(1),
            Event::DocumentStart => {
                documents += 1;
                if documents > 1 {
                    return fail("YAML requires one document");
                }
            }
            Event::StreamEnd => {
                return if documents == 1 {
                    Ok(())
                } else {
                    fail("YAML requires one document")
                };
            }
            _ => {}
        }
    }
    fail("YAML event budget exceeded")
}
