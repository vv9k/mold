use clap::Parser;
use colored::{Color, Colorize};
use mold::Mold;
use similar::ChangeTag;
use std::io;
use std::path::{Path, PathBuf};

macro_rules! exit {
    ($($t:tt)+) => {{
        eprintln!($($t)+);
        std::process::exit(1);
    }}
}

#[derive(Debug, Parser)]
#[clap(version = "0.1.0", author = "Wojciech Kępka <wojciech@wkepka.dev>")]
/// A simple configuration template rendering program.
///
/// The main goal of mold is to allow users to easily switch configuration files between different
/// contexts. One example usage would be to have custom themes for multiple programs with one easy
/// way to switch all of their configuration at once.
///
/// The context file contains multiple namespaces as well as a global namespace. Each namespace can
/// have multiple key-value entries. Those variables can then be used in the templates like this:
/// {% variable1 %}. The name of the variable is enclosed in `{%` and `%}` with any amount of
/// whitespace in between allowed.
struct Opts {
    #[clap(subcommand)]
    command: Subcommand,
}

#[derive(Debug, Parser)]
enum Subcommand {
    /// Renders specified files with a given context.
    Render {
        /// Input files to render.
        templates: Vec<PathBuf>,
        #[clap(short, long)]
        /// Location of the context file to use for rendering.
        context_file: PathBuf,
        #[clap(short, long)]
        /// If specified the rendered content will be placed to this location, otherwise it will be
        /// printed to standard output.
        output_path: Option<PathBuf>,
        #[clap(short, long)]
        /// Specifies the namespace in the context to use for rendering. If not specified
        /// only GLOBAL namespace will be used.
        namespace: Option<String>,
        #[clap(long)]
        /// By default, if there is no value for a variable name in the context nothing will
        /// be rendered in place. This option enables rendering of missing variables.
        show_missing: bool,
        /// If true a diff of current file content and new rendered content will be displayed
        #[clap(long)]
        show_diff: bool,
        /// If true a header before each file will be printed
        #[clap(long)]
        show_headers: bool,
        /// If true no separator will be printed
        #[clap(long)]
        no_separator: bool,
    },
    /// Render specified context. If the context has no `renders` field this command has no effect.
    RenderContext {
        /// Location of the context file to use for rendering.
        context_file: PathBuf,
        #[clap(short, long)]
        /// Specifies the namespace in the context to use for rendering. If not specified
        /// only GLOBAL namespace will be used.
        namespace: Option<String>,
        #[clap(long)]
        /// By default, if there is no value for a variable name in the context nothing will
        /// be rendered in place. This option enables rendering of missing variables.
        show_missing: bool,
        #[clap(long)]
        /// If true a diff of current file content and new rendered content will be displayed
        show_diff: bool,
    },
    /// Prints a diff of current file content and newly rendered content.
    Diff {
        /// Template to render and diff.
        template: PathBuf,
        /// Location of the file to compare to.
        output_path: PathBuf,
        #[clap(short, long)]
        /// Location of the context file to use for diffing.
        context_file: PathBuf,
        #[clap(long)]
        /// By default, if there is no value for a variable name in the context nothing will
        /// be rendered in place. This option enables rendering of missing variables.
        show_missing: bool,
        #[clap(short, long)]
        /// Specifies the namespace in the context to use for rendering. If not specified
        /// only GLOBAL namespace will be used.
        namespace: Option<String>,
    },
}

struct Line(Option<usize>);

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

fn diff<W: io::Write>(writer: &mut W, a: &str, b: &str) -> io::Result<()> {
    let diff = similar::TextDiff::from_lines(a, b);
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            writer.write_all(format!("{:-^1$}\n", "-", 80).as_bytes())?;
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, sign_color) = match change.tag() {
                    ChangeTag::Delete => ("-".red(), Color::Red),
                    ChangeTag::Insert => ("+".green(), Color::Green),
                    ChangeTag::Equal => continue,
                };
                writer.write_all(
                    format!(
                        "{}{} |{}",
                        Line(change.old_index()),
                        Line(change.new_index()),
                        sign
                    )
                    .as_bytes(),
                )?;
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        writer.write_all(
                            format!("{}", value.color(sign_color).underline().on_black())
                                .as_bytes(),
                        )?;
                    } else {
                        writer.write_all(format!("{}", value.color(sign_color)).as_bytes())?;
                    }
                }
                if change.missing_newline() {
                    writer.write_all(b"\n")?;
                }
            }
        }
    }
    Ok(())
}

