use ms_compat::{MsError, Options, format, parse};
use std::{
    fmt::Write as _,
    io::Write as _,
    path::Path,
    process::{Command, Stdio},
};

const SEED: u64 = 20_261_110;
const RANDOM_NUMBERS: usize = 100_000;
const RANDOM_STRINGS: usize = 20_000;

// Explicit algorithm instead of a random crate: the corpus stays identical
// across dependency updates, operating systems, and CPU architectures.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64)
            .expect("remainder is less than bound, which already fits usize")
    }
}

enum Case {
    Format(f64),
    Parse(String),
}

impl Case {
    fn request(&self) -> String {
        match self {
            Self::Format(value) => format!("F\t{:016x}", value.to_bits()),
            Self::Parse(value) => {
                let mut request = String::from("P\t");
                for byte in value.bytes() {
                    write!(request, "{byte:02x}").unwrap();
                }
                request
            }
        }
    }

    fn rust_result(&self) -> String {
        match self {
            Self::Format(value) => {
                match (
                    format(*value, Options::default()),
                    format(*value, Options { long: true }),
                ) {
                    (Ok(short), Ok(long)) => format!("F\t{short}\t{long}"),
                    (Err(MsError::NotFinite), Err(MsError::NotFinite)) => "F_ERR".into(),
                    result => panic!("Unexpected format result for {value}: {result:?}"),
                }
            }
            Self::Parse(value) => match parse(value) {
                Ok(number) => format!("P\t{:016x}", number.to_bits()),
                Err(MsError::InvalidLength) => "P_ERR_LENGTH".into(),
                Err(MsError::InvalidFormat) => "P_ERR_FORMAT".into(),
                result => panic!("Unexpected parse result for {value:?}: {result:?}"),
            },
        }
    }
}

fn corpus() -> Vec<Case> {
    let mut random = SplitMix64(SEED);
    let mut cases = number_cases(&mut random);
    cases.extend(string_cases(&mut random));
    cases
}

fn number_cases(random: &mut SplitMix64) -> Vec<Case> {
    let mut cases = Vec::with_capacity(RANDOM_NUMBERS + RANDOM_STRINGS + 1_000);
    for _ in 0..RANDOM_NUMBERS {
        cases.push(Case::Format(f64::from_bits(random.next())));
    }
    for value in [
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MAX,
        -f64::MAX,
    ] {
        cases.push(Case::Format(value));
    }
    // Independent numeric constants catch regressions in the library constants.
    for size in [
        1_000.0,
        60_000.0,
        3_600_000.0,
        86_400_000.0,
        604_800_000.0,
        2_629_800_000.0,
        31_557_600_000.0,
    ] {
        for multiplier in [1.0, 1.5, 2.5, 10.0, 1e20, 1e21, 1_000_000_000_000_000_100.0] {
            let bits = f64::to_bits(size * multiplier);
            // Adjacent floats on either side of selection, rounding, and
            // scientific-notation boundaries, with both signs.
            for distance in 0..4 {
                for adjacent in [bits - distance, bits + distance] {
                    let value = f64::from_bits(adjacent);
                    cases.push(Case::Format(value));
                    cases.push(Case::Format(-value));
                }
            }
        }
    }

    cases
}

fn string_cases(random: &mut SplitMix64) -> Vec<Case> {
    let mut strings: Vec<String> = [
        "",
        " ",
        "1s\n",
        "1s\r",
        "1s\r\n",
        "1s\u{2028}",
        "1s\u{2029}",
        "1\u{00a0}s",
        "-0",
        "-.0",
        "1.",
        "+1",
        "1e2s",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    strings.extend([
        "1".repeat(100),
        "1".repeat(101),
        "😀".repeat(50),
        "😀".repeat(51),
    ]);
    let units = [
        "",
        "ms",
        "msec",
        "msecs",
        "millisecond",
        "milliseconds",
        "s",
        "sec",
        "secs",
        "second",
        "seconds",
        "m",
        "min",
        "mins",
        "minute",
        "minutes",
        "h",
        "hr",
        "hrs",
        "hour",
        "hours",
        "d",
        "day",
        "days",
        "w",
        "week",
        "weeks",
        "mo",
        "month",
        "months",
        "y",
        "yr",
        "yrs",
        "year",
        "years",
        "ſ",
        "ſecond",
        "µs",
        "fortnight",
        "monthss",
        "１２",
    ];
    for _ in 0..RANDOM_STRINGS {
        let count = random.below(110);
        let mut digits: String = (0..count)
            .map(|_| char::from(b"0123456789"[random.below(10)]))
            .collect();
        if random.below(10) < 6 {
            let position = random.below(digits.len() + 1);
            digits.insert(position, '.');
        }
        let prefix = ["", "", "-", "+", " ", "\t"][random.below(6)];
        let unit: String = units[random.below(units.len())]
            .chars()
            .map(|ch| {
                if random.below(2) == 0 {
                    ch
                } else {
                    ch.to_ascii_uppercase()
                }
            })
            .collect();
        let spaces = " ".repeat(random.below(4));
        let suffix = ["", "", "", "\n", " ", "\t"][random.below(6)];
        strings.push(format!("{prefix}{digits}{spaces}{unit}{suffix}"));
    }
    strings.into_iter().map(Case::Parse).collect()
}

#[test]
#[ignore = "requires Node.js; run cargo test --test js_compat -- --ignored --nocapture"]
fn matches_pinned_javascript_original() {
    let cases = corpus();
    assert_eq!(cases.len(), 100_793 + 20_017);
    let mut requests = String::new();
    for case in &cases {
        writeln!(requests, "{}", case.request()).unwrap();
    }

    let node = std::env::var_os("NODE").unwrap_or_else(|| "node".into());
    let oracle = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/js/oracle.cjs");
    let mut child = Command::new(&node).arg(oracle)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().unwrap_or_else(|error| panic!("Cannot start {node:?}: {error}. Install Node.js or set NODE to its executable path."));
    // The oracle reads all input before it emits output. Dropping this handle
    // signals EOF; wait_with_output drains stdout and stderr together.
    let write_result = child.stdin.take().unwrap().write_all(requests.as_bytes());
    let output = child
        .wait_with_output()
        .expect("Cannot wait for the JS oracle");
    assert!(
        output.status.success(),
        "JS oracle failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    write_result.expect("Cannot send the corpus to the JS oracle");
    let stdout = String::from_utf8(output.stdout).expect("JS oracle returned invalid UTF-8");
    let results: Vec<_> = stdout.lines().collect();
    assert_eq!(
        results.len(),
        cases.len(),
        "JS oracle returned an incomplete response"
    );
    for (index, (case, javascript)) in cases.iter().zip(results).enumerate() {
        assert_eq!(
            case.rust_result(),
            javascript,
            "Compatibility mismatch: seed={SEED}, case={index}, request={:?}",
            case.request()
        );
    }
    eprintln!(
        "seed={SEED}: 100793 numeric inputs (both formats) and 20017 parse inputs match upstream"
    );
}
