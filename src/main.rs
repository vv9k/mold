use clap::Parser;
use colored::{Color, Colorize};
use mold::Mold;
use similar::ChangeTag;
use std::io;

#[derive(Debug, Parser)]
#[clap(version = "0.1.0", author = "Wojciech Kępka <wojciech@wkepka.dev>")]
struct Opts {
    #[clap(subcommand)]
    command: Subcommand,
}

#[derive(Debug, Parser)]
enum Subcommand {
    Render {
        input_file: std::path::PathBuf,
        output_path: Option<std::path::PathBuf>,
        #[clap(short, long)]
        namespace: Option<String>,
        #[clap(short, long)]
        context_file: Option<std::path::PathBuf>,
        #[clap(long)]
        /// By default, if there is no value for a variable name in the context nothing will
        /// be rendered in place. This option enables rendering of missing variables.
        show_missing: bool,
        #[clap(long)]
        no_diff: bool,
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

fn print_diff<W: io::Write>(writer: &mut W, a: &str, b: &str) -> io::Result<()> {
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

fn main() {
    macro_rules! exit {
        ($($t:tt)+) => {{
            eprintln!($($t)+);
            std::process::exit(1);
        }}
    }

    let opts = Opts::parse();

    match opts.command {
        Subcommand::Render {
            input_file: file,
            output_path,
            context_file,
            namespace,
            show_missing,
            no_diff,
        } => {
            let show_diff = !no_diff;
            let context_file = if let Some(context_file) = context_file {
                context_file
            } else if let Some(context_file) = dirs::config_dir().map(|dir| dir.join("mold.yaml")) {
                context_file
            } else {
                exit!("no context file found, exiting...");
            };

            let mold = match Mold::new(&context_file) {
                Ok(mold) => mold,
                Err(e) => exit!("failed to initialize mold - {}", e),
            };

            match mold.render_file(&file, namespace.as_deref(), show_missing) {
                Ok(rendered) => {
                    let len = file.to_string_lossy().len() + 6;
                    let line = std::iter::repeat("-").take(len).collect::<String>();
                    if let Some(output_path) = output_path.as_deref() {
                        if show_diff {
                            if let Ok(loaded) = std::fs::read_to_string(&output_path) {
                                println!("{:=^1$}", "=", 80);
                                println!("|{: ^1$}DIFF", " ", 37);
                                println!("| Template:  {}", file.to_string_lossy().bold());
                                println!("| Output:    {}", output_path.to_string_lossy().bold());
                                println!(
                                    "| Namespace: {}",
                                    namespace.as_deref().unwrap_or(mold::GLOBAL_NS).bold()
                                );
                                let _ = print_diff(&mut io::stdout(), &loaded, &rendered);
                            }
                        }
                        if let Err(e) = std::fs::write(&output_path, rendered.as_bytes()) {
                            eprintln!("failed to save rendered file `{}` - {}", file.display(), e);
                        }
                    } else {
                        println!("{:=^1$}", "=", 80);
                        println!("File: {}\n{}", file.display(), line);
                        println!("{}", rendered);
                    }
                }
                Err(e) => eprintln!("failed to render file `{}` - {}", file.display(), e),
            }
        }
    }
}
