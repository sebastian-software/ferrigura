//! Differential run over the corpus: the port's stages, chart and `layout()` against the
//! `@upsetjs/venn.js` 2.0.0 bundle, as Mermaid's venn renderer runs it in Chromium.
//!
//! Each case in `tests/corpus` holds the areas the renderer handed the library, the random numbers
//! each step drew, and what the library returned or threw. The corpus is written by
//! `tools/capture-corpus.ts`; see `NOTICE.md` for the sources.
//!
//! The random source of the tests is a port of the harness's generator (`tools/port/geometry/
//! common.ts`, `renderReference`), seeded with 1 for each case, with the same draws in the same
//! order: the chart's draws, then the layout's, from one generator, as one render makes them. The
//! recorded draws are checked against the port's, so a difference in the generator is reported as
//! such.
//!
//! Every value is compared exactly: a double is the same double as the reference's, including the
//! sign of zero and NaN (any NaN matches any NaN, as JavaScript cannot tell them apart).

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use ferrigura_venn::{Area, AreaLayout, Centre, Circle, Solution};
use serde_json::Value;

/// Values must agree exactly: the port reproduces the library's arithmetic, so a difference is a
/// bug, not rounding.
const TOLERANCE: f64 = 0.0;

/// The cases the corpus must hold. The floor only goes up.
const CASE_FLOOR: usize = 447;

/// The renderer's chart padding (`VennDiagram`'s default), and its layout padding by default.
const CHART_PADDING: f64 = 15.0;

/// The harness's random generator (mulberry32, `renderReference` in tools/port/geometry/common.ts),
/// recording each draw.
struct Recorder {
    state: i32,
    draws: Vec<f64>,
}

impl Recorder {
    /// The generator of a render with `clock.seed` = `seed`.
    fn seeded(seed: u32) -> Self {
        Self {
            state: seed as i32,
            draws: Vec::new(),
        }
    }

    fn draw(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5_u32 as i32);
        let state = self.state;
        let mut t = (state ^ ((state as u32) >> 15) as i32).wrapping_mul(1 | state);
        t = (t.wrapping_add((t ^ ((t as u32) >> 7) as i32).wrapping_mul(61 | t))) ^ t;
        let value = f64::from((t ^ ((t as u32) >> 14) as i32) as u32) / 4_294_967_296.0;
        self.draws.push(value);
        value
    }
}

fn corpus_dir() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "corpus"]
        .iter()
        .collect()
}

