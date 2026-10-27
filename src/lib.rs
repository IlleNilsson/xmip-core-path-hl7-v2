#![forbid(unsafe_code)]

//! The hl7-v2 path technology — a technology of `xmip-core-path`.
//!
//! [`Hl7v2Language`] is the [`PathLanguage`] `hl7-v2`: a field, component
//! or subcomponent of an HL7 v2 message in ER7, read for promote, route
//! and process and written for demote. The message is read by the
//! contract technology's own reader (`xmip-core-contract-hl7-v2`), once
//! per Message however many paths read it, and a value is read and
//! written through its escaping.
//!
//! A path is HL7's own notation, every number from 1:
//! `SEG[#N]-FIELD[[REPETITION]][-COMPONENT[-SUBCOMPONENT]]` — `PID-5-2` is
//! the given name, `OBX#2-5` the value of the second `OBX`, `PID-3[2]-1`
//! the first component of the second identifier. A write replaces one
//! value and writes the message again, segments ended by CR as HL7 v2
//! ends them, a new Stream (ADR-0013).

use contract::ContractError;
use hl7::{Encoding, Message};
use path::{CompiledExpression, Content, Editable, Form, PathLanguage, Rewriting};
use stream::Stream;
use xcore::ScalarValue;

/// The language `hl7-v2`.
pub struct Hl7v2Language;

impl PathLanguage for Hl7v2Language {
    fn language(&self) -> &'static str {
        "hl7-v2"
    }

    fn compile(&self, expression: &str) -> Result<Box<dyn CompiledExpression>, ContractError> {
        Ok(Box::new(Address::parse(expression)?))
    }
}

/// Where a path points.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Address {
    segment: String,
    occurrence: usize,
    field: usize,
    repetition: usize,
    component: usize,
    subcomponent: usize,
}

impl Address {
    fn parse(expression: &str) -> Result<Self, ContractError> {
        let refuse = || {
            ContractError::new(format!(
                "{expression:?} is not SEG[#N]-FIELD[[REPETITION]][-COMPONENT[-SUBCOMPONENT]], \
                 counted from 1"
            ))
        };
        let number = |text: &str| text.trim().parse::<usize>().ok().filter(|n| *n > 0);
        let mut parts = expression.trim().split('-');
        let head = parts.next().ok_or_else(refuse)?;
        let (segment, occurrence) = match head.split_once('#') {
            Some((segment, n)) => (segment, number(n).ok_or_else(refuse)?),
            None => (head, 1),
        };
        let field = parts.next().ok_or_else(refuse)?;
        let (field, repetition) = match field.split_once('[') {
            Some((field, rest)) => (
                field,
                rest.strip_suffix(']').and_then(number).ok_or_else(refuse)?,
            ),
            None => (field, 1),
        };
        let field = number(field).ok_or_else(refuse)?;
        let component = parts.next().map_or(Some(1), number).ok_or_else(refuse)?;
        let subcomponent = parts.next().map_or(Some(1), number).ok_or_else(refuse)?;
        let sound = segment.len() == 3 && segment.bytes().all(|b| b.is_ascii_alphanumeric());
        if parts.next().is_some() || !sound {
            return Err(refuse());
        }
        Ok(Self {
            segment: segment.to_string(),
            occurrence,
            field,
            repetition,
            component,
            subcomponent,
        })
    }
}

/// A message's segments, each its fields as written, `MSH-1` at index 1.
struct Segments {
    encoding: Encoding,
    segments: Vec<Vec<String>>,
}

impl Segments {
    fn read(text: &str) -> Result<Self, ContractError> {
        let message = Message::parse(text).map_err(|issue| {
            ContractError::new(format!("not an HL7 v2 message: {}", issue.message))
        })?;
        Ok(Self {
            encoding: message.encoding,
            segments: message
                .segments
                .iter()
                .map(|fields| fields.iter().map(|field| (*field).to_string()).collect())
                .collect(),
        })
    }

    fn index(&self, address: &Address) -> Option<usize> {
        self.segments
            .iter()
            .enumerate()
            .filter(|(_, fields)| fields[0] == address.segment)
            .nth(address.occurrence - 1)
            .map(|(at, _)| at)
    }

    /// Where a value lies in a field: repetition, component, subcomponent.
    fn value(&self, field: &str, address: &Address) -> Option<String> {
        let e = self.encoding;
        let repetition = field.split(e.repetition).nth(address.repetition - 1)?;
        let component = repetition.split(e.component).nth(address.component - 1)?;
        let sub = component
            .split(e.subcomponent)
            .nth(address.subcomponent - 1)?;
        Some(e.unescape(sub))
    }

    fn written(&self) -> String {
        let field = self.encoding.field.to_string();
        let lines: Vec<String> = self
            .segments
            .iter()
            .map(|fields| {
                if fields[0] == "MSH" {
                    // MSH-1 is the separator itself, put back where it stands.
                    format!("MSH{field}{}", fields[2..].join(&field))
                } else {
                    fields.join(&field)
                }
            })
            .collect();
        let mut text = lines.join("\r");
        text.push('\r');
        text
    }
}

impl Form for Segments {
    fn parse(stream: &Stream) -> Result<Self, ContractError> {
        let text = stream
            .text()
            .map_err(|error| ContractError::new(format!("not UTF-8 text: {error}")))?;
        Self::read(text)
    }
}

impl Editable for Segments {
    const MEDIA_TYPE: &'static str = "x-application/hl7-v2+er7";

    fn open(bytes: &[u8]) -> Result<Self, ContractError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| ContractError::new(format!("not UTF-8 text: {error}")))?;
        Self::read(text)
    }

    fn into_bytes(self) -> Result<Vec<u8>, ContractError> {
        Ok(self.written().into_bytes())
    }
}

