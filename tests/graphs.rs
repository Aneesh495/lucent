use lucent::{
    eval::{Angle, Scope},
    graph::{CellKind, Context, Controls, CurveProblem, View, area, curve},
    lang::parse,
    program::Program,
};

fn controls() -> Controls {
    Controls {
        bits: 96,
        angle: Angle::Radians,
        budget: 4000,
        tolerance: 0.45,
    }
}

#[test]
fn poles_never_join_and_cells_cover_the_whole_domain() {
    let expr = parse("1/(x-0.000013)").unwrap().expression().unwrap();
    let view = View {
        x: [-4.0, 4.0],
        y: [-6.0, 6.0],
        pixels: [720, 540],
    };
    let ctx = Context {
        program: &Program::default(),
        scope: &Scope::new(),
        control: controls(),
    };
    let plot = curve(
        &ctx,
        &CurveProblem {
            horizontal: None,
            vertical: &expr,
            axis: "x",
            domain: view.x,
            view,
        },
    )
    .unwrap();
    assert_eq!(plot.cells.first().unwrap().domain[0], view.x[0]);
    assert_eq!(plot.cells.last().unwrap().domain[1], view.x[1]);
    assert!(
        plot.cells
            .windows(2)
            .all(|pair| pair[0].domain[1] == pair[1].domain[0])
    );
    let containing: Vec<_> = plot
        .cells
        .iter()
        .filter(|c| c.domain[0] <= 0.000013 && c.domain[1] >= 0.000013)
        .collect();
    assert!(!containing.is_empty());
    assert!(containing.iter().all(|c| c.kind == CellKind::Unresolved));
    assert!(
        plot.cells
            .iter()
            .filter(|c| c.kind == CellKind::Segment)
            .count()
            > 8
    );
}

#[test]
fn implicit_crossing_is_labeled_sampled() {
    let expr = parse("x^2 + y^2 - 1").unwrap().expression().unwrap();
    let view = View {
        x: [-2.0, 2.0],
        y: [-2.0, 2.0],
        pixels: [128, 128],
    };
    let ctx = Context {
        program: &Program::default(),
        scope: &Scope::new(),
        control: controls(),
    };
    let plot = area(&ctx, &expr, view, false).unwrap();
    assert!(plot.cells.iter().any(|c| c.kind == CellKind::Empty));
    assert!(plot.cells.iter().any(|c| c.kind == CellKind::Crossing
        && c.reason.as_deref().unwrap().contains("topology unresolved")));
}

#[test]
fn interval_gradient_proves_a_single_crossing_on_regular_cells() {
    let expr = parse("x - .31").unwrap().expression().unwrap();
    let view = View {
        x: [-2.0, 2.0],
        y: [-2.0, 2.0],
        pixels: [128, 128],
    };
    let ctx = Context {
        program: &Program::default(),
        scope: &Scope::new(),
        control: controls(),
    };
    let plotted = area(&ctx, &expr, view, false).unwrap();
    let crossings: Vec<_> = plotted
        .cells
        .iter()
        .filter_map(|c| c.topology.as_ref())
        .collect();
    assert!(!crossings.is_empty());
    assert!(crossings.iter().all(|c| {
        c.axis == "x"
            && c.derivative[0] > 0.0
            && c.edge_brackets
                .iter()
                .all(|[a, b]| *a <= 0.31 && 0.31 <= *b)
    }));
}
