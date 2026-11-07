# LibreQuake renderer fixture

`lq_e0m1.bsp` is **Baseless Base Banter**, episode 0 map 1 of
[LibreQuake](https://github.com/lavenderdotpet/LibreQuake). It is a complete
single-player level, rather than a minimal test room.

## Origin

- Release: [v0.09-beta](https://github.com/lavenderdotpet/LibreQuake/releases/tag/v0.09-beta).
- Binary archive: [full.zip](https://github.com/lavenderdotpet/LibreQuake/releases/download/v0.09-beta/full.zip).
- BSP and LIT: extracted unchanged from `full/id1/pak0.pak`, entries
  `maps/lq_e0m1.bsp` and `maps/lq_e0m1.lit`.
- `palette.lmp`: copied unchanged from the release tag's
  [`lq1/gfx/palette.lmp`](https://github.com/lavenderdotpet/LibreQuake/blob/v0.09-beta/lq1/gfx/palette.lmp).
  This is the shared palette used to expand the BSP's embedded texture indices.
- MAP and CONF: copied from the release tag's
  [source map](https://github.com/lavenderdotpet/LibreQuake/blob/v0.09-beta/lq1/maps/src/e0/lq_e0m1.map)
  and [build configuration](https://github.com/lavenderdotpet/LibreQuake/blob/v0.09-beta/lq1/maps/src/-buildconfigs-/e0/lq_e0m1.conf).
- `COPYING` and `CREDITS`: copied from the release archive's `full/id1/docs/`.
  Retain these notices when redistributing the assets.

The release tag's [dependency setup](https://github.com/lavenderdotpet/LibreQuake/blob/v0.09-beta/.github/scripts/install_deps.sh)
selects ericw-tools **2.0.0-alpha9**. The per-map configuration requests
`qbsp -transwater -splitturb`, `vis -noambientsky`, and `light -extra4`.
Worldspawn specifies `_bounce 3`, `_dirt 1`, `_sunlight 250`,
`_sunlight_color "200 200 255"`, and `_sunlight2 300`.
These establish the upstream build settings; an original compile log is not
included, so the exact invocation used for the shipped binary is not verified.

## Verified binary contents

| Property | Value |
| --- | --- |
| Format | Quake BSP29 |
| BSP size | 1,740,788 bytes |
| Vertices | 4,643 |
| Faces | 4,145 |
| Leaves, including solid leaves | 1,302 |
| World visibility leaves | 1,047 |
| Models, including world | 26 |
| Embedded textures | 108; no missing texture entries |
| Compressed VIS lump | 24,332 bytes |
| Grayscale lighting lump | 187,232 bytes |
| Colored lighting sidecar | QLIT version 1; 561,704 bytes |

All 1,047 available world PVS rows were decompressed and checked for the
expected row length. Their visible-leaf counts range from 1 to 255, providing
nontrivial visibility culling cases. The LIT payload has exactly three RGB
bytes per grayscale lighting sample, after its eight-byte header.

Player start: `(512, 2048, 304)`, yaw `15` degrees in Quake coordinates.

## Using this fixture

Run `cargo run` from the project. The viewer loads `lq_e0m1.bsp`,
`lq_e0m1.lit`, and `palette.lmp` from this directory using paths relative to
Cargo's manifest directory. It requires no installed Quake game files.

The BSP's four indexed texture mip levels are expanded through the palette
and uploaded as sRGB color textures. External RGB lighting is packed on the
standard sixteen-unit lightmap grid into a linear texture array. All four
light style slots are treated as enabled. The original BSPX extension is ignored.

The app keeps the BSP for camera-to-leaf lookup. The mesh precomputes each
leaf's visible world-face indices and texture batches, and uploads the selected
leaf's indices to one reusable GPU buffer. The window title shows the current
leaf. Clockwise Quake face winding is converted to the pipeline's
counterclockwise triangle order.

Controls: WASD to fly, Space / Left Ctrl to move up/down, Left Shift for faster
movement, left click to toggle mouse look, and Escape to quit. Movement is
noclip. The camera starts at the player spawn with a 22-unit eye offset.

This is a world-geometry viewer: brush entities such as doors and platforms,
models, sprites, sounds, gameplay, animated light styles, and special sky/liquid
rendering remain separate work. Sky and liquid faces currently use their ordinary
textures. No distance-based brightness effect is applied.

The existing PNG exports under `assets/textures/librequake/` and the old
`lq_e0m1-notex.lit` copy are not used by this viewer.

The BSP, sidecars, and PNG exports retain the LibreQuake licensing and attribution
in `COPYING` and `CREDITS`; keep those notices when redistributing the assets.