impl CompiledExpression for Address {
    fn read(&self, content: &Content<'_>) -> Result<Option<ScalarValue>, ContractError> {
        let segments = content.form::<Segments>()?;
        Ok(segments
            .index(self)
            .and_then(|at| segments.segments[at].get(self.field))
            .and_then(|field| segments.value(field, self))
            .filter(|value| !value.is_empty())
            .map(ScalarValue::Text))
    }

    /// Replace one value. A segment the message does not hold is refused;
    /// a field, repetition, component or subcomponent past the end is
    /// added, padded with empty ones. `MSH-1` and `MSH-2`, the encoding
    /// characters, are not written.
    fn write(&self, rewriting: &mut Rewriting, value: ScalarValue) -> Result<(), ContractError> {
        if self.segment == "MSH" && self.field <= 2 {
            return Err(ContractError::new(
                "MSH-1 and MSH-2 are the encoding characters",
            ));
        }
        let written = match value {
            ScalarValue::Null => String::new(),
            other => other
                .text()
                .ok_or_else(|| ContractError::new("binary has no text form in HL7 v2"))?
                .into_owned(),
        };
        let segments = rewriting.form_mut::<Segments>()?;
        let e = segments.encoding;
        let at = segments
            .index(self)
            .ok_or_else(|| ContractError::new(format!("the message holds no {}", self.segment)))?;
        let fields = &mut segments.segments[at];
        if fields.len() <= self.field {
            fields.resize(self.field + 1, String::new());
        }
        let field = &mut fields[self.field];
        *field = replaced(
            field,
            [e.repetition, e.component, e.subcomponent],
            [self.repetition, self.component, self.subcomponent],
            &e.escape(&written),
        );
        Ok(())
    }
}

/// `text` with the value at `positions`, cut level by level by `separators`,
/// replaced by `value`, padding with empty parts where it is short.
fn replaced(text: &str, separators: [char; 3], positions: [usize; 3], value: &str) -> String {
    let Some((&separator, rest_separators)) = separators.split_first() else {
        return value.to_string();
    };
    let mut parts: Vec<String> = text.split(separator).map(str::to_string).collect();
    let at = positions[0] - 1;
    if parts.len() <= at {
        parts.resize(at + 1, String::new());
    }
    let inner: [char; 2] = [rest_separators[0], rest_separators[1]];
    parts[at] = replaced_inner(&parts[at], inner, [positions[1], positions[2]], value);
    parts.join(&separator.to_string())
}

fn replaced_inner(text: &str, separators: [char; 2], positions: [usize; 2], value: &str) -> String {
    let mut parts: Vec<String> = text.split(separators[0]).map(str::to_string).collect();
    let at = positions[0] - 1;
    if parts.len() <= at {
        parts.resize(at + 1, String::new());
    }
    let mut subs: Vec<String> = parts[at].split(separators[1]).map(str::to_string).collect();
    let sub = positions[1] - 1;
    if subs.len() <= sub {
        subs.resize(sub + 1, String::new());
    }
    subs[sub] = value.to_string();
    parts[at] = subs.join(&separators[1].to_string());
    parts.join(&separators[0].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract::fixture::stream;
    use xcore::StreamId;

    const ADT: &str = "MSH|^~\\&|LAB|HOSP|EMR|CLINIC|20260908103000||ADT^A01|MSG0001|P|2.5\r\
PID|1||123456^^^HOSP^MR~654321^^^LAB^PI||DOE^JOHN||19800101|M\r\
OBX|1|ST|NOTE||first\rOBX|2|ST|NOTE||a \\F\\ b\r";

    fn read(path: &str) -> Option<ScalarValue> {
        let stream = stream(ADT);
        Hl7v2Language
            .compile(path)
            .expect("compiles")
            .read(&Content::of(&stream))
            .expect("reads")
    }

    fn text(value: &str) -> ScalarValue {
        ScalarValue::Text(value.into())
    }

    #[test]
    fn a_field_component_and_repetition_are_read_in_hl7_notation() {
        assert_eq!(read("MSH-9-2"), Some(text("A01")));
        assert_eq!(read("MSH-10"), Some(text("MSG0001")));
        assert_eq!(read("PID-5-2"), Some(text("JOHN")));
        assert_eq!(read("PID-3[2]-4"), Some(text("LAB")));
        assert_eq!(
            read("OBX#2-5"),
            Some(text("a | b")),
            "escaping is read back"
        );
        assert_eq!(read("PID-30"), None);
        for wrong in ["PID", "PID-0", "pid5", "PID-5-a", "PID-3[0]", "PID-1-1-1-1"] {
            assert!(Hl7v2Language.compile(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn a_value_is_written_escaped_into_a_new_stream() {
        let mut rewriting = Rewriting::of(&stream(ADT), StreamId::new(2));
        let mut write = |path: &str, value: &str| {
            Hl7v2Language.compile(path).and_then(|compiled| {
                compiled.write(&mut rewriting, ScalarValue::Text(value.into()))
            })
        };
        write("PID-5-2", "JANE").expect("writes");
        write("OBX-5", "x^y").expect("escapes");
        write("PID-12-2", "late").expect("pads");
        assert!(write("MSH-2", "x").is_err());
        assert!(write("ZZZ-1", "x").is_err());
        let written = rewriting.finish().expect("finishes");
        let text = String::from_utf8_lossy(written.bytes()).into_owned();
        assert!(text.starts_with("MSH|^~\\&|LAB|"), "{text}");
        assert!(text.contains("|DOE^JANE|"), "{text}");
        assert!(text.contains("OBX|1|ST|NOTE||x\\S\\y\r"), "{text}");
        assert!(text.contains("|M||||^late\r"), "{text}");
    }
}
