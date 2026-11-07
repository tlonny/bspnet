# GoldSrc renderer fixture

`nzp_warehouse.bsp` is **Warehouse (Classic)** by the NZ:P Team, extracted
unchanged from `nzp/maps/nzp_warehouse.bsp` in the
[NZ:P PC asset archive](https://github.com/nzp-team/assets/releases/download/newest/pc-nzp-assets.zip),
downloaded on 2026-11-03. The `newest` release is mutable; the hashes below
identify the downloaded files.

- BSP SHA-256: `932adf90e42084c364bdfb523ebb160a4f525aae42bc1a76f1700ca9f192f930`
- Archive SHA-256: `ce24eec0780995b23b12a2b07fa59c9ae414bbf533627f9cc1671b369ab9aa8a`
- License: CC BY-SA 4.0; see `COPYING` and the
  [upstream notice](https://github.com/nzp-team/assets/blob/main/LICENSE.md).

## Verified binary contents

| Property | Value |
| --- | --- |
| Format | GoldSrc BSP30 |
| File size | 646,036 bytes |
| Embedded textures | 22, each with four mip levels and a 256-colour RGB palette |
| External or missing texture entries | 0 |
| RGB lighting lump | 258,066 bytes |
| Samples with unequal RGB channels | 66,331 |
| Faces | 1,390; 1,311 with active light styles |
| Light styles | 0 and 3; 87 faces include style 3 |
| Vertices | 2,485 |
| Leaves | 728 |
| World visibility leaves | 131 |
| Compressed visibility lump | 1,503 bytes |
| Brush models, including world | 54 |

Validation checked all standard lump bounds, embedded mip and palette bounds,
all lit faces' RGB sample ranges using their texture-space extents and style
counts, and all 132 available visibility rows against the world PVS row size.
The entity lump identifies the compiler as `ZHLT v3.4 VL34 (Nov 21 2023)`.

This fixture needs no WAD or LIT sidecar for its embedded textures and baked
lighting. External models, sounds and other gameplay assets are not included.
It uses NZ:P entity classes, including `func_door_nzp` and numbered player
spawns; it is a rendering/parser fixture, not a drop-in Half-Life game level.

The decoder loads this fixture's geometry, embedded textures, leaf PVS,
and face-owned RGB lightmaps in tests. Each lit face stores shared lightmap
dimensions and a separate RGB sample array for each active style. The application
currently presents an empty window; rendering is not yet implemented.
