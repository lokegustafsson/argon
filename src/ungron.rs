use rayon::{iter::ParallelIterator, str::ParallelString};
use simd_json::{value::borrowed::Value, StaticNode};
use std::{
    borrow::Cow,
    io::{self, BufWriter, Write},
    mem::{self, ManuallyDrop},
};

pub fn process(data: &[u8]) -> Result<(), ()> {
    if data.is_empty() {
        tracing::error!("got EOF");
        return Err(());
    }

    let data = std::str::from_utf8(data).unwrap();
    let json = data
        .par_lines()
        .fold_with(Value::Static(StaticNode::Null), add_line_to_json)
        .reduce(|| Value::Static(StaticNode::Null), merge_json);

    let mut output = BufWriter::new(io::stdout().lock());
    simd_json::to_writer_pretty(&mut output, &json).unwrap();
    output.write_all(b"\n").unwrap();
    output.flush().unwrap();

    // Leak `json` for quicker exit
    let _ = ManuallyDrop::new(json);
    Ok(())
}

fn add_line_to_json<'a>(mut json: Value<'a>, line: &'a str) -> Value<'a> {
    let line = line.strip_prefix("json").unwrap();
    add_line_to_json_impl(&mut json, line);
    json
}
fn add_line_to_json_impl<'a>(mut json: &mut Value<'a>, mut line: &'a str) {
    // Grammar-ish:
    // `line = "json" path* " = " item ","`
    // `path = "." [^\.\[ ]*`
    // `item = "{}" | "[]" | '"blah"' | 12345 | null`
    loop {
        let bytes = line.as_bytes();
        match bytes.first().unwrap() {
            b'[' => {
                if bytes[1] == b'"' {
                    unimplemented!("non-integer square bracket access");
                } else {
                    if let Value::Static(StaticNode::Null) = json {
                        *json = Value::Array(Vec::new());
                    }
                    let Value::Array(v) = json else { unreachable!() };
                    v.push(Value::Static(StaticNode::Null));

                    let end = memchr::memchr(b']', bytes).unwrap();
                    line = &line[end + 1..];
                    json = v.last_mut().unwrap();
                }
            }
            b'.' => {
                if let Value::Static(StaticNode::Null) = json {
                    *json = Value::Object(Box::default());
                }
                let Value::Object(o) = json else { unreachable!() };

                let next = 1 + memchr::memchr3(b'[', b'.', b' ', &bytes[1..]).unwrap();

                json = o
                    .entry(Cow::Borrowed(&line[1..next]))
                    .or_insert(Value::Static(StaticNode::Null));
                line = &line[next..];
            }
            b' ' => {
                match line.as_bytes()[3] {
                    b'"' => {
                        *json = Value::String(Cow::Borrowed(
                            line.strip_prefix(" = \"")
                                .unwrap()
                                .strip_suffix("\";")
                                .unwrap(),
                        ));
                    }
                    b'n' | b'{' | b'[' => {}
                    b't' => *json = Value::Static(StaticNode::Bool(true)),
                    b'f' => *json = Value::Static(StaticNode::Bool(false)),
                    _ => {
                        let digits = line.strip_prefix(" = ").unwrap().strip_suffix(";").unwrap();
                        let node = Err(digits)
                            .or_else(|digits| match digits.parse::<u64>() {
                                Ok(num) => Ok(StaticNode::from(num)),
                                Err(_) => Err(digits),
                            })
                            .or_else(|digits| match digits.parse::<i64>() {
                                Ok(num) => Ok(StaticNode::from(num)),
                                Err(_) => Err(digits),
                            })
                            .or_else(|digits| match digits.parse::<f64>() {
                                Ok(num) => Ok(StaticNode::from(num)),
                                Err(_) => Err(digits),
                            })
                            .unwrap();
                        *json = Value::Static(node);
                    }
                }
                return;
            }
            _ => panic!("invalid"),
        }
    }
}

fn merge_json<'a>(j1: Value<'a>, j2: Value<'a>) -> Value<'a> {
    match (j1, j2) {
        (Value::Object(mut o1), Value::Object(mut o2)) => Value::Object({
            if o2.len() > o1.len() {
                mem::swap(&mut o1, &mut o2);
            }
            for (k, v) in *o2 {
                o1.insert_nocheck(k, v);
            }
            o1
        }),
        (Value::Array(mut a1), Value::Array(mut a2)) => Value::Array({
            a1.append(&mut a2);
            a1
        }),
        (Value::Static(StaticNode::Null), any) => any,
        (any, Value::Static(StaticNode::Null)) => any,
        (a, b) => panic!("invalid gronlines; merging {a} and {b}"),
    }
}
