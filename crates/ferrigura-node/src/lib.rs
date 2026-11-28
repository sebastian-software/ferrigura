//! The `ferrigura` Node addon, Node.js only; there is no WebAssembly build.
//!
//! `render(text, config?, options?, fonts?)` returns the SVG of a Mermaid diagram, with the options of the
//! `ferrigura` crate. `Renderer` resolves a configuration once, for many renders:
//! `new Renderer(config?, fonts?)`, then `renderer.render(text, options?)`. Both give the same SVG for the
//! same configuration, byte for byte.
//!
//! The crate has one cargo feature per diagram type, named as its constructor in
//! `ferrigura::diagrams`, and `all`, the default. A build with fewer features draws fewer types and
//! links only those (decision 0020); a text of another type is refused as not registered, and a
//! text of a type that ferrigura does not draw is refused with an error that names the type.

#[cfg(not(feature = "all"))]
use std::sync::LazyLock;
use std::sync::{Arc, Mutex, PoisonError};

#[cfg(any(test, not(feature = "all")))]
use ferrigura::diagrams::{self, Plugin};
use ferrigura::{Animation, ColorScheme, FontStyle, FontWeight, RenderOptions, RendererBuilder};
use ferrigura_model::value::Value;
use napi::bindgen_prelude::Uint8Array;
use napi::{Error, Result, Status};
use napi_derive::napi;

// A render makes many small allocations, which mimalloc serves faster than the system allocator. The
// targets are the ones of the `mimalloc` dependency in Cargo.toml. The default features are off, so
// mimalloc does not replace the process's malloc, which Node and V8 share.
#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", target_env = "gnu"),
    all(target_os = "windows", target_env = "msvc")
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// One host-supplied face. The JavaScript facade reads local files before calling the addon.
#[napi(object)]
pub struct FontInput {
    /// Explicit family name, including spaces when needed.
    pub family: String,
    /// Font bytes, copied into the renderer during construction.
    pub data: Uint8Array,
    /// Integer CSS weight from 1 through 1000. Defaults to 400.
    pub weight: Option<f64>,
    /// `normal` (default) or `italic`.
    pub style: Option<String>,
}

/// Renderer-scoped font registration, separate from Mermaid configuration and SVG options.
#[napi(object)]
pub struct FontOptionsInput {
    /// Faces registered once before rendering. The first family is the default unless configured.
    pub fonts: Option<Vec<FontInput>>,
    /// Registered fallback family names, in order. No files are discovered or downloaded.
    pub fallbacks: Option<Vec<String>>,
}

/// The options that are not Mermaid configuration.
#[napi(object)]
pub struct RenderOptionsInput {
    /// Optional draw animation. An empty object enables the default speed and source order.
    pub animation: Option<AnimationInput>,
    /// Mermaid-compatible geometry when true. False (default) enables optimized flowchart routing
    /// and compact sequence participant boxes. Font, color and readability fixes always apply.
    pub compat: Option<bool>,
    /// The design mode when true: ferrigura's own look for flowcharts, state and class diagrams, mindmaps, and pie, XY, quadrant and gantt charts, with
    /// roomier nodes and spacing for the first three. Other diagram types and the combination with `compat` are rejected.
    pub design: Option<bool>,
    /// The id of the SVG element, which scopes its styles. By default it is derived from the text and
    /// the configuration, so the same input gives the same id.
    pub id: Option<String>,
    /// The colors: `light`, `dark`, or `auto` to follow the viewer's color scheme. By default the
    /// configured theme decides.
    pub color_scheme: Option<String>,
    /// Fixed UTC time in integer milliseconds since the Unix epoch.
    pub now: Option<f64>,
}

/// Options for one-time, script-free SVG draw animation.
/// Supports flowcharts, Gantt, mind maps, pie charts, and XY charts.
#[napi(object)]
pub struct AnimationInput {
    /// Positive, finite multiplier of 700 SVG user units per second on average. Defaults to 1.
    /// Each connection, bar, or sector eases in and out independently.
    pub speed: Option<f64>,
    /// `source` (default) draws one line at a time; `flow` draws branches in parallel.
    pub order: Option<String>,
}

