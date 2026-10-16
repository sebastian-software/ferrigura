//! `ferrigura`: renders a Mermaid diagram to SVG on the command line.
//!
//! The diagram text comes from a file or from standard input, and the SVG goes to standard output
//! or to the file given with `--output`. The options are the ones `ferrigura::render_with` takes,
//! and `--config` is the configuration that `initialize` takes, as a JSON object.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ferrigura::{ColorScheme, RenderOptions};
use ferrigura_model::value::Value;

const USAGE: &str = "\
Renders a Mermaid diagram to SVG.

Usage: ferrigura [OPTIONS] [FILE]

Arguments:
  [FILE]  The diagram text. Standard input is read when FILE is omitted or is `-`.

Options:
  -c, --config <FILE>    The configuration as a JSON object, what `initialize` takes
      --id <ID>          The id of the SVG element, which scopes its styles
      --scheme <SCHEME>  The colours: `light`, `dark`, or `auto` to follow the viewer's
                         colour scheme. By default the configured theme decides.
  -o, --output <FILE>    Write the SVG to FILE instead of standard output
  -h, --help             Print this help
  -V, --version          Print the version
";

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Help,
    Version,
    Render(Job),
}

/// A render: where the text comes from, the options, and where the SVG goes.
#[derive(Debug, Default, PartialEq, Eq)]
struct Job {
    input: Input,
    config: Option<PathBuf>,
    id: Option<String>,
    scheme: Option<ColorScheme>,
    output: Option<PathBuf>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum Input {
    #[default]
    Stdin,
    File(PathBuf),
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
    let mut inputs = 0;
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "-c" | "--config" => job.config = Some(value(&mut args, &argument)?.into()),
            "--id" => job.id = Some(value(&mut args, &argument)?),
            "--scheme" => job.scheme = Some(scheme(&value(&mut args, &argument)?)?),
            "-o" | "--output" => job.output = Some(value(&mut args, &argument)?.into()),
            "-" => {
                inputs += 1;
                job.input = Input::Stdin;
            }
            other if other.starts_with('-') => return Err(format!("unknown option {other}")),
            file => {
                inputs += 1;
                job.input = Input::File(file.into());
            }
        }
        if inputs > 1 {
            return Err("only one input can be given".to_owned());
        }
    }
    Ok(Command::Render(job))
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
    let text = match &job.input {
        Input::Stdin => {
            let mut text = String::new();
            io::stdin()
                .read_to_string(&mut text)
                .map_err(|error| format!("cannot read standard input: {error}"))?;
            text
        }
        Input::File(path) => read(path)?,
    };
    let config = match &job.config {
        Some(path) => read_config(path)?,
        None => Value::empty_object(),
    };
    let options = RenderOptions {
        id: job.id.clone(),
        color_scheme: job.scheme,
        now: None,
    };
    let svg =
        ferrigura::render_with(&text, &config, &options).map_err(|error| error.to_string())?;

    match &job.output {
        Some(path) => fs::write(path, format!("{svg}\n"))
            .map_err(|error| format!("cannot write {}: {error}", path.display())),
        None => {
            let mut stdout = io::stdout().lock();
            stdout
                .write_all(svg.as_bytes())
                .and_then(|()| stdout.write_all(b"\n"))
                .map_err(|error| format!("cannot write standard output: {error}"))
        }
    }
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
        assert_eq!(job(&[]), Job::default());
    }

    #[test]
    fn a_dash_is_standard_input() {
        assert_eq!(job(&["-"]).input, Input::Stdin);
    }

    #[test]
    fn a_file_is_the_input() {
        assert_eq!(
            job(&["pets.mmd"]).input,
            Input::File(PathBuf::from("pets.mmd"))
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
    fn only_one_input_is_accepted() {
        assert_eq!(
            parse_args(args(&["a.mmd", "b.mmd"])).unwrap_err(),
            "only one input can be given"
        );
        assert!(parse_args(args(&["-", "a.mmd"])).is_err());
    }
}
