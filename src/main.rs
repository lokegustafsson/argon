use simd_json::value::borrowed::Value;
use std::{
    cell::RefCell,
    fmt, fs,
    io::{self, BufWriter, StdoutLock},
    mem,
    path::Path,
    process::ExitCode,
};
use tracing_subscriber::{filter::targets::Targets, layer::Layer};

#[derive(clap::Parser, Debug)]
#[command(about, verbatim_doc_comment)]
struct Args {
    /// Filesystem path or URL to the json file to process.
    path_or_url_to_json: String,
}

fn main() -> ExitCode {
    tracing::subscriber::set_global_default(
        Targets::new()
            .with_target("h2", tracing::Level::INFO)
            .with_target("tokio_util::codec::framed_impl", tracing::Level::DEBUG)
            .with_target("eframe::native::run", tracing::Level::DEBUG)
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

    let args: Args = clap::Parser::parse();
    let target = Path::new(&args.path_or_url_to_json);
    if let Some(extension) = target.extension() {
        if extension != "json" {
            tracing::warn!("target missing json file extension; proceeding anyway");
        }
    } else {
        tracing::error!("cannot process a directory");
        return ExitCode::FAILURE;
    }

    let mut buf = match fs::read(target) {
        Ok(buf) => buf,
        Err(err) => {
            tracing::error!(?err, "could not read file");
            return ExitCode::FAILURE;
        }
    };

    let json = match simd_json::value::borrowed::to_value(&mut buf) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(?err, "could not parse json");
            return ExitCode::FAILURE;
        }
    };

    process_recursively(&json);
    ExitCode::SUCCESS
}

thread_local! {
    static LOCALS: RefCell<Locals> = RefCell::new(Locals::new());
}
struct Locals {
    output: BufWriter<StdoutLock<'static>>,
    stack: String,
    stack_item_starts: Vec<usize>,
}
impl Locals {
    fn new() -> Self {
        Self {
            output: BufWriter::new(io::stdout().lock()),
            stack: "json".to_owned(),
            stack_item_starts: Vec::new(),
        }
    }
}

fn process_recursively(json: &Value<'_>) {
    LOCALS.with(|cell| {
        let mut locals = cell.borrow_mut();

        match json {
            Value::Static(val) => {
                use io::Write;
                let locals: &mut Locals = &mut locals;
                writeln!(locals.output, "{} = {val};", locals.stack).unwrap();
            }
            Value::String(val) => {
                use io::Write;
                let locals: &mut Locals = &mut locals;
                writeln!(locals.output, "{} = {val:?};", locals.stack).unwrap();
            }
            Value::Array(array) => {
                {
                    use io::Write;
                    let locals: &mut Locals = &mut locals;
                    writeln!(locals.output, "{} = [];", locals.stack).unwrap();
                }
                {
                    use fmt::Write;
                    for (i, item) in array.iter().enumerate() {
                        {
                            let locals: &mut Locals = &mut locals;
                            locals.stack_item_starts.push(locals.stack.len());
                            write!(&mut locals.stack, "[{i}]").unwrap();
                        }
                        mem::drop(locals);
                        process_recursively(item);
                        locals = cell.borrow_mut();
                        {
                            let locals: &mut Locals = &mut locals;
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
                    let locals: &mut Locals = &mut locals;
                    writeln!(locals.output, "{} = {{}};", locals.stack).unwrap();
                }
                {
                    use fmt::Write;
                    for (key, value) in &**object {
                        {
                            let locals: &mut Locals = &mut locals;
                            locals.stack_item_starts.push(locals.stack.len());
                            let dot = if locals.stack.is_empty() { "" } else { "." };
                            write!(&mut locals.stack, "{dot}{key}").unwrap();
                        }
                        mem::drop(locals);
                        process_recursively(value);
                        locals = cell.borrow_mut();
                        {
                            let locals: &mut Locals = &mut locals;
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