/// Renders a diagram to an SVG document.
///
/// `config` is the configuration that `mermaid.initialize` takes, as an object. Front matter and
/// `%%{init}%%` directives in `text` apply on top of it, as they do in Mermaid. To render many diagrams
/// with one configuration, use `Renderer`, which resolves the configuration once.
///
/// # Errors
///
/// Throws if the text is not a diagram that ferrigura renders or of a type this build leaves out, if it
/// does not parse, if the configuration holds a value of the wrong type, or if the color scheme or the
/// id is not valid.
#[napi]
pub fn render(
    text: String,
    config: Option<serde_json::Value>,
    options: Option<RenderOptionsInput>,
    fonts: Option<FontOptionsInput>,
) -> Result<String> {
    let config = parse_config(config)?;
    let options = render_options(options)?;
    let (faces, fallbacks) = parse_fonts(fonts.as_ref())?;
    let renderer = cached_renderer(&config, &faces, &fallbacks)?;
    renderer.render_with(&text, &options).map_err(to_error)
}

/// A renderer that resolves its configuration once, for many renders.
///
/// `new Renderer(config?, fonts?)` takes the configuration that `render` takes as its second argument, and
/// `renderer.render(text, options?)` renders a diagram with it, as `render` does. The configuration is
/// merged onto the defaults when the renderer is made, and the typed view of each diagram type is
/// resolved at its first render; a render then merges only the diagram's own front matter and
/// directives. The output is the same as `render` gives with the same configuration. A renderer lives
/// as long as JavaScript holds it.
#[napi]
pub struct Renderer {
    inner: ferrigura::Renderer,
}

#[napi]
impl Renderer {
    /// Resolves the configuration of a renderer. A value of the wrong type is reported by the render
    /// that reads it, as `render` reports it.
    ///
    /// # Errors
    /// Throws immediately if `config` is not an object (when provided) or if supplied font bytes or
    /// face metadata are invalid.
    #[napi(constructor)]
    pub fn new(config: Option<serde_json::Value>, fonts: Option<FontOptionsInput>) -> Result<Self> {
        let config = parse_config(config)?;
        let (faces, fallbacks) = parse_fonts(fonts.as_ref())?;
        let (builder, _) = build_with_fonts(builder(&config), &faces, &fallbacks)?;
        Ok(Self {
            inner: builder.build(),
        })
    }

    /// Renders a diagram to an SVG document, with the configuration of this renderer.
    ///
    /// # Errors
    ///
    /// Throws as `render` does: if the text is not a diagram that ferrigura renders, if it does not
    /// parse, if the configuration or the text holds a value of the wrong type, or if the color
    /// scheme or the id is not valid.
    #[napi]
    pub fn render(&self, text: String, options: Option<RenderOptionsInput>) -> Result<String> {
        let options = render_options(options)?;
        self.inner.render_with(&text, &options).map_err(to_error)
    }
}

/// Validates a JS `config` already converted by N-API's depth- and node-bounded `serde_json::Value`
/// deserializer (so any JS getters run before `fonts` `Uint8Array` slices are extracted), rejecting
/// non-object values while treating `undefined` and `null` as an empty object.
fn parse_config(config: Option<serde_json::Value>) -> Result<Value> {
    match config {
        None | Some(serde_json::Value::Null) => Ok(Value::empty_object()),
        Some(value @ serde_json::Value::Object(_)) => Ok(Value::from(value)),
        Some(_) => Err(Error::new(
            Status::InvalidArg,
            "configuration must be an object",
        )),
    }
}

#[cfg(test)]
fn validate_config(config: Option<Value>) -> Result<Value> {
    match config {
        None | Some(Value::Null) => Ok(Value::empty_object()),
        Some(value) if value.is_object() => Ok(value),
        Some(_) => Err(Error::new(
            Status::InvalidArg,
            "configuration must be an object",
        )),
    }
}

struct BorrowedFace<'a> {
    family: &'a str,
    data: &'a [u8],
    weight: FontWeight,
    style: FontStyle,
}

