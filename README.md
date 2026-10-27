# xmip-core-path-hl7-v2

The `hl7-v2` path technology, a technology of
[xmip-core-path](https://github.com/IlleNilsson/xmip-core-path). It carries
`Hl7v2Language`, the `PathLanguage` for the language `hl7-v2`: a field,
repetition, component or subcomponent of an HL7 v2 message in ER7, in HL7's
own notation — `PID-5-2`, `OBX#2-5`, `PID-3[2]-1` — read for promote, route
and process and written for demote. The message is read by the contract
technology's own reader, once per Message, and a value is read and written
through HL7's escaping (`\F\`, `\S\`, `\T\`, `\R\`, `\E\`); a write writes the
message again, segments ended by CR, a new Stream (ADR-0013).
## Toolchain

`rust-toolchain.toml` pins the toolchain for the whole estate. Do not change it
here.

## Verification

The included workflow is manual-only and calls the versioned shared workflow at
`IlleNilsson/.github@v1`.
