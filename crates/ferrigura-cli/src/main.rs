//! `ferrigura`: renders Mermaid diagrams to SVG on the command line.
//!
//! The diagram texts come from files or from standard input. Each SVG goes to standard output, to the
//! file given with `--output` (one input), or to a file in the directory given with `--output-dir`,
//! named after its input. The options are the ones `ferrigura::RenderOptions` takes, and `--config` is
//! the configuration that `initialize` takes, as a JSON object. Several inputs are rendered through one
//! `ferrigura::Renderer`, so the configuration is resolved once.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use ferrigura::{Animation, ColorScheme, FontStyle, FontWeight, RenderOptions, Renderer};
use ferrigura_model::value::Value;

mod font_catalog;
mod protected;

/// Maximum size of a `--config` JSON file (2 MiB).
const CONFIG_LIMIT: u64 = 2 * 1024 * 1024;
/// Default `maxTextSize` in UTF-16 code units, matching `ferrigura::Renderer`.
const DEFAULT_MAX_TEXT_SIZE: u64 = 50_000;
/// Hard ceiling on diagram input bytes (64 MiB) to prevent OOM on huge files or streams.
const HARD_INPUT_LIMIT: u64 = 64 * 1024 * 1024;

// A render makes many small allocations, which mimalloc serves faster than the system allocator. The
// targets are the ones of the `mimalloc` dependency in Cargo.toml.
#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", target_env = "gnu"),
    all(target_os = "windows", target_env = "msvc")
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const USAGE: &str = "\
Renders Mermaid diagrams to SVG.

Usage: ferrigura [OPTIONS] [FILE]...
       ferrigura fonts fetch [OPTIONS] FAMILY...

Arguments:
  [FILE]...  The diagram texts. Standard input is read when no FILE is given, or for `-`. Several
             files are rendered through one renderer, in the order given. Without --output or
             --output-dir, their SVGs follow each other on standard output.

Options:
  -c, --config <FILE>     The configuration as a JSON object, what `initialize` takes
      --font-lock <FILE>  Register the verified faces from a local font lock
      --font <FAMILY=FILE> Register a local font as weight 400, normal; repeat for more families
      --font-face <FAMILY:WEIGHT:STYLE=FILE>
                          Register a face with weight 1..1000 and style normal or italic
      --font-fallback <FAMILY>
                          Add a registered fallback family, in order; repeat for more families
      --now <MILLISECONDS> Fixed UTC time as integer milliseconds since the Unix epoch
      --id <ID>           The id of the SVG element, which scopes its styles
      --scheme <SCHEME>   The colours: `light`, `dark`, or `auto` to follow the viewer's
                          colour scheme. By default the configured theme decides.
      --compat           Use Mermaid geometry instead of optimized routing and participant boxes
      --design           Draw flowcharts, state, class and ER diagrams, mindmaps, and pie,
                          XY, quadrant and gantt charts in the design mode: ferrigura's own look
      --animate           Draw a supported diagram once, one line at a time, with embedded CSS
      --animation-speed <N>
                          Positive multiplier of 700 SVG units/s on average; enables animation
      --animation-order <ORDER>
                          `source` (default) or `flow` for parallel branches; enables animation
  -o, --output <FILE>     Write the SVG to FILE instead of standard output (one input only)
      --output-dir <DIR>  Write each SVG to DIR/<name>.svg, named after its input file
                          (file inputs only)
  -h, --help              Print this help
  -V, --version           Print the version
";

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Version,
    FontFetch(Vec<String>),
    Render(Box<Job>),
}

/// A render: the inputs, the options, and where the SVGs go.
#[derive(Debug, Default, PartialEq, Eq)]
struct Job {
    /// The inputs in the order given; standard input when none is.
    inputs: Vec<Input>,
    config: Option<PathBuf>,
    fonts: Vec<FontInput>,
    font_lock: Option<PathBuf>,
    font_fallbacks: Vec<String>,
    id: Option<String>,
    now: Option<i64>,
    scheme: Option<ColorScheme>,
    compat: bool,
    design: bool,
    animation: Option<Animation>,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
}