/// Validates host-supplied font metadata and borrows the byte slices without copying.
fn parse_fonts(input: Option<&FontOptionsInput>) -> Result<(Vec<BorrowedFace<'_>>, Vec<String>)> {
    let Some(input) = input else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mut faces = Vec::new();
    if let Some(fonts) = &input.fonts {
        faces.reserve(fonts.len());
        for font in fonts {
            let weight = font.weight.unwrap_or(400.0);
            if !weight.is_finite() || weight.fract() != 0.0 || !(1.0..=1000.0).contains(&weight) {
                return Err(Error::new(
                    Status::InvalidArg,
                    "font weight must be an integer from 1 through 1000",
                ));
            }
            let weight = match weight as u16 {
                400 => FontWeight::Normal,
                700 => FontWeight::Bold,
                weight => FontWeight::Number(weight),
            };
            let style = match font.style.as_deref().unwrap_or("normal") {
                "normal" => FontStyle::Normal,
                "italic" => FontStyle::Italic,
                _ => {
                    return Err(Error::new(
                        Status::InvalidArg,
                        "font style must be normal or italic",
                    ));
                }
            };
            faces.push(BorrowedFace {
                family: &font.family,
                data: font.data.as_ref(),
                weight,
                style,
            });
        }
    }
    let fallbacks = input.fallbacks.clone().unwrap_or_default();
    Ok((faces, fallbacks))
}

struct OwnedFaceKey {
    family: String,
    data: Arc<Vec<u8>>,
    weight: FontWeight,
    style: FontStyle,
}

struct FontCacheKey {
    fingerprint: u64,
    faces: Vec<OwnedFaceKey>,
    fallbacks: Vec<String>,
}

