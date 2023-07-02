use std::{cell::RefCell, io, rc::Rc, fs, ffi::OsStr};

const TEST_CASES_ROUNDTRIP: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testcases/roundtrip");
const TEST_CASES_NONCANON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testcases/noncanon");

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
    for entry in fs::read_dir(TEST_CASES_ROUNDTRIP).unwrap() {
        let entry = entry.unwrap();
        assert!(entry.file_type().unwrap().is_file());
        let path = entry.path();
        assert_eq!(path.extension(), Some(OsStr::new("json")));
        let sample = fs::read_to_string(path).unwrap();

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
    for entry in fs::read_dir(TEST_CASES_NONCANON).unwrap() {
        let entry = entry.unwrap();
        assert!(entry.file_type().unwrap().is_file());
        let path = entry.path();
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }
        let json = fs::read_to_string(&path).unwrap();
        let expected_gron = fs::read_to_string(path.with_extension("js")).unwrap();

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