/// One explicitly named local face, read before diagram input or output.
#[derive(Debug, PartialEq, Eq)]
struct FontInput {
    family: String,
    path: PathBuf,
    weight: FontWeight,
    style: FontStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Input {
    Stdin,
    File(PathBuf),
}

impl Input {
    /// The name of the input in messages.
    fn name(&self) -> String {
        match self {
            Self::Stdin => "-".to_owned(),
            Self::File(path) => path.display().to_string(),
        }
    }
}

fn main() -> ExitCode {
    match parse_args(std::env::args().skip(1)) {
        Ok(Command::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("ferrigura {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::FontFetch(args)) => match font_catalog::fetch(args.into_iter()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("ferrigura: {message}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Render(job)) => match run(&job) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("ferrigura: {message}");
                ExitCode::FAILURE
            }
        },
        Err(message) => {
            eprintln!("ferrigura: {message}\nTry 'ferrigura --help' for more information.");
            ExitCode::from(2)
        }
    }
}

/// Reads the command line. Help and version end the parse at the option that asks for them.
fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut job = Job::default();
    let values: Vec<String> = args.into_iter().collect();
    if values.first().is_some_and(|arg| arg == "fonts")
        && values.get(1).is_some_and(|arg| arg == "fetch")
    {
        return Ok(Command::FontFetch(values.into_iter().skip(2).collect()));
    }
    let mut args = values.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "-c" | "--config" => job.config = Some(value(&mut args, &argument)?.into()),
            "--font-lock" => job.font_lock = Some(value(&mut args, &argument)?.into()),
            "--font" | "--font-face" => {
                let raw = value(&mut args, &argument)?;
                job.fonts.push(font_input(&raw, argument == "--font-face")?);
            }
            "--font-fallback" => {
                let family = value(&mut args, &argument)?;
                if family.trim().is_empty() {
                    return Err("--font-fallback needs a family name".to_owned());
                }
                job.font_fallbacks.push(family);
            }
            "--now" => {
                let raw = value(&mut args, &argument)?;
                job.now = Some(
                    raw.parse::<i64>()
                        .ok()
                        .filter(|n| (-8_640_000_000_000_000..=8_640_000_000_000_000).contains(n))
                        .ok_or_else(|| {
                            "--now must be integer Unix milliseconds within ±8640000000000000"
                                .to_owned()
                        })?,
                );
            }
            "--id" => job.id = Some(value(&mut args, &argument)?),
            "--scheme" => job.scheme = Some(scheme(&value(&mut args, &argument)?)?),
            "--compat" => job.compat = true,
            "--design" => job.design = true,
            "--animate" => {
                job.animation.get_or_insert_default();
            }
            "--animation-speed" => {
                let raw = value(&mut args, &argument)?;
                let speed = raw
                    .parse()
                    .map_err(|_| format!("invalid animation speed {raw:?}"))?;
                let order = job.animation.unwrap_or_default().order;
                let mut animation = Animation::new(speed).map_err(|error| error.to_string())?;
                animation.order = order;
                job.animation = Some(animation);
            }
            "--animation-order" => {
                job.animation.get_or_insert_default().order =
                    value(&mut args, &argument)?.parse()?;
            }
            "-o" | "--output" => job.output = Some(value(&mut args, &argument)?.into()),
            "--output-dir" => job.output_dir = Some(value(&mut args, &argument)?.into()),
            "--" => {
                for file in args {
                    job.inputs.push(Input::File(file.into()));
                }
                break;
            }
            "-" => job.inputs.push(Input::Stdin),
            other if other.starts_with('-') => return Err(format!("unknown option {other}")),
            file => job.inputs.push(Input::File(file.into())),
        }
    }
    job.settle()
}

/// Output file key for `--output-dir` collision checks: case-folded on case-insensitive platforms
/// (macOS/Windows) and exact on case-sensitive platforms.
#[derive(Debug, PartialEq, Eq, Hash)]
enum TargetKey {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    Folded(String),
    Raw(OsString),
}

impl TargetKey {
    fn from_path(path: &Path) -> Self {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            return Self::Folded(name.to_lowercase());
        }
        Self::Raw(path.file_name().map_or_else(
            || path.as_os_str().to_os_string(),
            std::ffi::OsStr::to_os_string,
        ))
    }
}

