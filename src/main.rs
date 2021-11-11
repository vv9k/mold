use clap::Parser;
use mold::Mold;

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
    },
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
        } => {
            let mold = Mold::default();

            let context_file = if let Some(context_file) = context_file {
                context_file
            } else if let Some(context_file) = dirs::config_dir().map(|dir| dir.join("mold.yaml")) {
                context_file
            } else {
                exit!("no context file found, exiting...");
            };

            let context = match mold.read_context(&context_file) {
                Ok(context) => context,
                Err(e) => exit!("failed to read context file - {:?}", e),
            };

            match mold.render_file(&file, &context, namespace.as_deref(), show_missing) {
                Ok(rendered) => {
                    let len = file.to_string_lossy().len() + 6;
                    let line = std::iter::repeat("-").take(len).collect::<String>();
                    if let Some(output_path) = output_path.as_deref() {
                        if let Err(e) = std::fs::write(&output_path, rendered.as_bytes()) {
                            eprintln!("failed to save rendered file `{}` - {}", file.display(), e);
                        }
                    } else {
                        println!("####################################################################################################");
                        println!("File: {}\n{}", file.display(), line);
                        println!("{}", rendered);
                    }
                }
                Err(e) => eprintln!("failed to render file `{}` - {}", file.display(), e),
            }
        }
    }
}
