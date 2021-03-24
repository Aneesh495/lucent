use serde_json::{Value, json};

fn run(req: Value) -> Value {
    serde_json::from_slice(&lucent::request(&serde_json::to_vec(&req).unwrap())).unwrap()
}

#[test]
fn production_requests_preserve_exact_and_enclosure_statuses() {
    let exact = run(json!({"op":"evaluate","expr":"f(3)+.1","definitions":["f(u):=u/7"]}));
    assert_eq!(exact["status"], "exact");
    assert_eq!(exact["result"]["value"], "37/70");
    let enclosure = run(json!({"op":"evaluate","expr":"(exp(1)+1e-40-exp(1))/1e-40"}));
    assert_eq!(enclosure["status"], "enclosure");
    assert_eq!(enclosure["target_met"], true);
    assert!(enclosure["precision"].as_u64().unwrap() > 96);
    let domain = run(json!({"op":"enclose","expr":"1/x","domain":[-1,1]}));
    assert_eq!(domain["status"], "unresolved");
}

#[test]
fn semantic_claim_rejects_excluded_true_value() {
    let request = json!({"op":"evaluate","expr":"sqrt(2)"});
    let mut claim = run(request.clone());
    assert_eq!(
        run(json!({"op":"verify","request":request,"claim":claim}))["status"],
        "verified"
    );
    claim["result"]["bounds"] = json!([1.415, 1.416]);
    assert_eq!(
        run(json!({"op":"verify","request":request,"claim":claim}))["status"],
        "rejected"
    );
}

#[test]
fn derivatives_and_improper_integrals_use_distinct_contracts() {
    let comparison = run(json!({"op":"derivative","expr":"exp(x^2)","at":0.9}));
    assert_eq!(comparison["status"], "derivative");
    let a = comparison["result"]["automatic"].as_array().unwrap();
    let s = comparison["result"]["symbolic"].as_array().unwrap();
    assert!(a[0].as_f64().unwrap() <= s[1].as_f64().unwrap());
    assert!(s[0].as_f64().unwrap() <= a[1].as_f64().unwrap());
    let tail = run(json!({"op":"improper","expr":"1/x^3","lower":2}));
    assert_eq!(tail["result"]["enclosure"], json!([0.125, 0.125]));
    assert_eq!(
        run(json!({"op":"improper","expr":"1/x","lower":1}))["status"],
        "divergent"
    );
    assert_eq!(
        run(json!({"op":"improper","expr":"sin(x)/x","lower":1}))["status"],
        "unresolved"
    );
}

#[test]
fn principal_complex_values_keep_the_cut_convention() {
    let log = run(json!({"op":"evaluate","expr":"log(-1+0i)"}));
    assert_eq!(log["status"], "complex");
    let phase = log["result"]["imaginary"]["bounds"].as_array().unwrap();
    assert!(phase[0].as_f64().unwrap() <= std::f64::consts::PI);
    assert!(phase[1].as_f64().unwrap() >= std::f64::consts::PI);
    let sqrt = run(json!({"op":"evaluate","expr":"sqrt(-4+0i)"}));
    assert_eq!(sqrt["result"]["imaginary"]["value"], "2");
    let across = run(json!({"op":"evaluate","expr":"log(complex(-2,interval(-.1,.1)))"}));
    assert_eq!(across["status"], "unresolved");
    let raster = run(
        json!({"op":"color","expr":"sqrt(z)","budget":121,"view":{"x":[-2,2],"y":[-2,2],"pixels":[200,200]}}),
    );
    assert_eq!(raster["status"], "approximate_coloring");
    assert_eq!(raster["result"]["rgba"].as_array().unwrap().len(), 121 * 4);
}

#[test]
fn ode_trajectory_and_initial_step_box_are_distinct() {
    let orbit = run(
        json!({"op":"ode","dx":"-y","dy":"x","initial":[1,0],"time":[0,2],
        "event":"x","validated":true,"tolerance":1e-8}),
    );
    assert_eq!(orbit["status"], "approximate_trajectory");
    let time = orbit["result"]["event"]["sample"]["t"].as_f64().unwrap();
    assert!((time - std::f64::consts::FRAC_PI_2).abs() < 1e-6);
    let tube = &orbit["result"]["local_box"];
    assert!(tube["contraction_upper"].as_f64().unwrap() < 1.0);
    let step = tube["time"][1].as_f64().unwrap();
    let x = &tube["endpoint"][0];
    assert!(x[0].as_f64().unwrap() < step.cos() && step.cos() < x[1].as_f64().unwrap());
    let bad = run(
        json!({"op":"ode","dx":"1/(x-x)","dy":"0","initial":[1,0],"time":[0,1],"tolerance":1e-6,"budget":12}),
    );
    assert_eq!(bad["status"], "partial");
    assert_eq!(bad["result"]["stiffness_warning"], false);
    let fast = run(
        json!({"op":"ode","dx":"-x","dy":"-1000y","initial":[1,1],"time":[0,1],"tolerance":1e-7,"budget":1600}),
    );
    assert_eq!(fast["result"]["stiffness_warning"], true);
}