impl Job {
    /// Checks what the options mean together, before anything is read or written: the inputs default
    /// to standard input, standard input is given once, and the SVGs of several inputs have different
    /// names.
    fn settle(mut self) -> Result<Command, String> {
        if self.inputs.is_empty() {
            self.inputs.push(Input::Stdin);
        }
        if self
            .inputs
            .iter()
            .filter(|input| **input == Input::Stdin)
            .count()
            > 1
        {
            return Err("standard input can be given only once".to_owned());
        }
        if self.output.is_some() && self.output_dir.is_some() {
            return Err("--output and --output-dir cannot be given together".to_owned());
        }
        if self.output.is_some() && self.inputs.len() > 1 {
            return Err("--output takes one input; use --output-dir for several".to_owned());
        }
        if let Some(dir) = &self.output_dir {
            let mut targets: HashMap<TargetKey, PathBuf> = HashMap::new();
            for input in &self.inputs {
                let target = svg_path(dir, input)?;
                let key = TargetKey::from_path(&target);
                if let Some(previous) = targets.insert(key, target.clone()) {
                    if previous == target {
                        return Err(format!("{} would be written twice", target.display()));
                    }
                    return Err(format!(
                        "{} collides with {} (names differ only in case)",
                        target.display(),
                        previous.display()
                    ));
                }
            }
        }
        Ok(Command::Render(Box::new(self)))
    }
}

fn value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("{option} needs a value"))
}

fn font_input(raw: &str, explicit_face: bool) -> Result<FontInput, String> {
    let (specification, path) = raw
        .split_once('=')
        .ok_or_else(|| "a font needs FAMILY=FILE or FAMILY:WEIGHT:STYLE=FILE".to_owned())?;
    if path.is_empty() {
        return Err("a font needs a file path".to_owned());
    }
    let (family, weight, style) = if explicit_face {
        let (family_weight, style) = specification
            .rsplit_once(':')
            .ok_or_else(|| "--font-face needs FAMILY:WEIGHT:STYLE=FILE".to_owned())?;
        let (family, weight) = family_weight
            .rsplit_once(':')
            .ok_or_else(|| "--font-face needs FAMILY:WEIGHT:STYLE=FILE".to_owned())?;
        let weight = weight
            .parse::<u16>()
            .ok()
            .filter(|weight| (1..=1000).contains(weight))
            .ok_or_else(|| "font weight must be an integer from 1 through 1000".to_owned())?;
        let style = match style {
            "normal" => FontStyle::Normal,
            "italic" => FontStyle::Italic,
            _ => return Err("font style must be normal or italic".to_owned()),
        };
        let weight = match weight {
            400 => FontWeight::Normal,
            700 => FontWeight::Bold,
            weight => FontWeight::Number(weight),
        };
        (family, weight, style)
    } else {
        (specification, FontWeight::Normal, FontStyle::Normal)
    };
    if family.trim().is_empty() {
        return Err("a font needs a family name".to_owned());
    }
    Ok(FontInput {
        family: family.to_owned(),
        path: path.into(),
        weight,
        style,
    })
}

fn scheme(name: &str) -> Result<ColorScheme, String> {
    match name {
        "light" => Ok(ColorScheme::Light),
        "dark" => Ok(ColorScheme::Dark),
        "auto" => Ok(ColorScheme::Auto),
        other => Err(format!(
            "unknown colour scheme {other}, expected light, dark or auto"
        )),
    }
}

fn run(job: &Job) -> Result<(), String> {
    let config = match &job.config {
        Some(path) => read_config(path)?,
        None => Value::empty_object(),
    };
    let input_limit = diagram_input_limit(&config);
    let mut builder = Renderer::builder(&config).with_all();
    let mut fallback_families = Vec::new();
    if let Some(path) = &job.font_lock {
        let (fonts, fallbacks) = font_catalog::read_lock(path)?;
        for (family, data, weight, style) in fonts {
            builder = builder
                .font_face_shared(&family, data, weight, style)
                .map_err(|error| format!("cannot register font {family:?}: {error}"))?;
        }
        fallback_families = fallbacks;
    }
    let mut loaded_fonts: HashMap<PathBuf, Arc<Vec<u8>>> = HashMap::new();
    for font in &job.fonts {
        let data = if let Some(existing) = loaded_fonts.get(&font.path) {
            Arc::clone(existing)
        } else {
            let bytes = Arc::new(read_font_file(&font.path)?);
            loaded_fonts.insert(font.path.clone(), Arc::clone(&bytes));
            bytes
        };
        builder = builder
            .font_face_shared(&font.family, data, font.weight, font.style)
            .map_err(|error| format!("cannot register font {:?}: {error}", font.family))?;
    }
    fallback_families.extend(job.font_fallbacks.iter().cloned());
    if !fallback_families.is_empty() {
        builder = builder.font_fallbacks(&fallback_families);
    }
    let renderer = builder.build();
    let options = RenderOptions {
        id: job.id.clone(),
        color_scheme: job.scheme,
        compat: job.compat,
        design: job.design,
        animation: job.animation,
        now: job.now,
    };
    if let Some(dir) = &job.output_dir {
        fs::create_dir_all(dir)
            .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    }
    let several = job.inputs.len() > 1;
    #[cfg(unix)]
    let mut written_inodes: HashMap<(u64, u64), PathBuf> = HashMap::new();
    for input in &job.inputs {
        #[cfg(unix)]
        let result = render_input(
            &renderer,
            input,
            &options,
            job,
            input_limit,
            &mut written_inodes,
        );
        #[cfg(not(unix))]
        let result = render_input(&renderer, input, &options, job, input_limit);
        result.map_err(|message| {
            if several {
                format!("{}: {message}", input.name())
            } else {
                message
            }
        })?;
    }
    Ok(())
}

