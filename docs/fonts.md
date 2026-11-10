# Local font inputs

Font files are host inputs. Rust accepts owned bytes or explicit local files through
`Renderer::builder`; the CLI accepts repeated `--font FAMILY=FILE`,
`--font-face FAMILY:WEIGHT:STYLE=FILE`, and `--font-fallback FAMILY` flags. The Node facade accepts
`fonts` entries with one `data` or `path` value and an ordered `fallbacks` list. JavaScript reads
paths and passes bytes to the native addon. Mermaid configuration and diagram directives only
select family aliases; they cannot read files or download fonts.

Without host font inputs, the compatible bundled defaults and output remain. Empty input arrays
also retain these defaults. When fonts are supplied, the registry contains only those faces:
register every requested primary or code alias and the desired symbol/emoji coverage. There is no
implicit system discovery or bundled fallback in supplied-font mode.

## Resolution and ownership

The first registered family is the primary if no global family was configured. Explicit
`themeVariables.fontFamily` takes precedence as it does for other rendering. CSS family lists
choose the first registered alias; later registered names and the explicit fallback chain supply
per-character fallback. Unknown names in a list are skipped, but an entirely unresolved primary
list fails even when a fallback font exists. Matching ignores ASCII case.

`font` and `font_file` register weight 400, normal style. Their `font_face` and `font_file_face`
variants accept `FontWeight` and `FontStyle`. Numeric weights range from 1 to 1000. Resolution picks
the nearest registered weight in the requested style, choosing the lighter weight on an equal
distance. Missing italic uses an upright design. The SVG disables synthetic weight and slant and
binds each supplied design to its own subset family, so the browser selects the measured design.

Each renderer owns immutable font bytes and resolved handles. It is safe to reuse or render
concurrently; renderers with different bytes under the same alias have different identities and
do not share a configuration-only font cache. Low-level resolved `Arc<Face>` handles also remain
usable after their registry is dropped. Replacing a registration leaves old handles valid, but
embedding old and new bytes with identical family/weight/style in one SVG is rejected.

## Renderer roles

Global primary selection reaches the unified labels and titles and every custom renderer.
Unspecified per-type family defaults inherit that primary; explicit per-type family settings
remain strict registration requests. The diagram pages describe individual roles. These include
sequence participant/message/note families, C4 element/boundary/message families, journey task
families, Git graph labels, and use-case actor/use-case families. Per-node and per-edge
`font-family` declarations resolve before unified label measurement. Existing Markdown word
weight/style rules and text memo keys are preserved.

Code and monospace roles request the alias `monospace` unless a renderer exposes a configured code
family. Register that alias explicitly. Font size, requested weight and style continue to follow
the renderer's configuration and Markdown rules; registration metadata describes the actual
supplied design selected for them.

## Coverage and formats

TrueType outlines are subset while preserving default layout features, legacy OpenType `kern`
format 0 and color tables. CFF and CFF2 OpenType faces currently embed complete because the
subsetter cannot serialize their outlines; this increases SVG size. Their actual advances and
ink bounds are still measured. Variable fonts use the default variation instance: every declared
axis is pinned to its default in the embedded face, including optical size. Registered
weight/style choose designs, not variation coordinates.

Uncovered characters are measured with the primary face's missing glyph. SVG viewers may instead
draw system fallback with different metrics. Hosts requiring deterministic layout should provide
coverage and check `FontRegistry::missing_glyphs`; it reports each uncovered Unicode character and
family. A primary face remains in the embedded cascade for line metrics even if every visible
character uses an explicit fallback.

Invalid font data, unsupported OpenType-SVG and unsupported legacy kerning formats return typed
text errors. WOFF/WOFF2, host font discovery, automatic downloads, font catalogs, and a no-embedding
output mode are outside this API.
