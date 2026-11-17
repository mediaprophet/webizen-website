# EWM streaming rollback

Production `streaming.default` stays false until the user accepts EWM-08 evidence.

## One-command public revert

Leave `data/ewm/first-paint.json` `streaming.default` false, leave `packages/current.json` `streaming.default` false, and omit `?ewm=1` (and do not set canvas `data-ewm-streaming=1`).

The lush-woodland whole-file boot in `native/world-host/src/runtime/boot.rs` remains loadable. Shards under `packages/qbdl/` stay on disk.

Helper: `native/world-host/src/runtime/stream/flag.rs` constant `ROLLBACK`.

`ewm=1` is independent of `gpu-timing=1`.
