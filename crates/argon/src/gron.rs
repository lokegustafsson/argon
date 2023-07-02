use patched_simd_json::value::borrowed::{self, Value};
use std::{
    borrow::Cow,
    fmt,
    io::{self, BufWriter},
    mem::ManuallyDrop,
};

pub fn process(buf: &mut [u8], have_color: bool, output: Box<dyn io::Write>) -> Result<(), ()> {
    let json = match borrowed::to_value(buf) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!(?err, "could not parse json");
            return Err(());
        }
    };

    let mut locals = Locals::new(have_color, output);
    if have_color {
        process_recursively::<true>(&json, &mut locals);
    } else {
        process_recursively::<false>(&json, &mut locals);
    }
    {
        use io::Write;
        locals.output.flush().unwrap();
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

struct Locals {
    output: BufWriter<Box<dyn io::Write>>,
    stack: String,
    stack_item_starts: Vec<usize>,
}
impl Locals {
    fn new(color: bool, output: Box<dyn io::Write>) -> Self {
        Self {
            output: BufWriter::new(output),
            stack: if color {
                format!("{ANSI_KEY}json{ANSI_RESET}")
            } else {
                "json".to_owned()
            },
            stack_item_starts: Vec::new(),
        }
    }
}

fn process_recursively<const COLOR: bool>(json: &Value<'_>, locals: &mut Locals) {
    match json {
        Value::Static(val) => {
            use io::Write;
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
            let val = escape_c1_control_codes(val);
            use io::Write;
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
                    process_recursively::<COLOR>(item, locals);
                    {
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
                        locals.stack_item_starts.push(locals.stack.len());
                        let dot = if locals.stack.is_empty() { "" } else { "." };
                        let key = escape_c1_control_codes(key);
                        if COLOR {
                            write!(&mut locals.stack, "{dot}{ANSI_KEY}{key}{ANSI_RESET}").unwrap();
                        } else {
                            write!(&mut locals.stack, "{dot}{key}").unwrap();
                        }
                    }
                    process_recursively::<COLOR>(value, locals);
                    {
                        locals
                            .stack
                            .truncate(locals.stack_item_starts.pop().unwrap());
                    }
                }
            }
        }
    }
}

fn escape_c1_control_codes<'a>(mut s: &'a str) -> Cow<'a, str> {
    // A codepoint `x` between `0x80` and `0x9f` inclusive is in utf8 encoded as
    // `0xc2` followed by `x`.
    let sb = s.as_bytes();
    if !memchr::memchr_iter(0xc2, sb)
        .any(|i| i + 1 < sb.len() && 0x80 <= sb[i + 1] && sb[i + 1] <= 0x9f)
    {
        return Cow::Borrowed(s);
    }

    let mut ret = String::new();
    while let Some(i) = memchr::memchr(0xc2, s.as_bytes()) {
        if i + 1 >= s.as_bytes().len() {
            continue;
        }
        let val = s.as_bytes()[i + 1];
        if val < 0x80 || val > 0x9f {
            continue;
        }
        ret.push_str(&s[..i]);
        ret.push_str("\\u00");
        let high_nibble = val >> 4;
        ret.push(char::from(b'0' + high_nibble));
        let low_nibble = val & 0xf;
        ret.push(char::from(if low_nibble < 10 {
            b'0' + low_nibble
        } else {
            b'a' - 10 + low_nibble
        }));
        s = &s[i + 2..];
    }
    ret.push_str(s);
    Cow::Owned(ret)
}
