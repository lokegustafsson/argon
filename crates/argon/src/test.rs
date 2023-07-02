use include_dir::Dir;
use std::{cell::RefCell, ffi::OsStr, io, rc::Rc};

const TEST_CASES_ROUNDTRIP: Dir<'static> =
    include_dir::include_dir!("$TEST_CASE_DIR/roundtrip");
const TEST_CASES_NONCANON: Dir<'static> =
    include_dir::include_dir!("$TEST_CASE_DIR/noncanon");

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
    for entry in TEST_CASES_ROUNDTRIP
        .entries()
        .iter()
        .map(|entry| entry.as_file().unwrap())
    {
        assert_eq!(entry.path().extension(), Some(OsStr::new("json")));
        let sample = entry.contents_utf8().unwrap();

        let lines = gron(&sample);
        dbg!(&lines);
        let json = ungron(lines.as_bytes());
        if &json != &sample {
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

#[test]
fn noncanon_cases() {
    for entry in TEST_CASES_NONCANON
        .entries()
        .iter()
        .map(|entry| entry.as_file().unwrap())
    {
        if entry.path().extension() != Some(OsStr::new("json")) {
            continue;
        }
        let json = entry.contents_utf8().unwrap();
        let expected_gron = TEST_CASES_NONCANON
            .get_file(entry.path().with_extension("js"))
            .unwrap()
            .contents_utf8()
            .unwrap();

        let got_gron = gron(&json);
        let got_json = ungron(expected_gron.as_bytes());

        if &expected_gron != &got_gron {
            panic!(
                concat!(
                    "gronning test failure\n",
                    "BEGIN EXPECTED GRON\n",
                    "{}\n",
                    "END EXPECTED GRON\n",
                    "BEGIN GOT GRON\n",
                    "{}\n",
                    "END GOT GRON\n",
                ),
                expected_gron, got_gron
            );
        }
        if &json != &got_json {
            panic!(
                concat!(
                    "ungronning test failure\n",
                    "BEGIN EXPECTED JSON\n",
                    "{}\n",
                    "END EXPECTED JSON\n",
                    "BEGIN GOT JSON\n",
                    "{}\n",
                    "END GOT JSON\n",
                ),
                json, got_json
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