#[test]
fn feature_tools_expose_solved_equations_and_tangency_evidence() {
    let intersection =
        run(json!({"op":"feature","kind":"intersection","expr":"x^2","other":"2","domain":[1,2]}));
    assert_eq!(intersection["equation"], "((x ^ 2) - 2)");
    assert_eq!(
        intersection["result"]["unique"].as_array().unwrap().len(),
        1
    );
    let tangent =
        run(json!({"op":"feature","kind":"tangency","expr":"x^2","other":"0","domain":[-1,1]}));
    assert_eq!(tangent["result"]["points"][0]["classification"], "tangency");
    assert_eq!(
        tangent["result"]["points"][0]["root"]["bracket"],
        json!([0.0, 0.0])
    );
    let no_tangent =
        run(json!({"op":"feature","kind":"tangency","expr":"x^2+1","other":"0","domain":[-1,1]}));
    assert!(
        no_tangent["result"]["points"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let area =
        run(json!({"op":"feature","kind":"area_between","expr":"3","other":"x","domain":[0,1]}));
    let bounds = area["result"]["enclosure"].as_array().unwrap();
    let lo = bounds[0].as_f64().unwrap();
    let hi = bounds[1].as_f64().unwrap();
    assert!(lo <= 2.5 && 2.5 <= hi && hi - lo <= 1e-12);
}

#[test]
fn root_verifier_rejects_omitted_coverage() {
    let request = json!({"op":"roots","expr":"x^2-2","domain":[-3,3]});
    let mut claim = run(request.clone());
    assert_eq!(
        run(json!({"op":"verify","request":request,"claim":claim}))["status"],
        "verified"
    );
    claim["result"]["unique"].as_array_mut().unwrap().pop();
    assert_eq!(
        run(json!({"op":"verify","request":request,"claim":claim}))["status"],
        "rejected"
    );
}

#[test]
fn coordinate_cancellation_keeps_domain_guards() {
    let simple = run(json!({"op":"simplify","expr":"x/x+if(true, 1/3+.2, 1/0)"}));
    let source = simple["equation"].as_str().unwrap();
    assert!(source.contains("self_quotient(x)"));
    let enclosure = run(json!({"op":"enclose","expr":source,"domain":[-1,1]}));
    assert_eq!(enclosure["status"], "unresolved");
    let positive = run(json!({"op":"enclose","expr":source,"domain":[1,2]}));
    assert_eq!(positive["result"]["value"], "23/15");
    let independent = run(json!({"op":"simplify","expr":"interval(1,2)-interval(1,2)"}));
    assert_eq!(independent["result"]["guarded_identities"], 0);
}

#[test]
fn real_rational_powers_check_exact_roots_branches_and_units() {
    assert_eq!(
        run(json!({"op":"evaluate","expr":"(-8)^(2/3)"}))["result"]["value"],
        "4"
    );
    assert_eq!(
        run(json!({"op":"evaluate","expr":"(-8)^(-1/3)"}))["result"]["value"],
        "-1/2"
    );
    assert_eq!(
        run(json!({"op":"evaluate","expr":"(16m^4)^(1/4)"}))["result"]["dimension"],
        json!([1, 0, 0, 0])
    );
    let range = run(json!({"op":"enclose","expr":"x^(1/3)","domain":[-8,27]}));
    assert_eq!(range["result"]["bounds"], json!([-2.0, 3.0]));
    assert_eq!(
        run(json!({"op":"enclose","expr":"x^(1/2)","domain":[-8,27]}))["status"],
        "unresolved"
    );
    assert_eq!(
        run(json!({"op":"derivative","expr":"x^(3/2)","at":0}))["status"],
        "unresolved"
    );
    let derivative = run(json!({"op":"derivative","expr":"x^(1/3)","at":-8}));
    assert_eq!(derivative["status"], "derivative");
    let actual = derivative["result"]["automatic"].as_array().unwrap();
    assert!(actual[0].as_f64().unwrap() <= 1.0 / 12.0 && 1.0 / 12.0 <= actual[1].as_f64().unwrap());
}