fn expand(path: &Path) -> PathBuf {
    PathBuf::from(shellexpand::tilde(&path.to_string_lossy()).to_string())
}

fn display_diff(template: &Path, output: &Path, namespace: Option<&str>, rendered: &str) {
    if let Ok(loaded) = std::fs::read_to_string(&output) {
        println!("{:=^1$}", "=", 80);
        println!("|{: ^1$}DIFF", " ", 37);
        println!("| Template:  {}", template.to_string_lossy().bold());
        println!("| Output:    {}", output.to_string_lossy().bold());
        println!(
            "| Namespace: {}",
            namespace.as_deref().unwrap_or(mold::GLOBAL_NS).bold()
        );
        let _ = diff(&mut io::stdout(), &loaded, rendered);
    }
}

fn diff_template(
    mold: &Mold,
    template: &Path,
    output_path: &Path,
    namespace: Option<&str>,
    show_missing: bool,
) {
    let template = expand(template);
    match mold.render_file(&template, namespace.as_deref(), show_missing) {
        Ok(rendered) => {
            let output_path = expand(output_path);
            display_diff(&template, &output_path, namespace, &rendered);
        }
        Err(e) => eprintln!("failed to render file `{}` - {:?}", template.display(), e),
    }
}

fn render_template(
    mold: &Mold,
    namespace: Option<&str>,
    template: &Path,
    output_path: Option<&Path>,
    show_diff: bool,
    show_missing: bool,
    show_headers: bool,
    show_separator: bool,
) {
    let template = expand(template);
    match mold.render_file(&template, namespace.as_deref(), show_missing) {
        Ok(rendered) => {
            let len = template.to_string_lossy().len() + 6;
            let line = "-".repeat(len);
            if let Some(output_path) = output_path.as_deref() {
                let output_path = expand(output_path);
                if show_diff {
                    display_diff(&template, &output_path, namespace, &rendered);
                }
                println!("saving {} to {}", template.display(), output_path.display());
                if let Err(e) = std::fs::write(&output_path, rendered.as_bytes()) {
                    eprintln!(
                        "failed to save rendered file `{}` to `{}` - {:?}",
                        template.display(),
                        output_path.display(),
                        e
                    );
                }
            } else {
                if show_separator {
                    println!("{:=^1$}", "=", 80);
                }
                if show_headers {
                    println!("File: {}\n{}", template.display(), line);
                }
                println!("{}", rendered);
            }
        }
        Err(e) => eprintln!("failed to render file `{}` - {:?}", template.display(), e),
    }
}

fn main() {
    let opts = Opts::parse();

    match opts.command {
        Subcommand::Render {
            context_file,
            templates,
            output_path,
            namespace,
            show_missing,
            show_diff,
            show_headers,
            no_separator,
        } => {
            let mold = match Mold::new(&context_file) {
                Ok(mold) => mold,
                Err(e) => exit!("failed to initialize mold - {:?}", e),
            };

            templates.into_iter().for_each(|template| {
                render_template(
                    &mold,
                    namespace.as_deref(),
                    &template,
                    output_path.as_deref(),
                    show_diff,
                    show_missing,
                    show_headers,
                    !no_separator,
                );
            });
        }
        Subcommand::RenderContext {
            context_file,
            namespace,
            show_missing,
            show_diff,
        } => {
            let mold = match Mold::new(&context_file) {
                Ok(mold) => mold,
                Err(e) => exit!("failed to initialize mold - {:?}", e),
            };

            for (template, output_path) in mold.context().renders() {
                render_template(
                    &mold,
                    namespace.as_deref(),
                    template,
                    Some(output_path),
                    show_diff,
                    show_missing,
                    false,
                    true,
                );
            }
        }
        Subcommand::Diff {
            context_file,
            template,
            output_path,
            namespace,
            show_missing,
        } => {
            let mold = match Mold::new(&context_file) {
                Ok(mold) => mold,
                Err(e) => exit!("failed to initialize mold - {:?}", e),
            };

            diff_template(
                &mold,
                &template,
                &output_path,
                namespace.as_deref(),
                show_missing,
            );
        }
    }
}
