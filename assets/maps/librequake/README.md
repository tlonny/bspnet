# LibreQuake renderer fixture

`lq_e0m1.bsp` is **Baseless Base Banter**, episode 0 map 1 of
[LibreQuake](https://github.com/lavenderdotpet/LibreQuake). It is a complete
single-player level, rather than a minimal test room.

## Origin

- Release: [v0.09-beta](https://github.com/lavenderdotpet/LibreQuake/releases/tag/v0.09-beta).
- Binary archive: [full.zip](https://github.com/lavenderdotpet/LibreQuake/releases/download/v0.09-beta/full.zip).
- BSP and LIT: extracted unchanged from `full/id1/pak0.pak`, entries
  `maps/lq_e0m1.bsp` and `maps/lq_e0m1.lit`.
- PNG colors were expanded using `gfx/palette.lmp` from the same PAK.
  The palette is not bundled; the RGB PNGs do not need it at runtime.
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

The app loads **`lq_e0m1-notex.bsp`**, a derived BSP29 copy. The original
`lq_e0m1.bsp` remains unchanged for reference. This fixture was converted once;
new levels will be authored with external textures from the outset.

All 108 textures' largest mip levels were extracted into RGB PNGs in
`assets/textures/librequake/`, expanding colors through the original release
palette. Using these PNGs does not require a palette file.
Filenames preserve the original texture names, including special prefixes.
The derived texture lump retains the original texture order, names, and
dimensions, with all four mip offsets zero, matching ericw's `qbsp -notex`
placeholder representation. Other original mip levels are not exported.

The derived BSP has an empty grayscale lighting lump and no embedded RGB lighting.
It retains the original BSPX `BRUSHLIST` collision extension unchanged, updating
its file offset after repacking. The original release fixture also contains this
extension; our current parser ignores it.
`lq_e0m1-notex.lit` is an unchanged copy of the original RGB sidecar. Face
lightmap offsets and styles remain intact; they still describe the original
sample layout even though the grayscale samples have been removed. The rest
of the standard lumps, including geometry, texture mappings, entities, and VIS,
are preserved byte for byte; file offsets are recalculated when packing.

The PNGs and derived BSP retain the LibreQuake licensing and attribution
documented above in `COPYING` and `CREDITS`. The current renderer still uses
depth shading; PNG loading and texturing have not yet been implemented.

For the original, embedded-texture fixture:

The original BSP retains indexed textures; decoding them requires the original
release palette supplied separately. It also contains baked lightmaps and leaf
PVS. Load the `.lit` sidecar for RGB
lighting, or use the BSP's grayscale lighting initially. Other models are
brush entities such as platforms and doors; rendering the complete level
will eventually require their entity transforms too.

The converted BSP and RGB PNGs make world rendering independent of an
installed copy of Quake or a palette file. Models, sprites, sounds, and gameplay assets referenced
by entities are not included. Rebuilding the MAP also requires the texture WADs
from LibreQuake's developer archive; the MAP is included for inspection.
