use crate::ffi::CodeUnitWidth32;
pub use crate::regex_impl::Match as MatchImpl;
use crate::regex_impl::{
    Regex as RegexImpl, RegexBuilder as RegexBuilderImpl,
};

/// A compiled PCRE2 regular expression for matching sequences of Rust chars.
///
/// This regex is safe to use from multiple threads simultaneously. For top
/// performance, it is better to clone a new regex for each thread.
pub type Regex = RegexImpl<CodeUnitWidth32>;

/// A builder for configuring the compilation of a PCRE2 regex.
pub type RegexBuilder = RegexBuilderImpl<CodeUnitWidth32>;

/// Match represents a single match of a regex in a subject string.
///
/// The lifetime parameter `'s` refers to the lifetime of the matched portion
/// of the subject string.
pub type Match<'s> = MatchImpl<'s, CodeUnitWidth32>;

#[cfg(test)]
mod tests {
    use super::{Regex, RegexBuilder};
    use crate::is_jit_available;

    fn b(string: &str) -> Box<[char]> {
        string.chars().collect::<Vec<_>>().into_boxed_slice()
    }

    fn find_iter_tuples(re: &Regex, subject: &[char]) -> Vec<(usize, usize)> {
        let mut tuples = vec![];
        for result in re.find_iter(subject) {
            let m = result.unwrap();
            tuples.push((m.start(), m.end()));
        }
        tuples
    }

    fn cap_iter_tuples(re: &Regex, subject: &str) -> Vec<(usize, usize)> {
        let subject = subject.chars().collect::<Vec<_>>();
        let mut tuples = vec![];
        for result in re.captures_iter(&subject) {
            let caps = result.unwrap();
            let m = caps.get(0).unwrap();
            tuples.push((m.start(), m.end()));
        }
        tuples
    }

    #[test]
    fn caseless() {
        let re = RegexBuilder::new().caseless(true).build(b("a")).unwrap();
        assert!(re.is_match(&b("A")).unwrap());

        let re = RegexBuilder::new()
            .caseless(true)
            .ucp(true)
            .build(b("β"))
            .unwrap();
        assert!(re.is_match(&b("Β")).unwrap());
    }

    #[test]
    fn crlf() {
        let subject = &b("a\r\n");
        let re = RegexBuilder::new().crlf(true).build(b("a$")).unwrap();
        let m = re.find(subject).unwrap().unwrap();
        assert_eq!(m.as_pair(), (0, 1));
    }

    #[test]
    fn dotall() {
        let re = RegexBuilder::new().dotall(false).build(b(".")).unwrap();
        assert!(!re.is_match(&b("\n")).unwrap());

        let re = RegexBuilder::new().dotall(true).build(b(".")).unwrap();
        assert!(re.is_match(&b("\n")).unwrap());
    }

    #[test]
    fn extended() {
        let re = RegexBuilder::new().extended(true).build(b("a b c")).unwrap();
        assert!(re.is_match(&b("abc")).unwrap());
    }

    #[test]
    fn multi_line() {
        let re =
            RegexBuilder::new().multi_line(false).build(b("^abc$")).unwrap();
        assert!(!re.is_match(&b("foo\nabc\nbar")).unwrap());

        let re =
            RegexBuilder::new().multi_line(true).build(b("^abc$")).unwrap();
        assert!(re.is_match(&b("foo\nabc\nbar")).unwrap());
    }

    #[test]
    fn ucp() {
        let re = RegexBuilder::new().ucp(false).build(b(r"\w")).unwrap();
        assert!(!re.is_match(&b("β")).unwrap());

        let re = RegexBuilder::new().ucp(true).build(b(r"\w")).unwrap();
        assert!(re.is_match(&b("β")).unwrap());
    }

    #[test]
    fn utf() {
        let re = RegexBuilder::new().utf(false).build(b(".")).unwrap();
        assert_eq!(re.find(&b("β")).unwrap().unwrap().as_pair(), (0, 1));

        let re = RegexBuilder::new().utf(true).build(b(".")).unwrap();
        assert_eq!(re.find(&b("β")).unwrap().unwrap().as_pair(), (0, 1));
    }

    #[test]
    fn fmt_debug_works() {
        let subject = &b("x");
        let re = RegexBuilder::new().utf(false).build(b(".")).unwrap();
        let m = re.find(subject).unwrap().unwrap();
        let _ = format!("{:?}", m);
    }

    #[test]
    fn jit4lyfe() {
        if is_jit_available() {
            let re = RegexBuilder::new().jit(true).build(b(r"\w")).unwrap();
            assert!(re.is_match(&b("a")).unwrap());
        } else {
            // Check that if JIT isn't enabled, then we get an error if we
            // require JIT.
            RegexBuilder::new().jit(true).build(b(r"\w")).unwrap_err();
        }
    }

    // Unlike jit4lyfe, this tests that everything works when requesting the
    // JIT only if it's available. In jit4lyfe, we require the JIT or fail.
    // If the JIT isn't available, then in this test, we simply don't use it.
    #[test]
    fn jit_if_available() {
        let re = RegexBuilder::new()
            .jit_if_available(true)
            .build(b(r"\w"))
            .unwrap();
        assert!(re.is_match(&b("a")).unwrap());
    }

