use lucent::program::Program;

fn compile(sources: &[&str]) -> Result<Program, lucent::lang::Diagnostic> {
    Program::compile(
        &sources.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        ["amplitude"].into_iter(),
    )
}

#[test]
fn dependencies_are_lexical_and_topological() {
    let program = compile(&[
        "wave(t) := offset + amplitude*sin(t)",
        "offset := scale/2",
        "scale := 4",
    ])
    .unwrap();
    assert_eq!(program.order(), ["scale", "offset", "wave"]);
    assert_eq!(
        program.dependencies()["wave"].iter().collect::<Vec<_>>(),
        ["offset"]
    );
    assert!(
        compile(&["wave(t) := other(t)", "other(t) := wave(t)"])
            .unwrap_err()
            .message
            .contains("cycle")
    );
    assert!(compile(&["wave(t) := amplitude(t)"]).is_err());
    assert!(compile(&["wave(t) := t", "bad := wave"]).is_err());
    assert!(compile(&["x := 4"]).is_err());
}