/// Computes a safe UTF-8 byte ceiling for diagram input from `maxTextSize` (each UTF-16 code unit
/// is at most 3 UTF-8 bytes), bounded by [`HARD_INPUT_LIMIT`].
fn diagram_input_limit(config: &Value) -> u64 {
    let max_text = match config.get("maxTextSize") {
        Some(Value::Number(n))
            if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 && *n <= 9_007_199_254_740_991.0 =>
        {
            *n as u64
        }
        _ => DEFAULT_MAX_TEXT_SIZE,
    };
    max_text.saturating_mul(3).min(HARD_INPUT_LIMIT)
}

/// Renders one input, and writes its SVG where the job says.
fn render_input(
    renderer: &Renderer,
    input: &Input,
    options: &RenderOptions,
    job: &Job,
    limit: u64,
    #[cfg(unix)] written_inodes: &mut HashMap<(u64, u64), PathBuf>,
) -> Result<(), String> {
    let text = match input {
        Input::Stdin => read_stdin(limit)?,
        Input::File(path) => read_text_file(path, limit)?,
    };
    let svg = renderer
        .render_with(&text, options)
        .map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        write_svg(job, input, &svg, written_inodes)
    }
    #[cfg(not(unix))]
    {
        write_svg(job, input, &svg)
    }
}

/// Writes an SVG to `--output`, into `--output-dir`, or to standard output.
fn write_svg(
    job: &Job,
    input: &Input,
    svg: &str,
    #[cfg(unix)] written_inodes: &mut HashMap<(u64, u64), PathBuf>,
) -> Result<(), String> {
    if let Some(path) = &job.output {
        return write_svg_file(path, svg);
    }
    if let Some(dir) = &job.output_dir {
        let path = svg_path(dir, input)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if let Ok(meta) = fs::metadata(&path)
                && let Some(previous) = written_inodes.get(&(meta.dev(), meta.ino()))
            {
                return Err(format!(
                    "{} collides with {} on this file system",
                    path.display(),
                    previous.display()
                ));
            }
            let file = write_svg_file_returning(&path, svg)?;
            if let Ok(meta) = file.metadata() {
                written_inodes.insert((meta.dev(), meta.ino()), path);
            }
            return Ok(());
        }
        #[cfg(not(unix))]
        return write_svg_file(&path, svg);
    }
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(svg.as_bytes())
        .and_then(|()| stdout.write_all(b"\n"))
        .map_err(|error| format!("cannot write standard output: {error}"))
}

fn write_svg_file(path: &Path, svg: &str) -> Result<(), String> {
    write_svg_file_returning(path, svg).map(|_| ())
}

fn write_svg_file_returning(path: &Path, svg: &str) -> Result<fs::File, String> {
    fs::File::create(path)
        .and_then(|mut file| {
            file.write_all(svg.as_bytes())?;
            file.write_all(b"\n")?;
            Ok(file)
        })
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// The file of an input's SVG in the output directory: the input's name, with the extension `svg`.
fn svg_path(dir: &Path, input: &Input) -> Result<PathBuf, String> {
    let Input::File(path) = input else {
        return Err("--output-dir needs file inputs, as standard input has no name".to_owned());
    };
    let name = path
        .file_name()
        .ok_or_else(|| format!("{} has no name for its SVG", path.display()))?;
    Ok(dir.join(Path::new(name).with_extension("svg")))
}

fn read_stdin(limit: u64) -> Result<String, String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read standard input: {error}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "standard input exceeds size limit of {limit} bytes"
        ));
    }
    String::from_utf8(bytes).map_err(|error| format!("cannot read standard input: {error}"))
}

