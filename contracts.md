# ericw → bspnet runtime contracts

This document identifies compiler decisions that constrain the runtime: what
ericw assumes, what it writes, and what bspnet must preserve to use the result
correctly. A compiler setting whose effect is fully contained in ordinary baked
geometry or lighting does not require a corresponding runtime API.

Scope: ericw's **Quake output**, including optional BSP2/BSPX and lighting
extensions. Quake II and Half-Life game-specific semantics are outside this
contract. This is an audit of the current compiler, not a claim that bspnet
already implements every feature below. Optional features only become obligations
when the build enables them; unsupported features can instead be excluded from
the map/build specification.

Audited 2026-11-07 against ericw-tools commit
[`36eec1da2a194467e6baac2f444c2dd04b57d266`](https://github.com/ericwa/ericw-tools/tree/36eec1da2a194467e6baac2f444c2dd04b57d266).
Source links below are pinned to that revision. Re-audit when changing compiler
versions or output formats. Documentation and source sometimes differ; explicit
differences are noted below.

## 1. Materials, contents, and appearance

### Liquid prefixes

**Input:** texture names beginning with `*`; `*slime…` and `*lava…` select their
respective contents, while other `*…` names select water.

**Compiler result:** liquid volumes in the BSP and special treatment of their
boundaries, visibility, and possibly lighting.

**Runtime obligation:** use the compiled contents for volume queries and retain
the distinction between liquid boundaries and ordinary solid walls. Surface
appearance alone cannot identify whether a position is inside water. Conversely,
rendering a solid brush with a water shader cannot recover a compiled water volume.

Swimming, buoyancy, damage, fog, tint, opacity, waves, and texture animation are
bspnet policies. Neither lava damage nor Quake's water distortion is required by
the compiler. Ordinary world liquids need no separate runtime entity.

Keep each brush's faces consistent in contents. The manual describes mixed
contents as an error; the audited source warns and uses the first nonempty
contents it encounters. Do not depend on that order-dependent fallback.

Sources: [contents classification][contents], [brush contents selection][map].

### Transparent liquids and visibility

**Input:** `qbsp -transwater` versus `-notranswater`.

**Compiler result:** the portal data used by `vis` either permits visibility
through liquid boundaries or treats them as opaque.

**Runtime obligation:** transparent rendering needs a PVS that includes the
geometry visible through the water. Changing an opacity value cannot restore
geometry excluded by opaque-water visibility. Compiling for transparency and
rendering opaque water is conservative; the reverse can produce missing scenery.

This constrains visibility, not a specific runtime alpha key or numeric opacity.
Source: [QBSP options][qbsp-doc].

### Sky

**Input:** `sky…` texture names.

**Compiler result:** sky contents, sky boundaries, and sunlight tracing through
those boundaries. Sky faces commonly have no lightmap; splitting options can
alter this, so inspect the actual face/texinfo data.

**Runtime obligation:** preserve the authored sky openings and the compiled
contents/visibility assumptions. A sky renderer must cover those openings
without revealing geometry the compiler removed or considered invisible.

The two-layer Quake image, scrolling rates, panorama/cubemap choice, and runtime
sky property names are free choices. A visible sun should agree with the baked
sun direction if visual consistency is desired; moving that sun does not move
baked shadows. The compiler does not derive sunlight from a runtime skybox image.

Sources: [contents and lightmapping rules][contents], [special-face rules][map],
[sunlight settings][light-doc].

### Masked surfaces

**Input:** `{…` textures — opening brace, not closing brace.

**Compiler result:** `light` samples the texture mask when tracing through a
shadow-casting face. For ordinary indexed Quake textures, palette index 255 is
converted to transparent alpha.

**Runtime obligation:** match the baked mask's holes, placement, and texture
coordinates. Changing the replacement texture's cutout can leave shadows that
no longer match its visible silhouette. A different runtime material system is
fine if it reproduces the relevant mask.

The mask does **not** cut holes in collision or automatically prevent structural
brushes from blocking visibility/removing surrounding geometry. Use appropriate
brush/entity construction, such as compiler detail-fence geometry, for openings
that must remain visible. Decide separately whether players or projectiles can
pass through them.

Sources: [alpha-tested ray tracing][trace], [palette conversion][images],
[detail-fence behavior][qbsp-doc].

### Glass and material-dependent baked light

**Input:** brush-model `alpha`, entity/group `_light_alpha`, and texture
colour/alpha. QBSP copies `_light_alpha` from the containing entity (including
`func_group`) into its faces' extended texinfo metadata; it is not an arbitrary
per-face property exposed by the ordinary Quake map format, or a property attached
to a texture image. `_light_alpha` takes precedence over model `alpha` in the
audited lighting tracer. Shadow participation still depends on the model's shadow
setup.

**Compiler result:** partial/coloured transmission through glass. Texture colours
also influence configured bounce lighting and surface-light emission.

**Runtime obligation:** preserve the material assumptions when matching baked
lighting matters. An opaque runtime wall with a bake that transmits light through
it, or a blue runtime pane with a red transmitted-light bake, will disagree.
Runtime opacity changes do not rebake transmission. `_light_alpha` can deliberately
be an artistic approximation, so equality with visual opacity is not mandatory.

Ensure the compiler and runtime see compatible images and palettes. In the audited
Quake path, ericw initializes its built-in game palette; supplying a different
`palette.lmp` only to bspnet does not change the compiler's colour interpretation.
External images used by the compiler must also agree with runtime replacements.

Sources: [glass tracing and alpha precedence][trace], [image loading][images],
[surface lighting][surflight].

### Lit and unlit surfaces

**Input:** `-litwater`/`-splitturb`, `_litwater`, splitting settings, and special
texture names.

**Compiler result:** different combinations of `TEX_SPECIAL`, subdivision, face
light offsets, and lightmap samples.

**Runtime obligation:** respect the actual lightmap availability. Do not assume
all `*` surfaces are unlit, or that every visible face has a lightmap. The special
flag alone does not distinguish water from sky. Keep texture animation/UV
distortion independent from the baked lightmap coordinates unless intentionally
distorting the lighting too.

Sources: [special-face rules][map], [lightmapping eligibility][contents].

## 2. Brush models, geometry, collision, and visibility

### Model identity and placement

**Compiler result:** model 0 is the world; brush entities receive `model "*N"`
references to entries in the model lump. These are model indices, not entity
indices or stable asset identifiers across recompiles.

**Runtime obligation:** render and collide against the referenced model. Preserve
its coordinate system, texture projection, face ranges, and collision roots.
Convert all positions, planes, winding, transforms, and lighting consistently if
bspnet uses a different coordinate basis. Unknown brush-entity classes should not
silently lose their geometry unless omission is deliberate game policy.

**Origin brushes:** QBSP removes the marker, recentres model geometry, adjusts
texture projection, and writes an entity `origin`. Apply that placement exactly
once. An arbitrary brush model is not necessarily centred on its bounds; do not
invent a pivot from its centre or subtract its origin again.

**Legacy `rotate_…` classes:** QBSP recognizes this prefix, obtains the pivot from
the entity's `target`/matching `targetname`, and applies special origin/bounds
handling. Either honor the emitted placement or avoid that reserved prefix for
bspnet's custom entities. It does not supply a runtime rotation controller.

Sources: [model assignment][qbsp], [origin processing][map],
[legacy rotation origin][brush].

### Static world geometry and removed geometry

`func_group`, `func_detail`, `func_detail_wall`, `func_detail_fence`, and
`func_detail_illusionary` become world geometry and lose their independent runtime
identity. Their clipping/collision differences survive in the compiled data.
They cannot subsequently be moved as separate entities without another export
mechanism. In particular, illusionary detail can render without collision.

`skip` faces may be absent from the render mesh while their brush remains solid.
`clip` brushes affect collision hulls without supplying visible faces. Therefore
**collision cannot generally be reconstructed from the rendered triangles**.

`_mirrorinside` can emit reversed interior faces. Preserve their winding and
intended visibility; do not merge them away as accidental duplicate triangles.
Geometry omitted by CSG, outside filling, region compilation, or explicit omission
settings is unavailable to runtime movement, transparency, or noclip views.

Point-entity `origin` values also help QBSP identify occupied space during filling;
`_nofill` and `_lightgrid_hint` can exclude entities from that process. Runtime
spawn/teleport destinations must lie in space retained by the build. A custom
spawn classname is possible, but inventing a position property while omitting the
compiler's `origin` can change which space survives filling. Ensure disconnected
playable regions have suitable compile-time occupants.

Sources: [world-entity classification and face flags][map],
[brush/hull construction][brush], [outside filling][outside].

### Collision hull sizes and contents

Quake clip hulls represent collision against particular pre-expanded actor boxes.
Hull 0 supplies point geometry; the default Quake actor hulls use bounds
`(-16,-16,-32)..(16,16,24)` and `(-32,-32,-64)..(32,32,24)`.

Use the matching hull and origin convention when tracing those actor shapes.
Tracing an arbitrary capsule against one of these hulls does not yield correct
capsule collision. Respect per-model roots and contents terminals rather than
assuming every nonempty contents value is solid.

In particular, `clip` contributes solids to the expanded actor hulls but is absent
from the ordinary point/draw hull. Actor blocking and point-ray blocking therefore
need not agree. Do not assume that a player-blocking clip barrier also blocks a
hitscan trace.

`_hulls` and `-noclip` can omit hulls. `-bmodelcontents` permits non-solid contents
in Quake brush models; if enabled, moving-model contents queries must preserve
those distinctions. A water brush model must not become an ordinary solid simply
because it belongs to an entity.

`-wrbrushes` emits BSPX `BRUSHLIST` for brush-based collision with other actor
shapes. `-wrbrushesonly` also omits the standard clip hulls. This requires an
explicit collision implementation for that representation.

Sources: [game hull dimensions][contents], [hull construction][brush],
[BRUSHLIST layout][bspx-layout], [QBSP options][qbsp-doc].

### Trigger model omission

With `-notriggermodels`, qualifying brush entities receive `mins`/`maxs` instead
of an exported brush model. The audited detector tests a texture name ending in
`trigger`; it is not simply a test for a `trigger_` classname.

If enabled, bspnet must construct the intended trigger volume from the emitted
bounds. This is an axis-aligned bounding-box representation, not preservation of
an arbitrary convex brush shape. Without this option, ordinary brush-model
references remain the relevant geometry contract. A texture named `trigger` also
receives special no-lightmap treatment; rendering and activation are game policy.

Sources: [IsTrigger and ProcessEntity][qbsp], [lightmapping rules][contents].

### PVS and runtime motion

Use the compiled leaf/PVS indexing correctly. Visibility lists describe the static
compiled world; they are not a simulation of doors opening or closing. Separate
moving brush models need culling using their current transformed bounds/leaves.
Origin-brush models do not automatically receive bounds covering every rotation.

`func_illusionary_visblocker` deliberately creates a visibility barrier while
exporting empty contents. Its PVS cannot safely be used to see through that barrier
just because runtime geometry is invisible/non-solid. Similarly, `vis -visdist`
can exclude distant regions: runtime fog/draw-distance choices must hide those
omissions, or the visibility scheme must be changed.

`hint`/`hintskip` do not themselves need runtime interpretation; their result is
already encoded in the BSP/PVS. Quake BSP29 has no Quake II area-portal mechanism
for dynamically opening a closed world's visibility partition.

Sources: [visibility blocker processing][map], [VIS options][vis-doc],
[origin/bounds documentation][qbsp-doc].

## 3. Lightstyles and moving objects

### Lightmap composition

Each face can contain several lightmap contributions identified by **style ID**.
These are additive channels, not complete alternative images of the scene.
At runtime, combine the contributions using the current weight for each ID.
Style 0 is the ordinary steady contribution; unused style slots are sentinels,
not additional lights. Atlas layer indices are not style IDs.

Nonzero style numbers do not contain their own animation waveform. Quake's familiar
flicker patterns and timing belong to game/runtime code; bspnet may define its own
style controllers. All faces using the same style must observe the same weight.

Sources: [style allocation][entities], [per-style lighting][ltface],
[sample/style serialization][write].

### Switchable lights and shared target names

For light-class entities with a `targetname`, `light` assigns a style and writes it
back to the entity. Read the **final lit BSP's** `style`; do not guess an ID from
entity order or assume a fixed generated-style range.

Entities sharing a nonempty target name share the allocated style. The allocator
is also used for switchable shadows, so accidental reuse can couple a lamp and
a shadow controller. Allocate distinct names when their states must be independent.
The runtime decides initial on/off state and what gameplay event changes it;
Quake's trigger system and spawnflag meanings are not needed to drive the channel.

Styled/switchable sunlight and surface lights use the same lightmap-channel
principle. `_surflight_style`/`_surflight_targetname` are not automatic animation
of the emitting material: synchronize its visible emission separately if desired.

`qbsp -onlyents` can replace entity data after styles were allocated. Follow the
documented `light -onlyents` workflow for supported entity-only updates. Changes
to actual lighting/shadow geometry need rebaking; reassignment alone cannot create
missing samples. Ship matching BSP, lightmap sidecars, and entity/style data.

Sources: [entity/style allocation][entities], [surface lights][surflight],
[lighting update options][light-doc].

### Switchable shadows: the exact contract

**Input:** `_switchableshadow "1"` on a brush entity.

**Compiler result:** a generated `switchshadstyle` property and lightmap
contributions for light that can reach a surface when this caster stops blocking
it. The alternative `_switchableshadow_target` light setup uses the controlling
light's `style` instead.

**Runtime obligation:** preserve the generated style association and drive its
weight consistently with whether the caster should obstruct that light:

| Weight of the generated contribution | Result |
| --- | --- |
| 0 | Caster blocks the affected light; its baked shadow remains. |
| Normal enabled weight | Previously obstructed light is added; that shadow disappears. |
| Intermediate weight | Lighting fades between those outcomes; the shadow does not move. |

The model can remain present, rendered, and collidable in either state. This is
an occlusion toggle, not an entity-existence toggle.

**It does not move a door and bake two physical positions.** The audited lighting
path does not interpret door `angle`, `lip`, `speed`, or `wait` to derive an open
pose. Those may be replaced with bspnet's own movement schema.

Consequences:

- A door sliding completely out of the relevant light path can approximate
  closed/open lighting by toggling its shadow contribution.
- An open door with an exposed lip still ought to cast some shadow. Turning off
  the baked shadow also removes the lip's shadow; leaving it on retains the
  closed-position shadow. Neither exactly represents that open pose.
- A rotating or partially open door has the same limitation. Fading the channel
  does not create the moving silhouette. Accurate intermediate shadows need a
  different lighting solution.
- The documented feature applies to style-0 source light. It does not promise
  correct occlusion of other switchable/flickering lights.
- Casters sharing a style act as a group. The tracer stores one dynamic-occluder
  style per ray, so multiple independent overlapping shadow groups are not a
  general solution for all combinations of open/closed objects.
- The source preserves self-shadowing within a caster's own style group even
  when its shadow on other geometry is switched off.

Sources: [style assignment][entities], [occluder tracing][trace],
[routing blocked light to styles][ltface], [documented limitations][light-doc].

### Ordinary baked shadows and lighting on moving models

`_shadow`, `_shadowworldonly`, and `_shadowself` bake results for the compile-time
arrangement. World shadows do not follow later model motion. A moving model also
carries its baked surface lighting with it; it does not automatically acquire
lighting appropriate to its new location.

Choose static participation, self-shadowing, switchable shadows, approximate
uniform lighting, or runtime lighting according to the object's intended motion.
The runtime must not infer that any `func_door` automatically has usable
switchable-shadow data.

Likewise, `_phong` affects baked normals, `_light_twosided` affects illumination,
and lighting-channel masks affect which objects exchange light. These do not
automatically configure runtime smooth shading, backface rendering, or dynamic
light channels. Only reproduce them at runtime if combining dynamic lighting
that should agree with the bake. `FACENORMALS` can supply the baked normal basis.

Sources: [model settings][light-source], [tracing][trace],
[exported normal layout][bspx-layout].

## 4. Output formats and asset interpretation

These contracts apply even when no special entity classname is involved.

| Compiler output/option | Runtime obligation |
| --- | --- |
| BSP29, BSP2, or another target | Decode that exact layout and index width. `-allowupgrade` can upgrade output; disable it or support the upgraded format. Half-Life output also changes collision/palette assumptions. |
| Embedded Quake miptextures | Respect texture dimensions, mip offsets, and the compatible palette. Texinfo coordinates are in texture units; replacement-image resolution must not change physical texture scale. |
| `-notex` / `-xwadpath` | Supply external textures for headers with no embedded pixels. These are different from missing texture-directory entries. |
| Zero-size texture placeholders | Supply a defined replacement/dimension policy, or reject this optional output. Existing UV normalization cannot divide by zero. |
| Standard face lightmaps | Use compiler texture projections, grid extents, sample offsets, style order, and absent-lightmap markers. The ordinary grid is 16 texture units per luxel, not necessarily 16 world units. |
| RGB `.lit` / `RGBLIGHTING` | Decode RGB samples with the correct offset interpretation and matching BSP revision. Select an authoritative source rather than summing duplicate representations. |
| `_lightmap_scale`, `-lmshift`, `LMSHIFT`, `LMOFFSET` | Decode the emitted per-face resolution/offset metadata or use a verified fallback. Do not read higher-resolution samples using the ordinary grid. The docs flag QBSP `_lmscale` as broken in some versions; verify the chosen build. |
| `DECOUPLED_LM` | Use its explicit face dimensions, offset, and world-to-lightmap transform; base-texture UVs no longer define the lightmap grid. |
| `LMSTYLE` / `LMSTYLE16`, `-facestyles` | Support the extended style count/ID width and associated sample layout when required. Four ordinary byte style slots may not describe the full lighting. |
| `.lux` / `LIGHTINGDIR` | Decode directions in the specified representation and transform them consistently with geometry if using directional lightmapping. Plain diffuse lightmapping may ignore them. |
| HDR `.lit`, `LIGHTING_E5BGR9`, experimental `.lit2` | Implement the actual version/encoding before enabling it. These are not interchangeable with ordinary RGB8 `.lit`. |
| `-novanilla`, `-bspxonly` | Required fallback lighting may be absent. Ignoring BSPX is no longer a usable fallback strategy. |
| `LIGHTGRID_OCTREE` | If used to illuminate moving objects, decode sample validity, spatial structure, and per-style contributions. Invalid/occluded samples differ from valid black samples. |
| `FACENORMALS` | If used, respect per-face-corner normal/tangent indices; a shared position may have multiple normals at sharp edges. |
| `-nosubdivide` / large subdivisions | Accept the resulting polygon/lightmap dimensions or constrain compilation to the runtime's limits. |

Lightmap brightness also has an encoding convention: the audited writer identifies
128 as logical unit lighting under Quake's overbright convention. Match the
selected LDR/HDR conversion and runtime exposure deliberately. Compiler gamma/range
processing is already baked; blindly applying it again changes the result.

Sources: [format definitions][bspx-layout], [lighting output assembly][light-source],
[lighting sample encoding][write], [image loading][images],
[QBSP options][qbsp-doc], [LIGHT options][light-doc].

## 5. Other shared conventions

### Ambient sound fields

`vis` can bake four per-leaf ambient levels in the order water, sky, slime, lava.
If bspnet uses these fields, preserve that mapping and choose appropriate sounds.
It can instead ignore them and implement its own audio system.

The water-sound naming rules are narrower than water contents: documented ambient
water matching uses `*water…` and `*04water…`, whereas other `*…` names still compile
as water. `_noambient` and the `-noambient…` switches suppress baked sound fields;
they do not change liquid contents or rendering.

Sources: [VIS options][vis-doc], [ambient field definitions][generic-layout].

### Reserved class names and rewritten entity data

- Class names beginning with `light` are interpreted by `light` as light entities.
  Avoid using that prefix for an unrelated custom gameplay class.
- Compiler world/detail classes, `misc_external_map`, and legacy `rotate_…`
  classes have special processing. Aliases and prefab imports can rewrite
  classname/model/origin values. Consume the final emitted representation;
  do not apply prefab transformations a second time.
- Unknown custom properties are generally retained on surviving runtime entities,
  but they are not guaranteed byte-for-byte opaque. In particular, `light` applies
  Quake `\b` text-colour escape processing to **all entity values**, which can
  remove escapes and set high bits. Custom paths or strings containing that
  sequence can change, and the result need not be UTF-8. Avoid the sequence or
  define an export/encoding strategy; do not decode it again at runtime.
- Underscores are not a guaranteed namespace boundary: the settings reader accepts
  documented settings/aliases in context. Use distinct names for custom behavior
  rather than assuming `_` alone makes a property private to one consumer.

Sources: [entity processing][entities], [prefabs/aliases/classification][map],
[model output][qbsp], [settings parser][settings].

### Animation names are optional packaging conventions

QBSP recognizes `+` frame names and attempts to include their numbered/alternate
frames from the texture sources. That creates an asset-packaging convenience;
it does not require bspnet to implement Quake's animation timing or alternate-frame
selection. Bspnet may drop that naming convention and package its own animation
frames explicitly. No compiler feature automatically packages arbitrary
`wall0`, `wall1`, `wall2` sequences. `+0*water` also does not begin with the liquid
prefix and should not be used to combine the conventions.

Source: [AddAnimationFrames][qbsp], [contents classification][contents].

## 6. What remains bspnet's choice

Door/button movement schemas and defaults, vertical angle sentinels, lip semantics,
speed, wait times, trigger wiring, spawnflags, player movement, damage, sky image
layout, water distortion, and animation playback can be designed independently.
Use generated geometry/style references where needed, without reproducing Quake's
gameplay API around them.

Compiler-only inputs such as `wad`, ordinary light intensity/falloff, bounce/AO
settings, hint placement, and CSG ordering usually need no runtime interpretation.
Their output is already baked. If bspnet adds a corresponding runtime effect,
decide whether it supplements or replaces the bake to avoid double lighting or
contradictory geometry/material assumptions.

## 7. Current bspnet implications

This section records observations of the working tree during the audit; it is not
a complete implementation review or a list of approved code changes.

- [The directory decoder](src/bsp/directory.rs) accepts BSP29 only. Pin compilation
  accordingly until extended formats are implemented.
- [Texture decoding](src/bsp/texture.rs) expects embedded mip data and classifies
  sky/liquids by name. [Texture upload](src/level/texture.rs) currently implements
  classic sky masking, not the `{` cutout contract.
- [Entity decoding](src/bsp/entity/mod.rs) retains selected worldspawn/door/button
  fields and discards unsupported classes/properties. Generated light/shadow style
  controls and general model-origin support need explicit handling before maps
  rely on those contracts. Its UTF-8 requirement also matters for Quake text escapes.
- [The lightmap builder](src/level/lightmap/builder.rs) records style IDs, but
  [the solid shader](shader/solid.wgsl) sums present lightmap layers without
  independent runtime style weights. This does not implement switchable shadows.
- Existing Quake door/button defaults are runtime choices. Their presence in the
  code does not show that ericw requires those movement conventions.

Before adopting an optional compiler feature, specify its runtime consumer, its
asset representation, and its behavior when unsupported. A small compile/load
fixture should verify the boundary when that feature is implemented—particularly
shadow weight polarity, origin placement, liquid PVS, and extended lightmap offsets.

[contents]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/common/bspfile.cc
[map]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/qbsp/map.cc
[brush]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/qbsp/brush.cc
[qbsp]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/qbsp/qbsp.cc
[outside]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/qbsp/outside.cc
[entities]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/light/entities.cc
[trace]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/light/trace_embree.cc
[ltface]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/light/ltface.cc
[light-source]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/light/light.cc
[write]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/light/write.cc
[surflight]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/light/surflight.cc
[images]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/common/imglib.cc
[bspx-layout]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/include/common/bspxfile.hh
[generic-layout]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/include/common/bspfile_generic.hh
[settings]: https://github.com/ericwa/ericw-tools/blob/36eec1da2a194467e6baac2f444c2dd04b57d266/common/settings.cc
[qbsp-doc]: https://ericw-tools.readthedocs.io/en/latest/qbsp.html
[light-doc]: https://ericw-tools.readthedocs.io/en/latest/light.html
[vis-doc]: https://ericw-tools.readthedocs.io/en/latest/vis.html
