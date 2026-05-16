# Instrument

<!-- markdownlint-disable MD013 MD024 -->

[Back to index](index.md)

## `OpenPitInstrument`

Trading instrument view.

The caller owns the memory referenced by both string views.

Semantics:

- both string views set: instrument is present;
- both string views not set: instrument is absent;
- only one string view set: invalid payload.

```c
typedef struct OpenPitInstrument {
    OpenPitStringView underlying_asset;
    OpenPitStringView settlement_asset;
} OpenPitInstrument;
```
