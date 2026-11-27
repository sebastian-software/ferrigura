//! Standalone and shared-source parsing must agree on models and diagnostics.
use ferrigura_syntax::{config, detect, detect_prepared, mindmap, pie, preprocess, xychart};

#[test]
fn shared_source_matches_standalone_parsers() {
    for text in [
        "---\r\ntitle: Pets\r\n---\r\npie\r\n\"Dogs\": 3\r\n",
        "%% comment\n---\ntitle: Pets\n---\npie\n\"Dogs\": 3\n",
        "---\ntitle: Pets\n---\npie\n\"Dogs\": nope\n",
    ] {
        let source = preprocess::prepare(text).unwrap();
        assert_eq!(detect(text), detect_prepared(&source));
        assert_eq!(pie::parse(text), pie::parse_prepared(&source), "{text}");
    }
    for text in [
        "---\r\ntitle: Sales\r\n---\r\n%%{init: {'config': {'chartOrientation': 'horizontal'}}}%%\r\nxychart-beta\r\nbar [1, 2]\r\n",
        "xychart-beta\nbar [1, nope]\n",
    ] {
        let source = preprocess::prepare(text).unwrap();
        let directive = config::read_prepared(&source, detect_prepared(&source)).unwrap();
        assert_eq!(config::read(text).unwrap(), directive);
        assert_eq!(
            xychart::parse(text),
            xychart::parse_prepared(&source, &directive),
            "{text}"
        );
    }
    let text = "%%{init: {'mindmap': {'padding': 20}}}%%\nmindmap\n  root\n    child\n";
    let source = preprocess::prepare(text).unwrap();
    let directive = config::read_prepared(&source, detect_prepared(&source)).unwrap();
    assert_eq!(
        mindmap::parse(text),
        mindmap::parse_prepared(&source, &directive)
    );
}

#[test]
fn malformed_front_matter_keeps_standalone_errors() {
    let text = "---\nconfig: [\n---\npie\n";
    assert_eq!(
        preprocess::prepare(text).unwrap_err(),
        pie::parse(text).unwrap_err()
    );
    assert_eq!(detect(text), None);
}
