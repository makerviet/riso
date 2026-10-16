# FGS1 match state protocol

Match state is sent as binary WebSocket messages. Client input and ping/pong
remain JSON text messages because they are small and infrequent.

All numeric values are little-endian. The 16-byte envelope is:

| Offset | Type | Meaning |
| --- | --- | --- |
| 0 | 4 bytes | `FGS1` identifier |
| 4 | `u16` | major version (`1`) |
| 6 | `u16` | compatible minor version (`6`) |
| 8 | `u16` | message type (`1` = state snapshot) |
| 10 | `u16` | envelope flags |
| 12 | `u32` | payload byte length |

The payload contains sections. Each section starts with `tag:u16`,
`flags:u16`, and `byte_length:u32`, followed by exactly `byte_length` bytes.
Readers must skip unknown tags. Existing field meanings and widths are never
changed; additions use a new section tag or append to a section in a new minor
version.

Current section tags:

1. `METADATA`: pack and object metadata.
2. `CLOCKS`: physics tick, authoritative wall clock, simulation clock, and lag.
3. `METRICS`: timing, contact, broad-phase, sleeping, process CPU, and RSS metrics.
4. `PLAYERS`: length-prefixed player records.
5. `OBJECTS`: a count followed by tightly packed `f32 x/y/z` ball positions.
6. `PHYSICS`: pack-provided physical material and robot properties.
7. `SEMANTIC_EVENTS`: recent semantic event strings (debug subscribers only).
8. `DRIVE`: drivetrain limits (periodic full frames and baselines).
9. `SCORE`: `i32` blue, red, global, then a labelled breakdown map.
10. `PLAYER_PHYSICS`: per-player physics detail.
11. `TRANSFER_DEBUG`: reviewer-only transfer mechanism internals.
12. `BALL_DEBUG`: reviewer-only per-ball flags and contacts.
13. `INPUT_ACKS`: input acknowledgements.
14. `SCORE_SHEET`: the full raso-rules score sheet as `u32` byte length plus
    UTF-8 JSON `{"sheet": ScoreSheet | null, "scoringError": string | null}`
    (about 4 KB for FGC 2026). It is sent only on join baselines, on periodic
    full frames (every 10th publish), and in the frame right after the sheet
    or the scoring error changes; clients keep the last received sheet.

The Rust encoder and TypeScript decoder are the executable specification.
Protocol tests validate the identifier, declared payload length, and maximum
size of a 1,000-ball state frame.
