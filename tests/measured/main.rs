//! The module's timing program (`doc/governance/rust-style.md` section
//! 5c): a path read held to the millisecond rule (`CONTRIBUTING.md`: over a
//! millisecond beyond load is a defect). A plain `cargo test` leaves it
//! out; the landing runs it after every other test of the module, alone,
//! one test at a time (`cargo test --test measured --all-features --
//! --test-threads=1`), so what is measured is Xmip's work and not its
//! neighbors':
//!
//! - a path cuts only the field it names and walks the rest uncut: `MSH-9-2`
//!   of a laboratory result is read well within the millisecond; of the
//!   same result grown to twenty thousand segments it is load, in
//!   proportion to the message, and is held only to a third of the message
//!   read whole into the reader's fields, measured in the same rounds, so a
//!   read that builds the tree is caught however loaded the machine.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use contract::fixture::stream;
use hl7::er7::Message;
use path::{Content, Namespaces, PathLanguage};
use xcore::ScalarValue;
use xcore::latency::{Spread, spread};
use xmip_core_path_hl7_v2::Hl7v2Language;

/// A hundred fives of Messages for each path.
const ROUNDS: usize = 100 * xcore::fastest::FIVE;

/// A laboratory result with `lines` observations, each with its note.
fn result(lines: usize) -> String {
    let mut text = String::from(
        "MSH|^~\\&|LAB|HOSP|EMR|CLINIC|20260908103000||ORU^R01|MSG0001|P|2.5\r\
         PID|1||123456^^^HOSP^MR||DOE^JOHN||19800101|M\rOBR|1|||CBC\r",
    );
    for line in 1..=lines {
        write!(
            text,
            "OBX|{line}|NM|WBC^White cells||{line}|10*9/L\rNTE|{line}||note {line}\r"
        )
        .expect("held");
    }
    text
}

/// `path` read from `text` as a node reads it, a Message at a time:
/// compiled once, each round a Stream and a `Content` of its own, so the
/// text is decoded and walked every round; and beside each read, the same
/// text decoded and read whole into the reader's fields.
fn read(what: &str, path: &str, text: &str, expected: &str) -> (Spread, Spread) {
    let compiled = Hl7v2Language
        .compile(path, &Namespaces::new())
        .expect("compiles");
    let (mut walked, mut whole) = (Vec::new(), Vec::new());
    for _ in 0..ROUNDS {
        let message = stream(text);
        let started = Instant::now();
        let found = compiled.read(&Content::of(&message)).expect("reads");
        walked.push(started.elapsed());
        assert_eq!(found, Some(ScalarValue::Text(expected.into())), "{what}");
        let message = stream(text);
        let started = Instant::now();
        let tree = Content::of(&message)
            .text()
            .map(|text| Message::parse(text).is_ok());
        whole.push(started.elapsed());
        assert_eq!(tree.ok(), Some(true), "{what}: read whole");
    }
    (
        spread(what, walked),
        spread(&format!("{what}, read whole into the fields"), whole),
    )
}

#[test]
fn a_path_cuts_only_its_field_and_walks_the_rest_uncut() {
    let (small, _) = read(
        "MSH-9-2 of a laboratory result",
        "MSH-9-2",
        &result(1),
        "R01",
    );
    assert!(
        small.fastest_of_five < Duration::from_millis(1),
        "MSH-9-2 of a laboratory result: {small:?}"
    );
    let grown = result(10_000);
    let measured = [
        ("MSH-9-2 of a result of 20,003 segments", "MSH-9-2", "R01"),
        ("NTE#10000-3 of it", "NTE#10000-3", "note 10000"),
    ]
    .map(|(what, path, expected)| (what, read(what, path, &grown, expected)));
    for (what, (walked, whole)) in measured {
        assert!(
            walked.fastest_of_five * 3 < whole.fastest_of_five,
            "{what}: {walked:?} walked, {whole:?} read whole"
        );
    }
}
