# EWM streaming rollback

Production `streaming.default` stays false until the user accepts EWM-08 evidence.

## One-command public revert

Leave `data/ewm/catalog.q42` `streamingDefault` false, leave producer dumps `streaming.default` false, and omit `?ewm=1` (and do not set canvas `data-ewm-streaming=1`).

The woodland homepage no longer whole-file loads `scene.json`. Flag off fails closed to the reading view. The scene graph is `/production/lush-woodland/scene.q42` and the catalog is `/data/ewm/catalog.q42` (Q42VOL v4 CBOR-LD SuperBlocks of NQuins). Visitors never read those files. Shards under `packages/qbdl/` stay on disk.

Helper: `native/world-host/src/runtime/stream/flag.rs` constant `ROLLBACK`.

`ewm=1` is independent of `gpu-timing=1`.