impl FontCacheKey {
    fn matches(&self, fingerprint: u64, faces: &[BorrowedFace<'_>], fallbacks: &[String]) -> bool {
        self.fingerprint == fingerprint
            && self.fallbacks == fallbacks
            && self.faces.len() == faces.len()
            && self.faces.iter().zip(faces).all(|(cached, incoming)| {
                cached.family == incoming.family
                    && cached.weight == incoming.weight
                    && cached.style == incoming.style
                    && cached.data.as_slice() == incoming.data
            })
    }
}

fn font_fingerprint(faces: &[BorrowedFace<'_>], fallbacks: &[String]) -> Option<u64> {
    if faces.is_empty() && fallbacks.is_empty() {
        return None;
    }
    fn add(hash: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *hash = (*hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for face in faces {
        add(&mut hash, &(face.family.len() as u64).to_le_bytes());
        add(&mut hash, face.family.as_bytes());
        add(&mut hash, &face.weight.css_value().to_le_bytes());
        add(&mut hash, face.style.css_keyword().as_bytes());
        add(&mut hash, &(face.data.len() as u64).to_le_bytes());
        add(&mut hash, face.data);
    }
    add(&mut hash, &(fallbacks.len() as u64).to_le_bytes());
    for family in fallbacks {
        add(&mut hash, &(family.len() as u64).to_le_bytes());
        add(&mut hash, family.as_bytes());
    }
    Some(hash)
}

/// Applies only owned bytes: file and network loading never enter the addon. Duplicate byte slices
/// within one call share a single `Arc<Vec<u8>>`.
fn build_with_fonts(
    mut builder: RendererBuilder,
    faces: &[BorrowedFace<'_>],
    fallbacks: &[String],
) -> Result<(RendererBuilder, Vec<OwnedFaceKey>)> {
    let mut owned_faces: Vec<OwnedFaceKey> = Vec::with_capacity(faces.len());
    for face in faces {
        let data = owned_faces
            .iter()
            .find(|previous| previous.data.as_slice() == face.data)
            .map_or_else(
                || Arc::new(face.data.to_vec()),
                |previous| Arc::clone(&previous.data),
            );
        builder = builder
            .font_face_shared(face.family, Arc::clone(&data), face.weight, face.style)
            .map_err(|error| {
                Error::new(
                    Status::InvalidArg,
                    format!("cannot register font {:?}: {error}", face.family),
                )
            })?;
        owned_faces.push(OwnedFaceKey {
            family: face.family.to_owned(),
            data,
            weight: face.weight,
            style: face.style,
        });
    }
    if !fallbacks.is_empty() {
        builder = builder.font_fallbacks(fallbacks);
    }
    Ok((builder, owned_faces))
}

const RENDERER_CACHE_CAPACITY: usize = 8;
/// Maximum retained supplied-font bytes across all renderers in [`RENDERER_CACHE`].
const RENDERER_CACHE_FONT_BYTE_BUDGET: usize = 16 * 1024 * 1024;

struct CachedRenderer {
    config: Value,
    fonts: Option<FontCacheKey>,
    font_bytes: usize,
    renderer: Arc<ferrigura::Renderer>,
}

fn estimated_font_bytes(faces: &[OwnedFaceKey]) -> usize {
    let mut seen = Vec::new();
    let mut total = 0usize;
    for face in faces {
        let ptr = Arc::as_ptr(&face.data) as usize;
        if !seen.contains(&ptr) {
            seen.push(ptr);
            total = total.saturating_add(face.data.capacity());
        }
        // CFF/CFF2 fonts embed whole with a precomputed base64 payload (~4/3x).
        if face.data.starts_with(b"OTTO") {
            let b64 = face.data.len().saturating_mul(4) / 3 + 4;
            total = total.saturating_add(b64);
        }
    }
    total
}

static RENDERER_CACHE: Mutex<Vec<CachedRenderer>> = Mutex::new(Vec::new());

fn take_cached(
    cache: &mut Vec<CachedRenderer>,
    config: &Value,
    fingerprint: Option<u64>,
    faces: &[BorrowedFace<'_>],
    fallbacks: &[String],
) -> Option<Arc<ferrigura::Renderer>> {
    let position = cache.iter().position(|entry| {
        identical(&entry.config, config)
            && match (&entry.fonts, fingerprint) {
                (None, None) => true,
                (Some(key), Some(fp)) => key.matches(fp, faces, fallbacks),
                _ => false,
            }
    })?;
    let entry = cache.remove(position);
    let renderer = Arc::clone(&entry.renderer);
    cache.push(entry);
    Some(renderer)
}

/// Returns a cached renderer for this configuration and font set, avoiding repeated plugin setup,
/// font copying/validation, and configuration resolution across one-shot `render()` calls.
fn cached_renderer(
    config: &Value,
    faces: &[BorrowedFace<'_>],
    fallbacks: &[String],
) -> Result<Arc<ferrigura::Renderer>> {
    let fingerprint = font_fingerprint(faces, fallbacks);
    {
        let mut cache = RENDERER_CACHE
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(renderer) = take_cached(&mut cache, config, fingerprint, faces, fallbacks) {
            return Ok(renderer);
        }
    }
    let (builder, owned_faces) = build_with_fonts(builder(config), faces, fallbacks)?;
    let font_bytes = estimated_font_bytes(&owned_faces);
    let renderer = Arc::new(builder.build());
    if font_bytes > RENDERER_CACHE_FONT_BYTE_BUDGET {
        return Ok(renderer);
    }
    let fonts_key = fingerprint.map(|fp| FontCacheKey {
        fingerprint: fp,
        faces: owned_faces,
        fallbacks: fallbacks.to_vec(),
    });
    let mut cache = RENDERER_CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(existing) = take_cached(&mut cache, config, fingerprint, faces, fallbacks) {
        return Ok(existing);
    }
    while cache.len() >= RENDERER_CACHE_CAPACITY
        || (font_bytes > 0
            && cache
                .iter()
                .map(|entry| entry.font_bytes)
                .fold(0usize, usize::saturating_add)
                .saturating_add(font_bytes)
                > RENDERER_CACHE_FONT_BYTE_BUDGET)
    {
        if cache.is_empty() {
            break;
        }
        cache.remove(0);
    }
    cache.push(CachedRenderer {
        config: config.clone(),
        fonts: fonts_key,
        font_bytes,
        renderer: Arc::clone(&renderer),
    });
    Ok(renderer)
}

/// Exact value equality matching `ferrigura`'s shorthand cache (`0.0` vs `-0.0`, `NaN == NaN`).
fn identical(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a.to_bits() == b.to_bits(),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| identical(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((key_a, a), (key_b, b))| key_a == key_b && identical(a, b))
        }
        _ => false,
    }
}

/// The render options of the addon's input.
fn render_options(input: Option<RenderOptionsInput>) -> Result<RenderOptions> {
    Ok(match input {
        Some(input) => RenderOptions {
            animation: input
                .animation
                .map(|input| {
                    let mut animation = Animation::new(input.speed.unwrap_or(1.0))
                        .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
                    animation.order = input
                        .order
                        .as_deref()
                        .map(str::parse)
                        .transpose()
                        .map_err(|message| Error::new(Status::InvalidArg, message))?
                        .unwrap_or_default();
                    Ok::<_, Error>(animation)
                })
                .transpose()?,
            compat: input.compat.unwrap_or(false),
            design: input.design.unwrap_or(false),
            id: input.id,
            color_scheme: input
                .color_scheme
                .as_deref()
                .map(color_scheme)
                .transpose()?,
            now: input
                .now
                .map(|now| {
                    if !now.is_finite() || now.fract() != 0.0 || now.abs() > 8_640_000_000_000_000.0
                    {
                        return Err(Error::new(
                            Status::InvalidArg,
                            "now must be integer Unix milliseconds within ±8640000000000000",
                        ));
                    }
                    Ok(now as i64)
                })
                .transpose()?,
        },
        None => RenderOptions::default(),
    })
}

fn color_scheme(name: &str) -> Result<ColorScheme> {
    match name {
        "light" => Ok(ColorScheme::Light),
        "dark" => Ok(ColorScheme::Dark),
        "auto" => Ok(ColorScheme::Auto),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("unknown colour scheme {other}, expected light, dark or auto"),
        )),
    }
}

fn to_error(error: ferrigura::Error) -> Error {
    Error::new(Status::GenericFailure, error.to_string())
}

#[cfg(not(any(
    feature = "pie",
    feature = "flowchart",
    feature = "er",
    feature = "class",
    feature = "sequence",
    feature = "state",
    feature = "gantt",
    feature = "mindmap",
    feature = "xy_chart",
    feature = "quadrant_chart"
)))]
compile_error!(
    "ferrigura-node draws no diagram type: enable at least one feature, or the default `all`"
);

