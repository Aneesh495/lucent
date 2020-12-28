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
