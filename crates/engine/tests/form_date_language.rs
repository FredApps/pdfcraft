//! Form date fields name months in the app's date language (#5303). Its own test binary: the
//! names are process-wide, so other tests must not see them change.

use pdfcraft_engine::dates::use_form_date_language;
use pdfcraft_forms::af::{DateTime, Format, format_date, format_value, keystroke, parse_date};
use pdfcraft_js::{DocInfo, Event, Limits, run};

/// `util.printd(fmt)` of 18 October 2026, then of what `util.scand(fmt, typed)` reads.
fn printd(fmt: &str, typed: &str) -> String {
    let doc = DocInfo { file_name: "form.pdf".into(), num_pages: 1, page: 0, info: Vec::new(), xfa: false };
    let script = format!("event.value = util.printd('{fmt}', new Date(2026, 9, 18)) + '|' + util.printd('{fmt}', util.scand('{fmt}', '{typed}'));");
    let o = run(&script, &Event::field("Format", "date", ""), &doc, &[], &[], Limits::default());
    assert_eq!(o.error, None);
    o.value
}

#[test]
fn form_dates_follow_the_date_language() {
    let oct18 = DateTime { y: 2026, m: 10, d: 18, hh: 0, mm: 0, ss: 0 };
    let dd_mmm_yy = Format::Date("dd-mmm-yy".into());
    // English until the app says otherwise.
    assert_eq!(format_date(oct18, "dd-mmm-yy dddd ddd mmmm"), "18-Oct-26 Sunday Sun October");
    use_form_date_language("it");
    // AFDate_FormatEx("dd-mmm-yy"), what the issue's form used.
    assert_eq!(format_value(&dd_mmm_yy, "18-10-26"), "18-ott-26");
    assert_eq!(format_date(oct18, "dddd d mmmm yyyy, ddd"), "domenica 18 ottobre 2026, dom");
    // What it writes reads back, and so does English typing.
    assert_eq!(parse_date("18-ott-26", "dd-mmm-yy"), Some(oct18));
    assert_eq!(parse_date("18 ottobre 2026", "dd mmmm yyyy"), Some(oct18));
    assert_eq!(parse_date("18-Oct-26", "dd-mmm-yy"), Some(oct18));
    assert!(keystroke(&dd_mmm_yy, "date", "18-ott-26").is_ok());
    // util.printd writes Italian names; util.scand reads them.
    assert_eq!(printd("dd-mmm-yy", "18-ott-26"), "18-ott-26|18-ott-26");
    assert_eq!(printd("dd-mmm-yy", "18-Oct-26"), "18-ott-26|18-ott-26", "English typing reads too");
    assert_eq!(printd("dddd d mmmm yyyy", "domenica 18 ottobre 2026"), "domenica 18 ottobre 2026|domenica 18 ottobre 2026");
    // German short names end in a dot; they are written and read whole.
    use_form_date_language("de");
    assert_eq!(format_value(&dd_mmm_yy, "18-10-26"), "18-Okt.-26");
    assert_eq!(parse_date("18-Okt.-26", "dd-mmm-yy"), Some(oct18));
    // An unknown language is English again.
    use_form_date_language("xx");
    assert_eq!(format_value(&dd_mmm_yy, "18-10-26"), "18-Oct-26");
    assert_eq!(printd("dd-mmm-yy", "18-Oct-26"), "18-Oct-26|18-Oct-26");
    every_date_language_reads_back_what_it_writes();
    // Hostile typing never panics and stays quick: long, odd and mixed-script text.
    use_form_date_language("th");
    let junk = ["", " ", "ม.", "ม.ค.ม.ค.ม.ค.", "\u{0}", "1月1月1月", "tháng tháng", "Ottobre", "🙂ott🙂", "İstanbul ott 26"];
    for t in junk.iter().map(|s| s.to_string()).chain(["ม.ค.1".repeat(5000), "9".repeat(100_000)]) {
        let started = std::time::Instant::now();
        let _ = parse_date(&t, "dd-mmm-yy");
        let _ = keystroke(&dd_mmm_yy, "date", &t);
        assert!(started.elapsed() < std::time::Duration::from_secs(2), "{} chars took {:?}", t.len(), started.elapsed());
    }
    use_form_date_language("en");
}

/// Every date language: what a date field writes reads back as the same date, for each date
/// letter that writes a name, through the form path and through `util.scand`. Called from the
/// one test, which owns the process-wide names.
fn every_date_language_reads_back_what_it_writes() {
    let doc = DocInfo { file_name: "form.pdf".into(), num_pages: 1, page: 0, info: Vec::new(), xfa: false };
    let mut bad = Vec::new();
    for l in pdfcraft_engine::dates::DATE_LANGUAGES.iter() {
        use_form_date_language(l.code);
        for m in 1..=12u32 {
            let dt = DateTime { y: 2026, m, d: 18, hh: 0, mm: 0, ss: 0 };
            for fmt in ["dd-mmm-yy", "d mmmm yyyy", "dddd, mmmm d, yyyy", "ddd d mmm yyyy", "mmm d, yyyy ddd", "yyyymmmdd"] {
                let text = format_date(dt, fmt);
                // A short month ending in a number ("Thg 1") glued to the day is ambiguous
                // ("Thg 118": January 18 or November 8): nothing can read it back.
                if fmt == "yyyymmmdd" && l.months_short.iter().any(|n| n.ends_with(|c: char| c.is_ascii_digit())) {
                    continue;
                }
                if parse_date(&text, fmt).map(|p| (p.y, p.m, p.d)) != Some((2026, m, 18)) {
                    bad.push(format!("{} forms {fmt}: {text:?}", l.code));
                }
                let script = format!("var d = util.scand('{fmt}', {text:?}); event.value = d ? util.printd('yyyy-mm-dd', d) : 'null';");
                let o = run(&script, &Event::field("Format", "date", ""), &doc, &[], &[], Limits::default());
                if o.value != format!("2026-{m:02}-18") {
                    bad.push(format!("{} scand {fmt}: {text:?} -> {:?} {:?}", l.code, o.value, o.error));
                }
            }
        }
    }
    use_form_date_language("en");
    assert!(bad.is_empty(), "{} failures:\n{}", bad.len(), bad.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
}
