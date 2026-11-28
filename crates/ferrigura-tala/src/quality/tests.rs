// SPDX-License-Identifier: MPL-2.0
// Source: D2 b463ea5, internal/quality/scoring_test.go.

use super::*;

fn empty() -> PositionedGraph {
    PositionedGraph {
        detached_ancestry: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
    }
}

fn node(x: f64, y: f64, width: f64, height: f64) -> PositionedNode {
    PositionedNode {
        body: Box {
            x,
            y,
            width,
            height,
        },
        container: None,
        effective_container: None,
        ancestry_parent: None,
        is_container: false,
        clustered: false,
        label: None,
        outside_label: false,
        icon: None,
        outside_icon_reserve: None,
        fixed_top_left: None,
        modifier_extension: None,
        loop_offsets: [0.0; 4],
    }
}

fn edge(source: usize, target: usize, points: &[(f64, f64)]) -> PositionedEdge {
    PositionedEdge {
        source,
        target,
        points: points.iter().map(|&(x, y)| Point { x, y }).collect(),
        label: None,
        label_is_positioned: false,
        source_arrowhead_label: None,
        target_arrowhead_label: None,
    }
}

fn test_evaluation(
    graph: &PositionedGraph,
    limit: u64,
) -> (Result<Evaluation, EvaluationError>, u64) {
    let mut work = Work::new(None, limit).unwrap();
    (evaluate_with_work(graph, &mut work), work.used)
}

#[test]
fn evaluate_keeps_area_as_separate_tie_breaker() {
    let result = |width| {
        let mut graph = empty();
        graph.nodes.push(node(0.0, 0.0, width, 1.0));
        evaluate(&graph, None).unwrap().score
    };
    let smaller = result(99.0);
    let larger = result(100.0);
    assert_eq!(smaller.penalty, larger.penalty);
    assert_eq!((smaller.area, larger.area), (99.0, 100.0));
    assert!(smaller.compare(larger).is_lt());
}

#[test]
fn evaluate_preserves_large_fractional_area() {
    for (width, height) in [(12.5, 3.25), (1_000_000_000.0, 1_000_000_000.0)] {
        let mut graph = empty();
        graph.nodes.push(node(0.25, 0.5, width, height));
        let area = evaluate(&graph, None).unwrap().score.area;
        assert_eq!(area, width * height);
        if width == 1_000_000_000.0 {
            assert!(area > f64::from(i32::MAX));
        }
    }
}

#[test]
fn non_shared_crossing_count_uses_fixed_width_accumulator() {
    let edges = [
        edge(0, 1, &[(0.0, 0.0), (10.0, 10.0)]),
        edge(0, 1, &[(0.0, 10.0), (10.0, 0.0)]),
    ];
    let mut work = Work::new(None, 100).unwrap();
    let crossings: i64 = routes::count_crossings(&edges, &mut work).unwrap();
    assert_eq!(crossings, 1);
}

#[test]
fn evaluate_canceled_before_work() {
    assert_eq!(
        evaluate(&empty(), Some(&|| true)),
        Err(EvaluationError::Cancelled)
    );
}

#[test]
fn score_existing_label_placements_canceled_before_work() {
    let cancelled = std::cell::Cell::new(false);
    let check = || cancelled.get();
    let mut work = Work::new(Some(&check), 2_000).unwrap();
    work.add(1_023).unwrap();
    cancelled.set(true);
    assert_eq!(
        labels::score(&empty(), &mut work),
        Err(EvaluationError::Cancelled)
    );
    assert_eq!(work.used, 1_024);
}

#[test]
fn evaluate_work_limit_charges_edges_without_complete_routes() {
    for points in [&[][..], &[(10., 5.)][..]] {
        let graph = PositionedGraph {
            detached_ancestry: Vec::new(),
            nodes: vec![node(0., 0., 10., 10.), node(20., 0., 10., 10.)],
            edges: (0..1_000).map(|_| edge(0, 1, points)).collect(),
        };
        assert_eq!(
            test_evaluation(&graph, 1_500),
            (Err(EvaluationError::WorkLimit), 1_501)
        );
    }
}

