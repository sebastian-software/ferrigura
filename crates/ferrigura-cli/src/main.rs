//! `ferrigura`: renders Mermaid diagrams to SVG on the command line.
//!
//! The diagram texts come from files or from standard input. Each SVG goes to standard output, to the
//! file given with `--output` (one input), or to a file in the directory given with `--output-dir`,
//! named after its input. The options are the ones `ferrigura::RenderOptions` takes, and `--config` is
//! the configuration that `initialize` takes, as a JSON object. Several inputs are rendered through one
//! `ferrigura::Renderer`, so the configuration is resolved once.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ferrigura::{ColorScheme, RenderOptions, Renderer};
use ferrigura_model::value::Value;

const USAGE: &str = "\
Renders Mermaid diagrams to SVG.

Usage: ferrigura [OPTIONS] [FILE]...

Arguments:
  [FILE]...  The diagram texts. Standard input is read when no FILE is given, or for `-`. Several
             files are rendered through one renderer, in the order given. Without --output or
             --output-dir, their SVGs follow each other on standard output.

Options:
  -c, --config <FILE>     The configuration as a JSON object, what `initialize` takes
      --id <ID>           The id of the SVG element, which scopes its styles
      --scheme <SCHEME>   The colours: `light`, `dark`, or `auto` to follow the viewer's
                          colour scheme. By default the configured theme decides.
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
    Render(Job),
}

/// A render: the inputs, the options, and where the SVGs go.
#[derive(Debug, Default, PartialEq, Eq)]
struct Job {
    /// The inputs in the order given; standard input when none is.
    inputs: Vec<Input>,
    config: Option<PathBuf>,
    id: Option<String>,
    scheme: Option<ColorScheme>,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
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
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "-c" | "--config" => job.config = Some(value(&mut args, &argument)?.into()),
            "--id" => job.id = Some(value(&mut args, &argument)?),
            "--scheme" => job.scheme = Some(scheme(&value(&mut args, &argument)?)?),
            "-o" | "--output" => job.output = Some(value(&mut args, &argument)?.into()),
            "--output-dir" => job.output_dir = Some(value(&mut args, &argument)?.into()),
            "-" => job.inputs.push(Input::Stdin),
            other if other.starts_with('-') => return Err(format!("unknown option {other}")),
            file => job.inputs.push(Input::File(file.into())),
        }
    }
    job.settle()
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
            let mut targets = HashSet::new();
            for input in &self.inputs {
                let target = svg_path(dir, input)?;
                if !targets.insert(target.clone()) {
                    return Err(format!("{} would be written twice", target.display()));
                }
            }
        }
        Ok(Command::Render(self))
    }
}

fn value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("{option} needs a value"))
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
    let renderer = Renderer::new(&config);
    let options = RenderOptions {
        id: job.id.clone(),
        color_scheme: job.scheme,
        now: None,
        git_graph_seed: None,
        random_seed: None,
    };
    if let Some(dir) = &job.output_dir {
        fs::create_dir_all(dir)
            .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    }
    let several = job.inputs.len() > 1;
    for input in &job.inputs {
        render_input(&renderer, input, &options, job).map_err(|message| {
            if several {
                format!("{}: {message}", input.name())
            } else {
                message
            }
        })?;
    }
    Ok(())
}

/// Renders one input, and writes its SVG where the job says.
fn render_input(
    renderer: &Renderer,
    input: &Input,
    options: &RenderOptions,
    job: &Job,
) -> Result<(), String> {
    let text = match input {
        Input::Stdin => {
            let mut text = String::new();
            io::stdin()
                .read_to_string(&mut text)
                .map_err(|error| format!("cannot read standard input: {error}"))?;
            text
        }
        Input::File(path) => read(path)?,
    };
    let svg = renderer
        .render_with(&text, options)
        .map_err(|error| error.to_string())?;
    write_svg(job, input, &svg)
}

/// Writes an SVG to `--output`, into `--output-dir`, or to standard output.
fn write_svg(job: &Job, input: &Input, svg: &str) -> Result<(), String> {
    if let Some(path) = &job.output {
        return fs::write(path, format!("{svg}\n"))
            .map_err(|error| format!("cannot write {}: {error}", path.display()));
    }
    if let Some(dir) = &job.output_dir {
        let path = svg_path(dir, input)?;
        return fs::write(&path, format!("{svg}\n"))
            .map_err(|error| format!("cannot write {}: {error}", path.display()));
    }
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(svg.as_bytes())
        .and_then(|()| stdout.write_all(b"\n"))
        .map_err(|error| format!("cannot write standard output: {error}"))
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

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn read_config(path: &Path) -> Result<Value, String> {
    let json: serde_json::Value = serde_json::from_str(&read(path)?)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if !json.is_object() {
        return Err(format!(
            "{}: the configuration must be a JSON object",
            path.display()
        ));
    }
    Ok(Value::from(json))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|argument| (*argument).to_owned()).collect()
    }

    fn job(list: &[&str]) -> Job {
        match parse_args(args(list)) {
            Ok(Command::Render(job)) => job,
            other => panic!("expected a render, got {other:?}"),
        }
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
    fn the_colour_schemes_are_light_dark_and_auto() {
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
    }
}
