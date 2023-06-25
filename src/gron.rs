use simd_json::value::borrowed::Value;
use std::{
    cell::RefCell,
    fmt,
    io::{self, BufWriter, StdoutLock},
    mem::{self, ManuallyDrop},
};

pub fn process(buf: &mut [u8], have_color: bool) -> Result<(), ()> {
    let json = match simd_json::value::borrowed::to_value(buf) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(?err, "could not parse json");
            return Err(());
        }
    };

    LOCALS.with(|cell| *cell.borrow_mut() = Some(Locals::new(have_color)));
    if have_color {
        process_recursively::<true>(&json);
    } else {
        process_recursively::<false>(&json);
    }

    // Leak `json` for quicker exit
    let _ = ManuallyDrop::new(json);
    Ok(())
}

const ANSI_KEY: &str = "\x1B[34m";
const ANSI_NUM: &str = "\x1B[31m";
const ANSI_STR: &str = "\x1B[32m";
const ANSI_BRACE: &str = "\x1B[35m";
const ANSI_RESET: &str = "\x1B[0m";

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
