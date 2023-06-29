use std::{cell::RefCell, io, rc::Rc};

const HAVE_COLOR: bool = false;

fn gron(input: &str) -> String {
    let mut input = input.as_bytes().to_owned();
    let (output, ret) = Output::new();
    crate::gron::process(&mut input, HAVE_COLOR, output).unwrap();
    ret.get()
}
fn ungron(input: &[u8]) -> String {
    let (output, ret) = Output::new();
    crate::ungron::process(input, output).unwrap();
    ret.get()
}

#[test]
fn roundtrip_cases() {
    const SAMPLES: &[&str] = &[
        r#"{
  "abc": 123
}
"#,
        r#"{
  "abc\n\t": "abc\n\t",
  "abc": 123
}
"#,
    ];

    for sample in SAMPLES {
        let lines = gron(sample);
        dbg!(&lines);
        let json = ungron(lines.as_bytes());
        if &json != sample {
            panic!(
                "roundtrip test failure\nsample = {}\nlines = {}\njson = {}",
                sample, lines, json
            );
        }
    }
}

struct Output {
    inner: Rc<RefCell<Vec<u8>>>,
}
impl Output {
    fn new() -> (Box<Self>, Self) {
        let ret = Rc::new(RefCell::new(Vec::new()));
        (
            Box::new(Self {
                inner: Rc::clone(&ret),
            }),
            Self { inner: ret },
        )
    }
    fn get(self) -> String {
        (std::str::from_utf8(&(*self.inner).borrow()).unwrap()).to_owned()
    }
}
impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (*self.inner).borrow_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        (*self.inner).borrow_mut().flush()
    }
}
