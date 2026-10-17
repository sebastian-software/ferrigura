//! The architecture renderer against the reference's behaviour that is not a geometry: the ids of its
//! elements (`svgDraw.spec.ts`), the seeded layout (`architectureSeed.spec.ts`, and the seed of
//! `architecture.spec.ts`), the fallback icon, and the wrapping of a title. The geometry itself is
//! checked against Chromium by `pnpm port:geometry architecture`, over the fixtures and the probes.

use ferrigura_model::config::{defaults, typed};
use ferrigura_model::theme::Theme;
use ferrigura_model::value::Value;
use ferrigura_render::RenderError;
use ferrigura_render::architecture::{ArchitectureInput, render};
use ferrigura_render::theme::Themes;

/// The diagram of `svgDraw.spec.ts` (`should add ids`).
const SVG_DRAW: &str = "architecture-beta
  group api(cloud)[API]

  service db(database)[Database] in api
  service disk1(disk)[Storage] in api
  service disk2(disk)[Storage] in api
  service server(server)[Server] in api

  db:L -- R:server
  disk1:T -- B:server
  disk2:T -- B:db
";

/// The explicit-seed fixture: one VM in a nested group, and a sibling subscription group.
const SEEDED: &str = "architecture-beta
  group sub1(cloud)[Subscription A]
  group vnet1(cloud)[VNet A] in sub1
  service vm1(server)[VM] in vnet1
  service web(server)[Web App]
  service db(database)[Database] in sub1
  vm1:R -- L:web
  vm1:B -- T:db
";

/// Two data centres with web servers, databases, a load balancer and a junction (probe `p12`). Its
/// layout depends on the seed.
const LARGER: &str = "architecture-beta
  group dc1(cloud)[DC 1]
  group dc2(cloud)[DC 2]
  service w1(server)[Web 1] in dc1
  service w2(server)[Web 2] in dc1
  service w3(server)[Web 3] in dc1
  service db1(database)[DB 1] in dc1
  service w4(server)[Web 4] in dc2
  service w5(server)[Web 5] in dc2
  service db2(database)[DB 2] in dc2
  service lb(internet)[Load balancer]
  service fw(disk)[Firewall]
  service cdn(internet)[CDN]
  junction jx
  w1:R -- L:w2
  w2:R -- L:w3
  w3:B -- T:db1
  w4:R -- L:w5
  w5:B -- T:db2
  db1:R -- L:db2
  lb:R -- L:fw
  fw:R -- L:cdn
  lb:B -- T:jx
  jx:R -- L:w1
  jx:B -- T:w4
  cdn:B -- T:fw
";

/// Renders a diagram with the redux colour theme, the default configuration and the given settings of
/// the `architecture` section.
fn draw(text: &str, id: &str, settings: &[(&str, Value)]) -> Result<String, RenderError> {
    let diagram = ferrigura_syntax::architecture::parse(text).expect("the diagram parses");
    let mut value = defaults();
    for (key, setting) in settings {
        value
            .get_mut("architecture")
            .and_then(Value::as_object_mut)
            .expect("the defaults hold an architecture section")
            .insert((*key).to_owned(), setting.clone());
    }
    let config = typed(&value).expect("the configuration is typed");
    let variables = Theme::ReduxColor.variables(None);
    render(&ArchitectureInput {
        diagram: &diagram,
        config: &config,
        themes: Themes::Single(&variables),
        id,
    })
}

#[test]
fn the_elements_take_the_reference_ids() {
    let svg = draw(SVG_DRAW, "svg", &[]).expect("the diagram renders");
    for id in [
        "svg-group-api",
        "svg-service-db",
        "svg-service-disk1",
        "svg-service-disk2",
        "svg-service-server",
        "svg-L_db_server_0",
        "svg-L_disk1_server_0",
        "svg-L_disk2_db_0",
    ] {
        assert!(svg.contains(&format!("id=\"{id}\"")), "{id} is missing");
    }
}

#[test]
fn the_layout_is_the_same_for_the_same_seed() {
    let first = draw(SEEDED, "svg", &[("seed", Value::Number(42.0))]).expect("renders");
    let second = draw(SEEDED, "svg", &[("seed", Value::Number(42.0))]).expect("renders");
    assert_eq!(first, second);
}

#[test]
fn another_seed_gives_another_layout() {
    let one = draw(LARGER, "svg", &[("seed", Value::Number(1.0))]).expect("renders");
    let two = draw(LARGER, "svg", &[("seed", Value::Number(2.0))]).expect("renders");
    assert!(one != two, "seeds 1 and 2 lay the diagram out the same way");
}

#[test]
fn the_randomized_start_is_the_same_for_the_same_seed() {
    let settings = [
        ("randomize", Value::Bool(true)),
        ("seed", Value::Number(5.0)),
    ];
    let first = draw(SEEDED, "svg", &settings).expect("renders");
    let second = draw(SEEDED, "svg", &settings).expect("renders");
    assert_eq!(first, second);
}

#[test]
fn a_seed_of_zero_is_refused_as_not_reproducible() {
    let error = draw(SEEDED, "svg", &[("seed", Value::Number(0.0))])
        .expect_err("a seed of 0 is the reference's real Math.random");
    assert!(matches!(error, RenderError::Unsupported(_)), "{error}");
}

#[test]
fn an_unknown_icon_draws_the_question_mark() {
    let svg = draw(
        "architecture-beta
  service unknown(iconnamedoesntexist)[Unknown Icon]
",
        "svg",
        &[],
    )
    .expect("renders");
    assert!(
        svg.contains(">?</tspan>"),
        "the fallback icon has no question mark"
    );
    assert!(
        svg.contains("fill: #087ebf"),
        "the fallback icon has no blue square"
    );
}

#[test]
fn a_long_title_wraps_inside_a_service() {
    let svg = draw(
        "architecture-beta
  service queue(disk)[Message queue for background processing of asynchronous jobs]
",
        "svg",
        &[],
    )
    .expect("renders");
    let lines = svg.matches("class=\"text-outer-tspan row\"").count();
    assert!(lines > 1, "the title has {lines} line");
}
