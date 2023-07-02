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
    const CORRECTLY_FORMATTED_SAMPLES: &[&str] = &[
        r#"{
  "abc": 123
}
"#,
        r#"{
  "abc": 123,
  "abc\n\t": "abc\n\t"
}
"#,
    ];

    for sample in CORRECTLY_FORMATTED_SAMPLES {
        let lines = gron(sample);
        dbg!(&lines);
        let json = ungron(lines.as_bytes());
        if &json != sample {
            panic!(
                concat!(
                    "roundtrip test failure\n",
                    "BEGIN SAMPLE\n",
                    "{}\n",
                    "END SAMPLE\n",
                    "BEGIN LINES\n",
                    "{}\n",
                    "END LINES\n",
                    "BEGIN JSON\n",
                    "{}\n",
                    "END JSON",
                ),
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