fn load(name: &str) -> Value {
    let text = std::fs::read_to_string(corpus_dir().join(format!("{name}.json")))
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn case_names() -> Vec<String> {
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(corpus_dir().join("manifest.json")).expect("manifest.json"),
    )
    .expect("manifest.json parses");
    manifest["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|case| case["name"].as_str().expect("name").to_string())
        .collect()
}

/// A number of the corpus: a JSON number, or the sentinel string for -0 and the non-finite values.
fn number(value: &Value) -> f64 {
    match value {
        Value::Number(number) => number.as_f64().expect("a finite number"),
        Value::String(text) => match text.as_str() {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            "-0" => -0.0,
            other => panic!("not a number of the corpus: {other}"),
        },
        other => panic!("not a number of the corpus: {other}"),
    }
}

/// Whether two doubles agree: the same bits, or both NaN.
fn same(ours: f64, theirs: f64) -> bool {
    (ours.is_nan() && theirs.is_nan()) || ours.to_bits() == theirs.to_bits()
}

fn expect_same(label: &str, ours: f64, theirs: f64) -> Result<(), String> {
    if same(ours, theirs) || (ours - theirs).abs() <= TOLERANCE {
        Ok(())
    } else {
        Err(format!("{label}: {ours:?}, the reference has {theirs:?}"))
    }
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("an array of set ids")
        .iter()
        .map(|set| set.as_str().expect("a set id").to_string())
        .collect()
}

fn areas_of(value: &Value) -> Vec<Area> {
    value
        .as_array()
        .expect("areas")
        .iter()
        .map(|area| Area {
            sets: strings(&area["sets"]),
            size: number(&area["size"]),
        })
        .collect()
}

fn solution_of(value: &Value) -> Solution {
    value
        .as_array()
        .expect("circles")
        .iter()
        .map(|circle| {
            (
                circle["id"].as_str().expect("id").to_string(),
                Circle {
                    x: number(&circle["x"]),
                    y: number(&circle["y"]),
                    radius: number(&circle["radius"]),
                },
            )
        })
        .collect()
}

fn centre_of(value: &Value) -> Centre {
    Centre {
        x: number(&value["x"]),
        y: number(&value["y"]),
        disjoint: value["disjoint"].as_bool().unwrap_or(false),
    }
}

fn centres_of(value: &Value) -> Vec<(String, Centre)> {
    value
        .as_array()
        .expect("text centres")
        .iter()
        .map(|entry| {
            (
                entry["key"].as_str().expect("key").to_string(),
                centre_of(entry),
            )
        })
        .collect()
}

fn layout_of(value: &Value) -> Vec<AreaLayout> {
    value
        .as_array()
        .expect("layout")
        .iter()
        .map(|area| AreaLayout {
            sets: strings(&area["sets"]),
            text: centre_of(&area["text"]),
            circles: area["circles"]
                .as_array()
                .expect("circles")
                .iter()
                .map(|circle| ferrigura_venn::PlacedCircle {
                    set: circle["set"].as_str().expect("set").to_string(),
                    x: number(&circle["x"]),
                    y: number(&circle["y"]),
                    radius: number(&circle["radius"]),
                })
                .collect(),
        })
        .collect()
}

fn compare_areas(label: &str, expected: &[Area], actual: &[Area]) -> Result<(), String> {
    if expected.len() != actual.len() {
        return Err(format!(
            "{label}: {} areas, the reference has {}",
            actual.len(),
            expected.len()
        ));
    }
    for (index, (ours, theirs)) in actual.iter().zip(expected).enumerate() {
        if ours.sets != theirs.sets {
            return Err(format!(
                "{label}[{index}]: sets {:?}, the reference has {:?}",
                ours.sets, theirs.sets
            ));
        }
        expect_same(&format!("{label}[{index}].size"), ours.size, theirs.size)?;
    }
    Ok(())
}

fn compare_solution(label: &str, expected: &Value, actual: &Solution) -> Result<(), String> {
    let expected = solution_of(expected);
    if expected.len() != actual.len() {
        return Err(format!(
            "{label}: {} circles, the reference has {}",
            actual.len(),
            expected.len()
        ));
    }
    for ((ours_id, ours), (theirs_id, theirs)) in actual.iter().zip(&expected) {
        if ours_id != theirs_id {
            return Err(format!(
                "{label}: the circle {ours_id} is where the reference has {theirs_id}"
            ));
        }
        expect_same(&format!("{label}.{ours_id}.x"), ours.x, theirs.x)?;
        expect_same(&format!("{label}.{ours_id}.y"), ours.y, theirs.y)?;
        expect_same(
            &format!("{label}.{ours_id}.radius"),
            ours.radius,
            theirs.radius,
        )?;
    }
    Ok(())
}

fn compare_centres(
    label: &str,
    expected: &Value,
    actual: &[(String, Centre)],
) -> Result<(), String> {
    let expected = centres_of(expected);
    if expected.len() != actual.len() {
        return Err(format!(
            "{label}: {} centres, the reference has {}",
            actual.len(),
            expected.len()
        ));
    }
    for ((ours_key, ours), (theirs_key, theirs)) in actual.iter().zip(&expected) {
        if ours_key != theirs_key {
            return Err(format!(
                "{label}: the centre {ours_key} is where the reference has {theirs_key}"
            ));
        }
        expect_same(&format!("{label}.{ours_key}.x"), ours.x, theirs.x)?;
        expect_same(&format!("{label}.{ours_key}.y"), ours.y, theirs.y)?;
        if ours.disjoint != theirs.disjoint {
            return Err(format!("{label}.{ours_key}: disjoint differs"));
        }
    }
    Ok(())
}

fn compare_layout(label: &str, expected: &Value, actual: &[AreaLayout]) -> Result<(), String> {
    let expected = layout_of(expected);
    if expected.len() != actual.len() {
        return Err(format!(
            "{label}: {} areas, the reference has {}",
            actual.len(),
            expected.len()
        ));
    }
    for (index, (ours, theirs)) in actual.iter().zip(&expected).enumerate() {
        if ours.sets != theirs.sets {
            return Err(format!("{label}[{index}]: sets differ"));
        }
        expect_same(
            &format!("{label}[{index}].text.x"),
            ours.text.x,
            theirs.text.x,
        )?;
        expect_same(
            &format!("{label}[{index}].text.y"),
            ours.text.y,
            theirs.text.y,
        )?;
        if ours.text.disjoint != theirs.text.disjoint {
            return Err(format!("{label}[{index}].text: disjoint differs"));
        }
        if ours.circles.len() != theirs.circles.len() {
            return Err(format!("{label}[{index}]: circle count differs"));
        }
        for (circle, reference) in ours.circles.iter().zip(&theirs.circles) {
            if circle.set != reference.set {
                return Err(format!("{label}[{index}]: circle sets differ"));
            }
            let at = format!("{label}[{index}].{}", circle.set);
            expect_same(&format!("{at}.x"), circle.x, reference.x)?;
            expect_same(&format!("{at}.y"), circle.y, reference.y)?;
            expect_same(&format!("{at}.radius"), circle.radius, reference.radius)?;
        }
    }
    Ok(())
}

fn compare_draws(label: &str, expected: &Value, actual: &[f64]) -> Result<(), String> {
    let expected: Vec<f64> = expected
        .as_array()
        .expect("draws")
        .iter()
        .map(number)
        .collect();
    if expected.len() != actual.len() {
        return Err(format!(
            "{label}: {} draws, the reference drew {}",
            actual.len(),
            expected.len()
        ));
    }
    for (index, (ours, theirs)) in actual.iter().zip(&expected).enumerate() {
        expect_same(&format!("{label} draw {index}"), *ours, *theirs)?;
    }
    Ok(())
}

/// The areas the chart lays out: the zero-size singles and the areas that name them are left out.
fn chart_data(areas: &[Area]) -> Vec<Area> {
    let zero: Vec<&str> = areas
        .iter()
        .filter(|area| area.sets.len() == 1 && area.size == 0.0)
        .map(|area| area.sets[0].as_str())
        .collect();
    areas
        .iter()
        .filter(|area| !area.sets.iter().any(|set| zero.contains(&set.as_str())))
        .cloned()
        .collect()
}

fn expects_error(label: &str, errors: &Value, key: &str, failed: bool) -> Result<bool, String> {
    let recorded = !errors[key].is_null();
    if recorded != failed {
        return Err(if recorded {
            format!("{label}: the reference threw, the port did not")
        } else {
            format!("{label}: the port failed where the reference did not")
        });
    }
    Ok(recorded)
}

/// Checks one case against every step it records. Returns the first difference.
fn check_case(case: &Value) -> Result<(), String> {
    let areas = areas_of(&case["input"]["areas"]);
    let width = number(&case["input"]["width"]);
    let height = number(&case["input"]["height"]);
    let layout_padding = number(&case["input"]["layoutPadding"]);
    let errors = &case["errors"];

    // A set named __proto__ is refused by every step (the port does not reproduce the library's
    // output for it), so the cases that name one check the refusal and nothing else.
    if areas
        .iter()
        .any(|area| area.sets.iter().any(|set| set == "__proto__"))
    {
        let mut recorder = Recorder::seeded(1);
        let refused = ferrigura_venn::chart(&areas, width, height, CHART_PADDING, &mut || {
            recorder.draw()
        })
        .is_err()
            && ferrigura_venn::layout(&areas, width, height, layout_padding, &mut || {
                recorder.draw()
            })
            .is_err()
            && ferrigura_venn::greedy_layout(&areas).is_err();
        return if refused {
            Ok(())
        } else {
            Err("a set named __proto__ is not refused".to_string())
        };
    }

    let data = chart_data(&areas);

    // Stages, on the chart's areas: the missing pairs, the greedy placement, the constrained layout.
    let stages = &case["stages"];
    if !stages.is_null() {
        let stage_areas = areas_of(&stages["areas"]);
        let missing = ferrigura_venn::add_missing_areas(&data);
        compare_areas("addMissingAreas", &stage_areas, &missing)?;

        if expects_error("greedy", errors, "greedy", stages["greedy"].is_null())? {
            // The reference threw in the greedy placement: so must the port.
            if ferrigura_venn::greedy_layout(&stage_areas).is_ok() {
                return Err("greedy: the port placed the sets, the reference threw".to_string());
            }
        } else {
            let greedy = ferrigura_venn::greedy_layout(&stage_areas)
                .map_err(|error| format!("greedy: the port threw: {error}"))?;
            compare_solution("greedy", &stages["greedy"], &greedy)?;
        }

        let mut recorder = Recorder::seeded(1);
        if stage_areas.len() >= 8 {
            let constrained = if expects_error(
                "constrained",
                errors,
                "constrained",
                stages["constrained"].is_null(),
            )? {
                ferrigura_venn::constrained_mds_layout(&stage_areas, &mut || recorder.draw())
                    .err()
                    .map(|_| ())
                    .ok_or_else(|| {
                        "constrained: the port placed the sets, the reference threw".to_string()
                    })
                    .map(|()| None)
            } else {
                ferrigura_venn::constrained_mds_layout(&stage_areas, &mut || recorder.draw())
                    .map(Some)
                    .map_err(|error| format!("constrained: the port threw: {error}"))
            };
            if let Some(solution) = constrained? {
                compare_solution("constrained", &stages["constrained"], &solution)?;
            }
        }
        compare_draws("stages", &case["draws"]["stages"], &recorder.draws)?;
    } else {
        // No stages: the chart had nothing to lay out.
        if !data.is_empty() {
            return Err("the reference recorded no stages for a non-empty chart".to_string());
        }
    }

    // The chart, as the renderer calls it: from the seed, its own draws first.
    let mut render = Recorder::seeded(1);
    let chart = ferrigura_venn::chart(&areas, width, height, CHART_PADDING, &mut || render.draw());
    let chart_draws = render.draws.clone();
    let chart_failed = expects_error("chart", errors, "chart", chart.is_err())?;

    if !chart_failed {
        let chart = chart.map_err(|error| format!("chart: the port threw: {error}"))?;
        let record = &case["chart"];
        compare_solution("chart.circles", &record["circles"], &chart.circles)?;
        compare_centres(
            "chart.textCentres",
            &record["textCentres"],
            &chart.text_centres,
        )?;
        compare_draws("chart", &case["draws"]["chart"], &chart_draws)?;

        if data.is_empty() {
            if !record["venn"].as_array().is_some_and(Vec::is_empty) {
                return Err("chart: the reference drew circles for no areas".to_string());
            }
        } else {
            // The steps of the chart, one at a time, on the same draws.
            let mut steps = Recorder::seeded(1);
            let raw = ferrigura_venn::venn(&data, &mut || steps.draw())
                .map_err(|error| format!("venn: the port threw: {error}"))?;
            compare_solution("venn", &record["venn"], &raw)?;
            let normalized = ferrigura_venn::normalize_solution(&raw)
                .map_err(|error| format!("normalize: the port threw: {error}"))?;
            compare_solution("normalized", &record["normalized"], &normalized)?;
            let scaled = ferrigura_venn::scale_solution(&normalized, width, height, CHART_PADDING);
            compare_solution("scaled", &record["circles"], &scaled)?;
            let centres = ferrigura_venn::compute_text_centres(&scaled, &data)
                .map_err(|error| format!("text centres: the port threw: {error}"))?;
            compare_centres("textCentres", &record["textCentres"], &centres)?;
        }
    }

    // layout(): the same generator carries on after the chart, unless the chart threw, when the
    // generator starts again from the seed (as the renderer's second call does after a failure).
    let mut layout_render = render;
    if chart_failed {
        layout_render = Recorder::seeded(1);
    }
    let layout_start = layout_render.draws.len();
    let laid = ferrigura_venn::layout(&areas, width, height, layout_padding, &mut || {
        layout_render.draw()
    });
    let layout_draws = layout_render.draws[layout_start..].to_vec();
    if expects_error("layout", errors, "layout", laid.is_err())? {
        return compare_draws("layout", &case["draws"]["layout"], &layout_draws);
    }
    let laid = laid.map_err(|error| format!("layout: the port threw: {error}"))?;
    compare_layout("layout", &case["layout"], &laid)?;
    compare_draws("layout", &case["draws"]["layout"], &layout_draws)
}

/// Whether the harness generator reproduces the first draw the corpus recorded for seed 1 (the
/// generator is integer arithmetic, so this is the same number in every runtime).
#[test]
fn harness_generator_draws_the_recorded_numbers() {
    let mut recorder = Recorder::seeded(1);
    let first = recorder.draw();
    assert!(same(first, 0.627_073_940_588_161_3), "first draw {first:?}");
    let second = recorder.draw();
    assert!(
        same(second, 0.002_735_721_180_215_478),
        "second draw {second:?}"
    );
}

/// The result of one case: its index in the manifest, its group, and the first difference.
type Outcome = (usize, String, Result<(), String>);

#[test]
fn every_case_matches_the_reference_exactly() {
    let names = case_names();
    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(names.len().max(1));
    let next = AtomicUsize::new(0);
    let outcomes: Mutex<Vec<Outcome>> = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(name) = names.get(index) else {
                        break;
                    };
                    let case = load(name);
                    let group = case["group"].as_str().expect("group").to_string();
                    let outcome = check_case(&case);
                    outcomes
                        .lock()
                        .expect("outcomes")
                        .push((index, group, outcome));
                }
            });
        }
    });
    let mut outcomes = outcomes.into_inner().expect("outcomes");
    outcomes.sort_by_key(|(index, _, _)| *index);

    let mut failures = Vec::new();
    let mut groups = std::collections::BTreeMap::<String, (usize, usize)>::new();
    for (index, group, outcome) in &outcomes {
        let entry = groups.entry(group.clone()).or_insert((0, 0));
        entry.1 += 1;
        match outcome {
            Ok(()) => entry.0 += 1,
            Err(reason) => failures.push(format!("{}: {reason}", names[*index])),
        }
    }
    for line in &failures {
        println!("differs  {line}");
    }
    for (group, (matching, total)) in &groups {
        println!("{group}: {matching} of {total} cases match the reference");
    }
    println!(
        "{} of {} cases match the reference",
        names.len() - failures.len(),
        names.len()
    );
    assert!(
        names.len() >= CASE_FLOOR,
        "the corpus has {} cases",
        names.len()
    );
    assert!(failures.is_empty(), "{} cases differ", failures.len());
}