fn read_text_file(path: &Path, limit: u64) -> Result<String, String> {
    let file =
        fs::File::open(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if metadata.is_file() && metadata.len() > limit {
        return Err(format!(
            "{} exceeds size limit of {limit} bytes",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "{} exceeds size limit of {limit} bytes",
            path.display()
        ));
    }
    String::from_utf8(bytes).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn read_font_file(path: &Path) -> Result<Vec<u8>, String> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|error| format!("cannot read font {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("cannot read font {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "cannot read font {}: not a regular file",
            path.display()
        ));
    }
    if metadata.len() > font_catalog::LIMIT {
        return Err(format!(
            "cannot read font {}: exceeds size limit",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    file.take(font_catalog::LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read font {}: {error}", path.display()))?;
    if bytes.len() as u64 > font_catalog::LIMIT {
        return Err(format!(
            "cannot read font {}: exceeds size limit",
            path.display()
        ));
    }
    Ok(bytes)
}

fn read_config(path: &Path) -> Result<Value, String> {
    let text = read_text_file(path, CONFIG_LIMIT)?;
    let config: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    if !config.is_object() {
        return Err(format!(
            "{}: the configuration must be a JSON object",
            path.display()
        ));
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|argument| (*argument).to_owned()).collect()
    }

    fn job(list: &[&str]) -> Job {
        match parse_args(args(list)) {
            Ok(Command::Render(job)) => *job,
            other => panic!("expected a render, got {other:?}"),
        }
    }

    #[test]
    fn font_options_preserve_spaces_paths_and_fallback_order() {
        let parsed = job(&[
            "--font",
            "Project Sans=C:\\fonts\\regular.ttf",
            "--font-face",
            "Project Sans:600:italic=C:\\fonts\\italic.ttf",
            "--font-fallback",
            "Symbols One",
            "--font-fallback",
            "Symbols Two",
        ]);
        assert_eq!(parsed.fonts.len(), 2);
        assert_eq!(parsed.fonts[0].family, "Project Sans");
        assert_eq!(
            parsed.fonts[0].path,
            PathBuf::from("C:\\fonts\\regular.ttf")
        );
        assert_eq!(parsed.fonts[0].weight.css_value(), 400);
        assert_eq!(parsed.fonts[1].weight.css_value(), 600);
        assert_eq!(parsed.fonts[1].style, FontStyle::Italic);
        assert_eq!(parsed.font_fallbacks, ["Symbols One", "Symbols Two"]);
    }

    #[test]
    fn no_arguments_render_standard_input_with_the_defaults() {
        assert_eq!(
            job(&[]),
            Job {
                inputs: vec![Input::Stdin],
                ..Job::default()
            }
        );
    }

    #[test]
    fn a_dash_is_standard_input() {
        assert_eq!(job(&["-"]).inputs, vec![Input::Stdin]);
    }

    #[test]
    fn a_file_is_the_input() {
        assert_eq!(
            job(&["pets.mmd"]).inputs,
            vec![Input::File(PathBuf::from("pets.mmd"))]
        );
    }

    #[test]
    fn several_inputs_are_rendered_in_the_order_given() {
        assert_eq!(
            job(&["b.mmd", "-", "a.mmd"]).inputs,
            vec![
                Input::File(PathBuf::from("b.mmd")),
                Input::Stdin,
                Input::File(PathBuf::from("a.mmd")),
            ]
        );
    }

    #[test]
    fn options_are_read_in_both_spellings() {
        let short = job(&[
            "-c",
            "config.json",
            "--id",
            "pie-one",
            "--scheme",
            "auto",
            "-o",
            "out.svg",
        ]);
        assert_eq!(short.config, Some(PathBuf::from("config.json")));
        assert_eq!(short.id.as_deref(), Some("pie-one"));
        assert_eq!(short.scheme, Some(ColorScheme::Auto));
        assert_eq!(short.output, Some(PathBuf::from("out.svg")));
        let long = job(&["--config", "c.json", "--output", "o.svg"]);
        assert_eq!(long.output, Some(PathBuf::from("o.svg")));
    }

    #[test]
    fn the_color_schemes_are_light_dark_and_auto() {
        assert_eq!(job(&["--scheme", "light"]).scheme, Some(ColorScheme::Light));
        assert_eq!(job(&["--scheme", "dark"]).scheme, Some(ColorScheme::Dark));
        assert!(parse_args(args(&["--scheme", "blue"])).is_err());
    }

    #[test]
    fn help_and_version_end_the_parse() {
        assert_eq!(parse_args(args(&["--help", "--bogus"])), Ok(Command::Help));
        assert_eq!(parse_args(args(&["-V"])), Ok(Command::Version));
    }

    #[test]
    fn an_unknown_option_is_refused() {
        let error = parse_args(args(&["--colour", "red"])).unwrap_err();
        assert_eq!(error, "unknown option --colour");
    }

    #[test]
    fn an_option_without_its_value_is_refused() {
        assert_eq!(
            parse_args(args(&["--id"])).unwrap_err(),
            "--id needs a value"
        );
    }

    #[test]
    fn output_takes_one_input() {
        assert_eq!(
            parse_args(args(&["-o", "out.svg", "a.mmd", "b.mmd"])).unwrap_err(),
            "--output takes one input; use --output-dir for several"
        );
    }

    #[test]
    fn output_and_output_dir_are_not_given_together() {
        assert_eq!(
            parse_args(args(&["-o", "out.svg", "--output-dir", "out", "a.mmd"])).unwrap_err(),
            "--output and --output-dir cannot be given together"
        );
    }

    #[test]
    fn standard_input_is_given_once() {
        assert_eq!(
            parse_args(args(&["-", "-"])).unwrap_err(),
            "standard input can be given only once"
        );
    }

    #[test]
    fn each_svg_of_an_output_dir_is_named_after_its_input() {
        let job = job(&["--output-dir", "out", "pets/cats.mmd", "dogs.mmd"]);
        let dir = Path::new("out");
        assert_eq!(
            svg_path(dir, &job.inputs[0]),
            Ok(PathBuf::from("out/cats.svg"))
        );
        assert_eq!(
            svg_path(dir, &job.inputs[1]),
            Ok(PathBuf::from("out/dogs.svg"))
        );
    }

    #[test]
    fn an_output_dir_needs_file_inputs_with_distinct_names() {
        assert!(parse_args(args(&["--output-dir", "out"])).is_err());
        assert_eq!(
            parse_args(args(&["--output-dir", "out", "a/pets.mmd", "b/pets.mmd"])).unwrap_err(),
            "out/pets.svg would be written twice"
        );
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let case_error =
                parse_args(args(&["--output-dir", "out", "a/Pets.mmd", "b/pets.mmd"])).unwrap_err();
            assert!(case_error.contains("collides with"), "{case_error}");
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            assert!(parse_args(args(&["--output-dir", "out", "a/Pets.mmd", "b/pets.mmd"])).is_ok());
        }
    }

    #[test]
    fn double_dash_terminates_options_for_dash_prefixed_filenames() {
        let parsed = job(&["--id", "dash", "--", "-diagram.mmd", "--help", "-"]);
        assert_eq!(parsed.id.as_deref(), Some("dash"));
        assert_eq!(
            parsed.inputs,
            vec![
                Input::File(PathBuf::from("-diagram.mmd")),
                Input::File(PathBuf::from("--help")),
                Input::File(PathBuf::from("-")),
            ]
        );
    }

    #[test]
    fn diagram_input_limit_tracks_max_text_size_with_hard_ceiling() {
        assert_eq!(
            diagram_input_limit(&Value::empty_object()),
            DEFAULT_MAX_TEXT_SIZE * 3
        );
        let larger: Value = serde_json::from_str(r#"{"maxTextSize": 200000}"#).unwrap();
        assert_eq!(diagram_input_limit(&larger), 600_000);
        let huge: Value = serde_json::from_str(r#"{"maxTextSize": 100000000}"#).unwrap();
        assert_eq!(diagram_input_limit(&huge), HARD_INPUT_LIMIT);
        let invalid: Value = serde_json::from_str(r#"{"maxTextSize": -1}"#).unwrap();
        assert_eq!(diagram_input_limit(&invalid), DEFAULT_MAX_TEXT_SIZE * 3);
    }
}
