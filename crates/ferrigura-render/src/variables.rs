//! The theme variables read by name, as the renderers read them before the typed tokens
//! (`docs-port/typed-views.md`): colour tokens as text, sizes in pixels, lists of colours, and the
//! truthiness the reference's JavaScript gives a variable. Only the tests compile it: it is the
//! reference that the readings of `ferrigura_model::theme::ThemeTokens` are compared with below.

use ferrigura_model::theme::tokens::leading_integer;
use ferrigura_model::value::Value;

use crate::svg::number;

/// A variable as text: a string as it is, a number as JavaScript prints it.
fn text_of(variables: &Value, name: &str) -> Option<String> {
    match variables.get(name)? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number_value) => Some(number(*number_value)),
        _ => None,
    }
}

/// The items of a list variable as text, in order; items that are not text or numbers are skipped.
fn texts_of(variables: &Value, name: &str) -> Vec<String> {
    match variables.get(name) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(text) => Some(text.clone()),
                Value::Number(number_value) => Some(number(*number_value)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether a variable is truthy as JavaScript reads it: `true`, a non-empty string, a non-zero
/// number. A missing variable is not truthy.
fn truthy(variables: &Value, name: &str) -> bool {
    match variables.get(name) {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Number(number_value)) => *number_value != 0.0 && !number_value.is_nan(),
        Some(Value::Array(_) | Value::Object(_)) => true,
        Some(Value::Null) | None => false,
    }
}

/// The leading pixel count of a font size or stroke width, as `parseFontSize` reads it.
fn px(variables: &Value, name: &str) -> Option<f64> {
    leading_integer(&text_of(variables, name)?)
}

#[cfg(test)]
mod tests {
    //! The typed tokens of `ferrigura_model::theme::ThemeTokens` read what these helpers read, name
    //! by name: for the themes of both shipped schemes, with and without overrides, and for many
    //! generated maps of theme variables with values of every kind.

    use ferrigura_model::theme::{Family, Theme, ThemeTokens, Token};
    use ferrigura_model::value::Value;

    use super::{px, text_of, texts_of, truthy};

    /// The readings of a token against the helpers on the map, under the name.
    fn assert_token(map: &Value, name: &str, token: &Token) {
        assert_eq!(text_of(map, name), token.text(), "text of {name}");
        assert_eq!(texts_of(map, name), token.texts(), "texts of {name}");
        assert_eq!(truthy(map, name), token.truthy(), "truthy of {name}");
        assert_eq!(px(map, name), token.px(), "px of {name}");
    }

    /// Every index of a family against the helpers on the map, as `format!` names it.
    fn assert_family<const N: usize>(
        map: &Value,
        family: &Family<N>,
        prefix: &str,
        indices: std::ops::Range<usize>,
    ) {
        for index in indices {
            let name = format!("{prefix}{index}");
            assert_eq!(family.name(index), name);
            assert_token(map, &name, family.get(index));
        }
    }

    /// Every token of a map against the helpers: the names of the struct, the families, and the two
    /// groups, which read their own object.
    fn assert_map(map: &Value) {
        let tokens = ThemeTokens::from_value(map);
        for (name, token) in tokens.entries() {
            assert_token(map, &name, token);
        }
        assert_family(map, &tokens.pie, "pie", 0..30);
        assert_family(map, &tokens.venn, "venn", 0..30);
        assert_family(map, &tokens.border, "border", 0..30);
        assert_family(map, &tokens.surface, "surface", 0..30);
        assert_family(map, &tokens.surface_peer, "surfacePeer", 0..30);
        assert_family(map, &tokens.fill_type, "fillType", 0..30);
        assert_family(map, &tokens.git, "git", 0..30);
        assert_family(map, &tokens.git_branch_label, "gitBranchLabel", 0..30);
        assert_family(map, &tokens.git_inv, "gitInv", 0..30);
        assert_family(map, &tokens.c_scale, "cScale", 0..30);
        assert_family(map, &tokens.c_scale_inv, "cScaleInv", 0..30);
        assert_family(map, &tokens.c_scale_label, "cScaleLabel", 0..30);
        assert_family(map, &tokens.c_scale_peer, "cScalePeer", 0..30);

        // The group of the XY chart: an object, read by name, or nothing for any other value.
        match map.get("xyChart") {
            Some(xy) if xy.is_object() => {
                let group = tokens.xy_chart.as_ref().expect("an object is the group");
                for (name, token) in group.entries() {
                    assert_token(xy, &name, token);
                }
            }
            other => {
                assert!(tokens.xy_chart.is_none());
                if let Some(xy) = other {
                    for name in ferrigura_model::theme::tokens::XyChartTokens::NAMES {
                        assert_eq!(text_of(xy, name), None);
                    }
                }
            }
        }

        // The radar group: an object, read by name; a theme without it gives none (the defaults apply).
        match map.get("radar") {
            Some(radar) if radar.is_object() => {
                let group = tokens.radar.as_ref().expect("an object is the group");
                for (name, token) in group.entries() {
                    assert_token(radar, &name, token);
                }
            }
            other => {
                assert!(tokens.radar.is_none());
                if let Some(radar) = other {
                    for name in ferrigura_model::theme::tokens::RadarTokens::NAMES {
                        assert_eq!(text_of(radar, name), None);
                    }
                }
            }
        }

        // The packet group: the renderer reads its options from the object by key, as a map of values.
        let packet = map.get("packet").and_then(Value::as_object);
        match &tokens.packet {
            Some(group) => {
                assert!(packet.is_some());
                for (name, token) in group.entries() {
                    assert_eq!(
                        packet.and_then(|options| options.get(&name)),
                        token.value(),
                        "packet.{name}"
                    );
                }
            }
            None => assert!(packet.is_none()),
        }
    }

    /// A small deterministic generator, so that the generated maps are the same on every run.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            usize::try_from(self.0 >> 33).unwrap_or(0)
        }

        fn below(&mut self, count: usize) -> usize {
            self.next() % count
        }
    }

    const TEXTS: [&str; 16] = [
        "#f00",
        "#ABCDEF",
        "hsl(240, 90%, NaN%)",
        "unreadable",
        "",
        "12px",
        " 14px",
        "-3px",
        "+4px",
        "25px",
        "red",
        "rgb(1,2,3)",
        "0",
        "NaN",
        "Infinity",
        "2.5em",
    ];

    const NUMBERS: [f64; 12] = [
        0.0,
        -0.0,
        1.5,
        14.0,
        2.0,
        1e-11,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e21,
        -7.0,
        12.0,
    ];

    /// A value of any kind; `depth` limits the nesting of lists and objects.
    fn value(rng: &mut Lcg, depth: usize, names: &[String]) -> Value {
        match rng.below(if depth == 0 { 6 } else { 8 }) {
            0 | 1 => Value::String(TEXTS[rng.below(TEXTS.len())].to_owned()),
            2 => Value::Number(NUMBERS[rng.below(NUMBERS.len())]),
            3 => Value::Bool(rng.below(2) == 0),
            4 => Value::Null,
            5 => Value::String(format!("{}px", rng.below(40))),
            6 => Value::Array(
                (0..rng.below(4))
                    .map(|_| value(rng, depth - 1, names))
                    .collect(),
            ),
            _ => Value::Object(
                (0..rng.below(4))
                    .map(|_| {
                        (
                            names[rng.below(names.len())].clone(),
                            value(rng, depth - 1, names),
                        )
                    })
                    .collect(),
            ),
        }
    }

    /// The names a generated map uses: the names of the struct, the families' members inside and
    /// outside their slots, and the groups' keys.
    fn names() -> Vec<String> {
        let mut names: Vec<String> = ThemeTokens::NAMES
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        names.extend(
            ferrigura_model::theme::tokens::XyChartTokens::NAMES
                .iter()
                .map(|name| (*name).to_owned()),
        );
        names.extend(
            ferrigura_model::theme::tokens::PacketTokens::NAMES
                .iter()
                .map(|name| (*name).to_owned()),
        );
        names.extend(
            ferrigura_model::theme::tokens::RadarTokens::NAMES
                .iter()
                .map(|name| (*name).to_owned()),
        );
        for prefix in [
            "pie",
            "venn",
            "border",
            "surface",
            "surfacePeer",
            "fillType",
            "git",
            "gitBranchLabel",
            "gitInv",
            "cScale",
            "cScaleInv",
            "cScaleLabel",
            "cScalePeer",
        ] {
            names.extend((0..20).map(|index| format!("{prefix}{index}")));
            names.push(prefix.to_owned());
            names.push(format!("{prefix}01"));
        }
        names.extend(
            [
                "cScale15",
                "pie13",
                "fillType9",
                "xyChart",
                "packet",
                "radar",
            ]
            .map(str::to_owned),
        );
        names
    }

    #[test]
    fn the_redux_themes_read_the_same_as_the_helpers_with_no_overrides_and_with_generated_ones() {
        for theme in [Theme::ReduxColor, Theme::ReduxDarkColor] {
            assert_map(&theme.variables(None));
            assert_map(&theme.variables(Some(&Value::empty_object())));
        }
        let names = names();
        let mut rng = Lcg(0x5eed_1234_abcd_0001);
        for _ in 0..400 {
            let overrides = Value::Object(
                (0..1 + rng.below(40))
                    .map(|_| {
                        (
                            names[rng.below(names.len())].clone(),
                            value(&mut rng, 2, &names),
                        )
                    })
                    .collect(),
            );
            for theme in [Theme::ReduxColor, Theme::ReduxDarkColor] {
                assert_map(&theme.variables(Some(&overrides)));
            }
            // The overrides themselves are a map too: a renderer reads what the map holds.
            assert_map(&overrides);
        }
    }
}