#[test]
fn evaluate_work_limit_is_exact() {
    let graph = PositionedGraph {
        detached_ancestry: Vec::new(),
        nodes: (0..4)
            .map(|index| node(f64::from(index * 40), 0., 20., 20.))
            .collect(),
        edges: vec![
            edge(0, 2, &[(0., 0.), (100., 100.)]),
            edge(1, 3, &[(0., 100.), (100., 0.)]),
        ],
    };
    let expected = evaluate(&graph, None).unwrap();
    let needed = expected.work;
    assert!(needed >= 2);
    assert_eq!(test_evaluation(&graph, needed), (Ok(expected), needed));
    assert_eq!(
        test_evaluation(&graph, needed - 1),
        (Err(EvaluationError::WorkLimit), needed)
    );
}

#[test]
fn evaluate_preflights_engine_resources() {
    let graph = PositionedGraph {
        detached_ancestry: Vec::new(),
        nodes: (0..=10_000)
            .map(|index| node(f64::from(index), 0., 1., 1.))
            .collect(),
        edges: Vec::new(),
    };
    assert_eq!(evaluate(&graph, None), Err(EvaluationError::ResourceLimit));
}

#[test]
fn scores_and_selects_completed_go_candidates_without_changing_geometry() {
    use crate::seed_selection::{Scored, SeedPlan};

    #[derive(Deserialize)]
    struct Fixture {
        fixture: String,
        selected: i64,
        candidates: Vec<Candidate>,
    }
    #[derive(Deserialize)]
    struct Candidate {
        seed: i64,
        graph: PositionedGraph,
        penalty: f64,
        area: f64,
        work: u64,
    }

    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!("go-candidates.json")).unwrap();
    for fixture in fixtures {
        let seeds: Vec<_> = fixture
            .candidates
            .iter()
            .map(|candidate| candidate.seed)
            .collect();
        for concurrency in [1, 3] {
            let plan = SeedPlan::new(None, Some(&seeds), concurrency).unwrap();
            let winner = plan
                .coordinate(
                    || false,
                    |index, seed| {
                        let candidate = &fixture.candidates[index];
                        let actual = evaluate(&candidate.graph, None)?;
                        assert_eq!(
                            actual,
                            Evaluation {
                                score: LayoutScore {
                                    penalty: candidate.penalty,
                                    area: candidate.area,
                                },
                                work: candidate.work,
                            },
                            "{} seed {seed}",
                            fixture.fixture
                        );
                        Ok::<_, EvaluationError>(Scored {
                            score: actual.score,
                            candidate,
                        })
                    },
                )
                .unwrap();
            assert_eq!(winner.seed, fixture.selected, "{}", fixture.fixture);
            assert!(std::ptr::eq(
                winner.result.candidate,
                &fixture.candidates[winner.index],
            ));
        }
    }
}

#[derive(Deserialize)]
struct CorpusCase {
    fixture: String,
    graph: PositionedGraph,
    score: LayoutScore,
    work: u64,
}

fn completed_corpus() -> impl Iterator<Item = CorpusCase> {
    include_str!("go-completed-corpus.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
}

#[test]
fn evaluate_work_limit_covers_scoring_kernels() {
    let kernel = completed_corpus()
        .find(|case| case.fixture == "work-cover-scoring-kernels")
        .unwrap();
    let expected = Evaluation {
        score: kernel.score,
        work: kernel.work,
    };
    assert!(kernel.work >= 100);
    assert_eq!(
        test_evaluation(&kernel.graph, kernel.work),
        (Ok(expected), kernel.work)
    );
    assert_eq!(
        test_evaluation(&kernel.graph, kernel.work - 1),
        (Err(EvaluationError::WorkLimit), kernel.work)
    );
}

#[test]
fn evaluate_work_limit_covers_layout_corpus() {
    let mut count = 0;
    let mut peak = 0;
    for case in completed_corpus().filter(|case| case.fixture != "work-cover-scoring-kernels") {
        let evaluated = evaluate(&case.graph, None).unwrap();
        assert_eq!(
            evaluated,
            Evaluation {
                score: case.score,
                work: case.work,
            },
            "{}",
            case.fixture
        );
        peak = peak.max(evaluated.work);
        count += 1;
    }
    assert_eq!(count, 34);
    assert!((50_000..=75_000).contains(&peak));
}
