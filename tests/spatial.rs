use serde_json::{Value, json};

fn run(input: Value) -> Value {
    let raw = serde_json::to_vec(&input).unwrap();
    serde_json::from_slice(&lucent::request(&raw)).unwrap()
}

#[test]
fn sphere_vertices_approach_the_reference_surface() {
    let space = json!({"x":[-3,3],"y":[-3,3],"z":[-3,3]});
    let mesh = run(
        json!({"op":"isosurface","expr":"x^2+y^2+z^2=4","space":space,"depth":4,"budget":8000}),
    );
    assert_eq!(mesh["status"], "approximate_mesh");
    assert!(mesh["result"]["interval_excluded"].as_u64().unwrap() > 0);
    let faces = mesh["result"]["faces"].as_array().unwrap();
    assert!(faces.len() > 100);
    for face in faces {
        for point in face["vertices"].as_array().unwrap() {
            let r2: f64 = point
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap().powi(2))
                .sum();
            assert!((r2 - 4.0).abs() < 0.12);
        }
        let norm2: f64 = face["normal"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap().powi(2))
            .sum();
        assert!((norm2 - 1.0).abs() < 1e-12);
    }
}

#[test]
fn a_height_surface_records_holes_and_retains_valid_faces() {
    let mesh = run(
        json!({"op":"surface","expr":"sqrt(1-x^2-y^2)","space":{"x":[-2,2],"y":[-2,2],"z":[-1,1]},
        "depth":6,"budget":6000}),
    );
    assert_eq!(mesh["status"], "approximate_mesh");
    assert!(!mesh["result"]["faces"].as_array().unwrap().is_empty());
    assert!(
        mesh["result"]["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["excluded"] == true)
    );
    for face in mesh["result"]["faces"].as_array().unwrap() {
        for vertex in face["vertices"].as_array().unwrap() {
            let xy2 = vertex[0].as_f64().unwrap().powi(2) + vertex[1].as_f64().unwrap().powi(2);
            assert!(xy2 <= 1.0);
        }
    }
}
