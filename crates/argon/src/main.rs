#[cfg(not(target_feature = "avx2"))]
compile_error!("unexpectedly missing required feature AVX2");

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{
    fs,
    io::{self, Read},
    mem::ManuallyDrop,
    path::Path,
    process::ExitCode,
};
use tracing_subscriber::{filter::targets::Targets, layer::Layer};
use url::Url;

mod gron;
mod seccomp;
mod ungron;

#[cfg(test)]
mod test;

#[derive(clap::Parser, Debug)]
#[command(about, verbatim_doc_comment)]
/// Example invocations:
/// - `echo '[1,"abc\r\ncba"]' | argon`
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
    #[arg(short, long)]
    ungron: bool,
}

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
            from_file(Path::new(path_or_url_to_json), args.ungron)?
        }
    } else {
        let mut buf = Vec::new();
        io::stdin().lock().read_to_end(&mut buf).unwrap();
        buf
    };

    seccomp::setup_seccomp(args.ungron);

    let output = Box::new(io::stdout().lock());

    if args.ungron {
        ungron::process(&buf, output)?;
    } else {
        let have_color = match (args.color, args.no_color, atty::is(atty::Stream::Stdout)) {
            (true, false, _) => true,
            (false, true, _) => false,
            (_, _, tty) => tty,
        };
        gron::process(&mut buf, have_color, output)?;
    }
    // Leak `buf` for quicker exit
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
                    .with_writer(io::stderr)
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
fn from_file(path: &Path, ungron: bool) -> Result<Vec<u8>, ()> {
    let target = Path::new(path);
    if let Some(extension) = target.extension() {
        if !ungron && extension != "json" {
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
