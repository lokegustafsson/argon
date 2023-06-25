#[cfg(not(target_feature = "avx2"))]
compile_error!("unexpectedly missing required feature AVX2");

use simd_json::value::borrowed::Value;
use std::{
    cell::RefCell,
    fmt, fs,
    io::{self, BufWriter, Read, StdoutLock},
    mem::{self, ManuallyDrop},
    path::Path,
    process::ExitCode,
};
use tracing_subscriber::{filter::targets::Targets, layer::Layer};
use url::Url;

#[derive(clap::Parser, Debug)]
#[command(about, verbatim_doc_comment)]
/// Example invocations:
/// - `echo '[1,"abc\r\ncba]' | argon`
/// - `argon path/to/something.json`
/// - `argon https://api.github.com/repos/lokegustafsson/argon/commits?per_page=1`
struct Args {
    /// Filesystem path or URL to the json file to process.
    path_or_url_to_json: Option<String>,
    #[arg(short, long)]
    verbose: bool,
    #[arg(short, long)]
    color: bool,
    #[arg(short, long)]
    no_color: bool,
}

const ANSI_KEY: &str = "\x1B[34m";
const ANSI_NUM: &str = "\x1B[31m";
const ANSI_STR: &str = "\x1B[32m";
const ANSI_BRACE: &str = "\x1B[35m";
const ANSI_RESET: &str = "\x1B[0m";

fn main() -> ExitCode {
    match main_impl() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
fn main_impl() -> Result<(), ()> {
    let args: Args = clap::Parser::parse();
    setup_logging(args.verbose);

    let mut buf = if let Some(path_or_url_to_json) = &args.path_or_url_to_json {
        if let Ok(url_to_json) = Url::parse(path_or_url_to_json) {
            from_url(url_to_json)?
        } else {
            from_file(Path::new(path_or_url_to_json))?
        }
    } else {
        let mut buf = Vec::new();
        io::stdin().lock().read_to_end(&mut buf).unwrap();
        buf
    };

    let json = match simd_json::value::borrowed::to_value(&mut buf) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(?err, "could not parse json");
            return Err(());
        }
    };

    let have_color = match (args.color, args.no_color, atty::is(atty::Stream::Stdout)) {
        (true, false, _) => true,
        (false, true, _) => false,
        (_, _, tty) => tty,
    };
    LOCALS.with(|cell| *cell.borrow_mut() = Some(Locals::new(have_color)));
    if have_color {
        process_recursively::<true>(&json);
    } else {
        process_recursively::<false>(&json);
    }

    // Leak `json` and `buf` for quicker exit
    let _ = ManuallyDrop::new(json);
    let _ = ManuallyDrop::new(buf);
    Ok(())
}

fn setup_logging(verbose: bool) {
    tracing::subscriber::set_global_default(
        Targets::new()
            .with_target("h2", tracing::Level::INFO)
            .with_target(
                "hyper::client",
                if verbose {
                    tracing::Level::DEBUG
                } else {
                    tracing::Level::INFO
                },
            )
            .with_target("tokio_util::codec::framed_impl", tracing::Level::DEBUG)
            .with_default(tracing::Level::TRACE)
            .with_subscriber(
                tracing_subscriber::FmtSubscriber::builder()
                    .with_max_level(tracing::Level::TRACE)
                    .finish(),
            ),
    )
    .expect("enabling global logger");

    std::panic::set_hook(Box::new(|info| {
        let payload = info.payload();
        let msg = Option::or(
            payload.downcast_ref::<&str>().copied(),
            payload.downcast_ref::<String>().map(|s| &**s),
        )
        .unwrap_or("<no message>");
        let location = info
            .location()
            .map_or("<unknown location>".to_owned(), |loc| {
                format!("file {} at line {}", loc.file(), loc.line())
            });
        tracing::error!(location, msg, "panicked!");
    }));
}

fn from_url(url: Url) -> Result<Vec<u8>, ()> {
    let resp = reqwest::blocking::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION"),
        ))
        .build()
        .unwrap()
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .map_err(|err| tracing::error!(?err, "making request"))?;
    if resp.status().is_success() {
        Ok(resp.bytes().unwrap().as_ref().to_owned())
    } else {
        tracing::error!(status = %resp.status(), body = resp.text().unwrap_or("<missing".to_owned()), "server responded");
        Err(())
    }
}
fn from_file(path: &Path) -> Result<Vec<u8>, ()> {
    let target = Path::new(path);
    if let Some(extension) = target.extension() {
        if extension != "json" {
            tracing::warn!("target missing json file extension; proceeding anyway");
        }
    } else {
        tracing::error!("cannot process a directory");
        return Err(());
    }

    match fs::read(target) {
        Ok(buf) => Ok(buf),
        Err(err) => {
            tracing::error!(?err, "could not read file");
            Err(())
        }
    }
}