/// The plugins of this build, one per enabled feature.
#[cfg(any(test, not(feature = "all")))]
#[expect(
    clippy::vec_init_then_push,
    reason = "a `vec!` literal cannot hold `#[cfg]` elements"
)]
fn plugins() -> Vec<Plugin> {
    let mut plugins = Vec::new();
    #[cfg(feature = "pie")]
    plugins.push(diagrams::pie());
    #[cfg(feature = "flowchart")]
    plugins.push(diagrams::flowchart());
    #[cfg(feature = "er")]
    plugins.push(diagrams::er());
    #[cfg(feature = "class")]
    plugins.push(diagrams::class());
    #[cfg(feature = "sequence")]
    plugins.push(diagrams::sequence());
    #[cfg(feature = "state")]
    plugins.push(diagrams::state());
    #[cfg(feature = "gantt")]
    plugins.push(diagrams::gantt());
    #[cfg(feature = "mindmap")]
    plugins.push(diagrams::mindmap());
    #[cfg(feature = "xy_chart")]
    plugins.push(diagrams::xy_chart());
    #[cfg(feature = "quadrant_chart")]
    plugins.push(diagrams::quadrant_chart());
    plugins
}

#[cfg(not(feature = "all"))]
static PLUGINS: LazyLock<Box<[Plugin]>> = LazyLock::new(|| plugins().into_boxed_slice());