    // This tests a regression caused a segfault in the pcre2 library
    // https://github.com/BurntSushi/rust-pcre2/issues/10
    #[test]
    fn jit_test_lazy_alloc_subject() {
        let subject: Vec<char> = vec![];

        let re = RegexBuilder::new()
            .jit_if_available(true)
            .build(b(r"xxxx|xxxx|xxxx"))
            .unwrap();
        assert!(!re.is_match(&subject).unwrap());
    }

    #[test]
    fn utf_with_invalid_data() {
        let re = RegexBuilder::new().build(b(r".")).unwrap();
        assert_eq!(re.find(&b("\u{FF}")).unwrap().unwrap().as_pair(), (0, 1));

        let re = RegexBuilder::new().utf(true).build(b(r".")).unwrap();
        assert_eq!(re.find(&b("\u{FF}")).unwrap().unwrap().as_pair(), (0, 1));
    }

    #[test]
    fn capture_names() {
        let re = RegexBuilder::new()
            .build(b(r"(?P<foo>abc)|(def)|(?P<a>ghi)|(?P<springsteen>jkl)"))
            .unwrap();
        assert_eq!(
            re.capture_names().to_vec(),
            vec![
                None,
                Some("foo".to_string()),
                None,
                Some("a".to_string()),
                Some("springsteen".to_string()),
            ]
        );

        // Test our internal map as well.
        let capture_names_idx = re.get_capture_names_idxs();
        assert_eq!(capture_names_idx.len(), 3);
        assert_eq!(capture_names_idx["foo"], 1);
        assert_eq!(capture_names_idx["a"], 3);
        assert_eq!(capture_names_idx["springsteen"], 4);
    }

    #[test]
    fn captures_get() {
        let subject = &b("abc123");
        let re = Regex::new(b(r"[a-z]+(?:([0-9]+)|([A-Z]+))")).unwrap();
        let caps = re.captures(subject).unwrap().unwrap();

        let text1: &[char] = caps.get(1).map_or(&[], |m| m.as_bytes());
        let text2: &[char] = caps.get(2).map_or(&[], |m| m.as_bytes());
        assert_eq!(text1, &*b("123"));
        assert_eq!(text2, &*b(""));
    }

    #[test]
    fn find_iter_empty() {
        let re = Regex::new(b(r"(?m:^)")).unwrap();
        assert_eq!(find_iter_tuples(&re, &b("")), &[(0, 0)]);
        assert_eq!(find_iter_tuples(&re, &b("\n")), &[(0, 0)]);
        assert_eq!(find_iter_tuples(&re, &b("\n\n")), &[(0, 0), (1, 1)]);
        assert_eq!(find_iter_tuples(&re, &b("\na\n")), &[(0, 0), (1, 1)]);
        assert_eq!(
            find_iter_tuples(&re, &b("\na\n\n")),
            vec![(0, 0), (1, 1), (3, 3),]
        );
    }

    #[test]
    fn captures_iter_empty() {
        let re = Regex::new(b(r"(?m:^)")).unwrap();
        assert_eq!(cap_iter_tuples(&re, ""), &[(0, 0)]);
        assert_eq!(cap_iter_tuples(&re, "\n"), &[(0, 0)]);
        assert_eq!(cap_iter_tuples(&re, "\n\n"), &[(0, 0), (1, 1)]);
        assert_eq!(cap_iter_tuples(&re, "\na\n"), &[(0, 0), (1, 1)]);
        assert_eq!(
            cap_iter_tuples(&re, "\na\n\n"),
            &[(0, 0), (1, 1), (3, 3),]
        );
    }

    #[test]
    fn max_jit_stack_size_does_something() {
        if !is_jit_available() {
            return;
        }

        let hundred = "\
            ABCDEFGHIJKLMNOPQRSTUVWXY\
            ABCDEFGHIJKLMNOPQRSTUVWXY\
            ABCDEFGHIJKLMNOPQRSTUVWXY\
            ABCDEFGHIJKLMNOPQRSTUVWXY\
        ";
        let hay = format!("{}", hundred.repeat(100));

        // First, try a regex that checks that we can blow the JIT stack limit.
        let re = RegexBuilder::new()
            .ucp(true)
            .jit(true)
            .max_jit_stack_size(Some(1))
            .build(b(r"((((\w{10})){100}))+"))
            .unwrap();
        let result = re.is_match(&b(&hay));
        if result.is_ok() {
            // Skip this test, since for some reason we weren't able to blow
            // the stack limit.
            return;
        }
        let err = result.unwrap_err();
        assert!(err.to_string().contains("JIT stack limit reached"));

        // Now bump up the JIT stack limit and check that it succeeds.
        let re = RegexBuilder::new()
            .ucp(true)
            .jit(true)
            .max_jit_stack_size(Some(1 << 20))
            .build(b(r"((((\w{10})){100}))+"))
            .unwrap();
        assert!(re.is_match(&b(&hay)).unwrap());
    }
}
