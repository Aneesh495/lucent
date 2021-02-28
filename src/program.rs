use std::collections::{BTreeMap, BTreeSet};

use crate::lang::{Diagnostic, Expr, Kind, Span, Statement, parse};

#[derive(Clone, Debug)]
pub struct Definition {
    pub params: Option<Vec<String>>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Clone, Default, Debug)]
pub struct Program {
    definitions: BTreeMap<String, Definition>,
    dependencies: BTreeMap<String, BTreeSet<String>>,
    order: Vec<String>,
}

const AXES: &[&str] = &["x", "y", "z", "t", "u", "v"];

pub fn constant(name: &str) -> bool {
    matches!(
        name,
        "pi" | "e" | "i" | "true" | "false" | "m" | "cm" | "km" | "s" | "ms" | "kg" | "rad" | "deg"
    )
}

pub fn builtin(name: &str) -> bool {
    matches!(
        name,
        "if" | "sum"
            | "product"
            | "sqrt"
            | "exp"
            | "ln"
            | "log"
            | "sin"
            | "cos"
            | "tan"
            | "atan"
            | "abs"
            | "erf"
            | "erfc"
            | "complex"
            | "re"
            | "im"
            | "conj"
            | "interval"
            | "dot"
            | "transpose"
            | "det"
            | "solve"
            | "normal_cdf"
            | "normal_sf"
            | "self_quotient"
            | "self_difference"
    )
}

impl Program {
    pub fn compile(
        sources: &[String],
        parameters: impl Iterator<Item = impl AsRef<str>>,
    ) -> Result<Self, Diagnostic> {
        let empty = Span { start: 0, end: 0 };
        if sources.len() > 64 {
            return Err(Diagnostic::at(empty, "At most 64 definitions are allowed"));
        }
        let parameters: BTreeSet<_> = parameters.map(|name| name.as_ref().to_owned()).collect();
        for name in &parameters {
            if !valid_parameter(name) {
                return Err(Diagnostic::at(
                    empty,
                    format!("Invalid or reserved parameter {name}"),
                ));
            }
        }
        let mut program = Self::default();
        for source in sources {
            let Statement::Definition {
                name,
                params,
                body,
                span,
            } = parse(source)?
            else {
                return Err(Diagnostic::at(empty, "A definition uses := for assignment"));
            };
            if !valid_parameter(&name) || parameters.contains(&name) {
                return Err(Diagnostic::at(
                    span,
                    format!("{name} is reserved or already a parameter"),
                ));
            }
            if program
                .definitions
                .insert(name.clone(), Definition { params, body, span })
                .is_some()
            {
                return Err(Diagnostic::at(span, format!("Duplicate definition {name}")));
            }
        }
        for (name, def) in &program.definitions {
            let locals: BTreeSet<_> = def.params.iter().flatten().map(String::as_str).collect();
            let mut deps = BTreeSet::new();
            program.references(&def.body, &locals, &parameters, &mut deps)?;
            program.dependencies.insert(name.clone(), deps);
        }
        let mut active = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for name in program.definitions.keys() {
            visit(
                name,
                &program.dependencies,
                &program.definitions,
                &mut active,
                &mut visited,
                &mut program.order,
            )?;
        }
        Ok(program)
    }

    pub fn get(&self, name: &str) -> Option<&Definition> {
        self.definitions.get(name)
    }
    pub fn dependencies(&self) -> &BTreeMap<String, BTreeSet<String>> {
        &self.dependencies
    }
    pub fn order(&self) -> &[String] {
        &self.order
    }

    fn references(
        &self,
        expr: &Expr,
        locals: &BTreeSet<&str>,
        params: &BTreeSet<String>,
        output: &mut BTreeSet<String>,
    ) -> Result<(), Diagnostic> {
        match &expr.kind {
            Kind::Literal(_) => {}
            Kind::Name(name) => {
                if locals.contains(name.as_str())
                    || params.contains(name)
                    || constant(name)
                    || AXES.contains(&name.as_str())
                {
                    return Ok(());
                }
                let Some(def) = self.get(name) else {
                    return Err(Diagnostic::at(expr.span, format!("Unknown name {name}")));
                };
                if def.params.is_some() {
                    return Err(Diagnostic::at(
                        expr.span,
                        format!("{name} needs function arguments"),
                    ));
                }
                output.insert(name.clone());
            }
            Kind::Call(name, args) => {
                if !builtin(name) {
                    let Some(def) = self.get(name) else {
                        return Err(Diagnostic::at(
                            expr.span,
                            format!("Unknown function {name}"),
                        ));
                    };
                    if def.params.as_ref().is_none_or(|p| p.len() != args.len()) {
                        return Err(Diagnostic::at(
                            expr.span,
                            format!("Invalid argument count for {name}"),
                        ));
                    }
                    output.insert(name.clone());
                }
                for arg in args {
                    self.references(arg, locals, params, output)?;
                }
            }
            Kind::Negative(value) => self.references(value, locals, params, output)?,
            Kind::Binary(_, a, b) => {
                self.references(a, locals, params, output)?;
                self.references(b, locals, params, output)?;
            }
            Kind::List(items) => {
                for item in items {
                    self.references(item, locals, params, output)?;
                }
            }
        }
        Ok(())
    }
}

fn valid_parameter(name: &str) -> bool {
    let mut chars = name.bytes();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
        && !constant(name)
        && !builtin(name)
        && !AXES.contains(&name)
}

fn visit(
    name: &str,
    deps: &BTreeMap<String, BTreeSet<String>>,
    definitions: &BTreeMap<String, Definition>,
    active: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) -> Result<(), Diagnostic> {
    if visited.contains(name) {
        return Ok(());
    }
    if !active.insert(name.to_owned()) {
        return Err(Diagnostic::at(
            definitions[name].span,
            format!("Dependency cycle through {name}"),
        ));
    }
    for dep in &deps[name] {
        visit(dep, deps, definitions, active, visited, order)?;
    }
    active.remove(name);
    visited.insert(name.to_owned());
    order.push(name.to_owned());
    Ok(())
}