/// A builder with the plugins of this build.
fn builder(config: &Value) -> RendererBuilder {
    #[cfg(feature = "all")]
    {
        ferrigura::Renderer::builder(config).with_all()
    }
    #[cfg(not(feature = "all"))]
    {
        PLUGINS
            .iter()
            .copied()
            .fold(ferrigura::Renderer::builder(config), RendererBuilder::with)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use super::*;

    const SANS: &[u8] = include_bytes!("../../ferrigura-text/fonts/LexicaUltralegible-Regular.ttf");
    const MONO: &[u8] =
        include_bytes!("../../ferrigura-text/fonts/AtkinsonHyperlegibleMono-Regular.ttf");

    /// Which features this build has, in the order of `plugins`.
    const FEATURES: [bool; 10] = [
        cfg!(feature = "pie"),
        cfg!(feature = "flowchart"),
        cfg!(feature = "er"),
        cfg!(feature = "class"),
        cfg!(feature = "sequence"),
        cfg!(feature = "state"),
        cfg!(feature = "gantt"),
        cfg!(feature = "mindmap"),
        cfg!(feature = "xy_chart"),
        cfg!(feature = "quadrant_chart"),
    ];

    fn render_for_test(text: &str, config: Option<Value>) -> Result<String> {
        let config = validate_config(config)?;
        let renderer = cached_renderer(&config, &[], &[])?;
        renderer
            .render_with(text, &RenderOptions::default())
            .map_err(to_error)
    }

    #[test]
    fn one_plugin_per_enabled_feature() {
        let plugins = plugins();
        let kinds: HashSet<_> = plugins.iter().map(Plugin::kind).collect();
        assert_eq!(kinds.len(), plugins.len());
        assert_eq!(plugins.len(), FEATURES.iter().filter(|on| **on).count());
    }

    #[test]
    fn rejects_non_object_config_and_accepts_null_or_object() {
        assert_eq!(validate_config(None).unwrap(), Value::empty_object());
        assert_eq!(
            validate_config(Some(Value::Null)).unwrap(),
            Value::empty_object()
        );
        let obj = Value::Object(BTreeMap::from([(
            "theme".to_owned(),
            Value::String("dark".to_owned()),
        )]));
        assert_eq!(validate_config(Some(obj.clone())).unwrap(), obj);
        for invalid in [
            Value::Bool(true),
            Value::Number(123.0),
            Value::String("dark".to_owned()),
            Value::Array(vec![]),
        ] {
            let error = validate_config(Some(invalid)).unwrap_err();
            assert_eq!(error.status, Status::InvalidArg);
            assert!(error.reason.contains("object"), "{error}");
        }
    }

    #[test]
    fn caches_one_shot_renderers_for_default_and_supplied_fonts() {
        let empty = Value::empty_object();
        let default_a = cached_renderer(&empty, &[], &[]).unwrap();
        let default_b = cached_renderer(&empty, &[], &[]).unwrap();
        assert!(Arc::ptr_eq(&default_a, &default_b));

        let sans_face = [BorrowedFace {
            family: "Fixture",
            data: SANS,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
        }];
        let mono_face = [BorrowedFace {
            family: "Fixture",
            data: MONO,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
        }];
        let font_a = cached_renderer(&empty, &sans_face, &[]).unwrap();
        let font_b = cached_renderer(&empty, &sans_face, &[]).unwrap();
        let font_c = cached_renderer(&empty, &mono_face, &[]).unwrap();
        assert!(Arc::ptr_eq(&font_a, &font_b));
        assert!(!Arc::ptr_eq(&font_a, &default_a));
        assert!(!Arc::ptr_eq(&font_a, &font_c));
    }

    #[cfg(feature = "sequence")]
    #[test]
    fn draws_a_type_whose_feature_is_on() {
        let svg = render_for_test("sequenceDiagram\n  A->>B: Hi\n", None).unwrap();
        assert!(svg.starts_with("<svg "));
    }

    #[cfg(not(feature = "pie"))]
    #[test]
    fn refuses_a_type_whose_feature_is_off() {
        let error = render_for_test("pie\n  \"Dogs\" : 1\n", None).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("pie"), "{message}");
        assert!(message.contains("not registered"), "{message}");
    }
}