thread_local! {
    static LOCALS: RefCell<Option<Locals>> = RefCell::new(None);
}
struct Locals {
    output: BufWriter<StdoutLock<'static>>,
    stack: String,
    stack_item_starts: Vec<usize>,
}
impl Locals {
    fn new(color: bool) -> Self {
        Self {
            output: BufWriter::new(io::stdout().lock()),
            stack: if color {
                format!("{ANSI_KEY}json{ANSI_RESET}")
            } else {
                "json".to_owned()
            },
            stack_item_starts: Vec::new(),
        }
    }
}

fn process_recursively<const COLOR: bool>(json: &Value<'_>) {
    LOCALS.with(|cell| {
        let mut locals = cell.borrow_mut();

        match json {
            Value::Static(val) => {
                use io::Write;
                let locals: &mut Locals = locals.as_mut().unwrap();
                if COLOR {
                    writeln!(
                        locals.output,
                        "{} = {ANSI_NUM}{val}{ANSI_RESET};",
                        locals.stack
                    )
                    .unwrap();
                } else {
                    writeln!(locals.output, "{} = {val};", locals.stack).unwrap();
                }
            }
            Value::String(val) => {
                use io::Write;
                let locals: &mut Locals = locals.as_mut().unwrap();
                if COLOR {
                    writeln!(
                        locals.output,
                        "{} = \"{ANSI_STR}{val}{ANSI_RESET}\";",
                        locals.stack
                    )
                    .unwrap();
                } else {
                    writeln!(locals.output, "{} = \"{val}\";", locals.stack).unwrap();
                }
            }
            Value::Array(array) => {
                {
                    use io::Write;
                    let locals: &mut Locals = locals.as_mut().unwrap();
                    if COLOR {
                        writeln!(
                            locals.output,
                            "{} = {ANSI_BRACE}[]{ANSI_RESET};",
                            locals.stack
                        )
                        .unwrap();
                    } else {
                        writeln!(locals.output, "{} = [];", locals.stack).unwrap();
                    }
                }
                {
                    use fmt::Write;
                    for (i, item) in array.iter().enumerate() {
                        {
                            let locals: &mut Locals = locals.as_mut().unwrap();
                            locals.stack_item_starts.push(locals.stack.len());
                            if COLOR {
                                write!(
                                    &mut locals.stack,
                                    "{ANSI_BRACE}[{ANSI_NUM}{i}{ANSI_BRACE}]{ANSI_RESET}"
                                )
                                .unwrap();
                            } else {
                                write!(&mut locals.stack, "[{i}]").unwrap();
                            }
                        }
                        mem::drop(locals);
                        process_recursively::<COLOR>(item);
                        locals = cell.borrow_mut();
                        {
                            let locals: &mut Locals = locals.as_mut().unwrap();
                            locals
                                .stack
                                .truncate(locals.stack_item_starts.pop().unwrap());
                        }
                    }
                }
            }
            Value::Object(object) => {
                {
                    use io::Write;
                    let locals: &mut Locals = locals.as_mut().unwrap();
                    if COLOR {
                        writeln!(
                            locals.output,
                            "{} = {ANSI_BRACE}{{}}{ANSI_RESET};",
                            locals.stack
                        )
                        .unwrap();
                    } else {
                        writeln!(locals.output, "{} = {{}};", locals.stack).unwrap();
                    }
                }
                {
                    use fmt::Write;
                    let mut object: Vec<(&str, &Value<'_>)> =
                        object.iter().map(|(k, v)| (k.as_ref(), v)).collect();
                    object.sort_unstable_by_key(|&(k, _)| k);
                    for (key, value) in object {
                        {
                            let locals: &mut Locals = locals.as_mut().unwrap();
                            locals.stack_item_starts.push(locals.stack.len());
                            let dot = if locals.stack.is_empty() { "" } else { "." };
                            if COLOR {
                                write!(&mut locals.stack, "{dot}{ANSI_KEY}{key}{ANSI_RESET}")
                                    .unwrap();
                            } else {
                                write!(&mut locals.stack, "{dot}{key}").unwrap();
                            }
                        }
                        mem::drop(locals);
                        process_recursively::<COLOR>(value);
                        locals = cell.borrow_mut();
                        {
                            let locals: &mut Locals = locals.as_mut().unwrap();
                            locals
                                .stack
                                .truncate(locals.stack_item_starts.pop().unwrap());
                        }
                    }
                }
            }
        }
    })
}
