# xmip-core-path-hl7-v2

The `hl7-v2` path technology, a technology of
[xmip-core-path](https://github.com/IlleNilsson/xmip-core-path). It carries
`Hl7v2Language`, the `PathLanguage` for the language `hl7-v2`: a field,
repetition, component or subcomponent of an HL7 v2 message in ER7, in HL7's
own notation — `PID-5-2`, `OBX#2-5`, `PID-3[2]-1` — read for promote, route
and process and written for demote, a value read and written through HL7's
escaping (`\F\`, `\S\`, `\T\`, `\R\`, `\E\`).

**A read cuts only its field** (ADR-0066, amendment 2026-11-15). The
Stream's own text, borrowed and never copied, is walked a segment at a time
by the Foundation's one segment walk
([xmip-core-message](https://github.com/IlleNilsson/xmip-core-message)'s
`segment`), through the contract technology's reader
([xmip-core-contract-hl7-v2](https://github.com/IlleNilsson/xmip-core-contract-hl7-v2)'s
`er7::Message::walk`), which hands it the separators `MSH` declares. Only the
field the path names, of the segment it names, is cut; past it the walk goes
on to the end of the message and cuts nothing, so a malformed message is
refused wherever it is malformed, in the reader's words, as before — no
`MSH` at the head, MSH-1 or MSH-2 not what they must be, an identifier that
is not one. The message is never read into a tree to answer a path.

**A write needs the whole message.** It reads the message into the
reader's fields, by the same walk, replaces one value, padding a field,
repetition, component or subcomponent past the end, and writes the message
again, segments ended by CR, a new Stream (ADR-0013).

**Measured** (`tests/measured`, the fastest of five over five hundred
Messages, each read as a node reads it, beside the same text read whole
into the reader's fields in the same rounds): `MSH-9-2` of a laboratory
result, held under the millisecond; of the same result grown to 20,003
segments, the walk being load in proportion to the message, held to a third
of the fields read whole. On 2026-11-15, on a machine carrying other work:
`MSH-9-2` 1.1 µs against 3.1 µs read whole; grown, 1.1 ms against 9.4 ms,
and `NTE#10000-3`, the last segment's field, 1.3 ms against 8.9 ms. The read
it replaced, which built the tree of owned fields, took 7 to 10 µs and,
grown, 17 to 34 ms.

## Verification

The included workflow is manual-only and calls the versioned shared workflow at
`IlleNilsson/.github@v1`.
