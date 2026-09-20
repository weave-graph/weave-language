//! Resolve declaration aliases before scalar checking and semantic identity hashing.
use crate::{
    scalars::{Budget, ScalarExpr, ScalarExpression, ScalarType, ScalarValue, error},
    syntax::{self, Diagnostic, ParameterKind, Span, Statement},
    vectors::{CheckedVector, VectorDescriptor},
};
use std::collections::BTreeMap;

pub(crate) fn local_types(s: &mut Statement, f: &mut impl FnMut(&mut ScalarType)) {
    fn parameter(p: &mut syntax::FunctionParameter, f: &mut impl FnMut(&mut ScalarType)) {
        if let ParameterKind::Scalar(t) = &mut p.kind {
            f(t);
        }
        if let Some(c) = &mut p.callback {
            parameter(&mut c.input, f);
            if let Some(t) = &mut c.output_type {
                f(t);
            }
        }
    }
    if let Statement::Function {
        parameters,
        scalar_return,
        ..
    } = s
    {
        for p in parameters {
            parameter(p, f);
        }
        if let Some(r) = scalar_return {
            f(&mut r.value_type);
        }
    }
}
pub(crate) fn local_exprs(s: &mut Statement, f: &mut impl FnMut(&mut ScalarExpr)) {
    match s {
        Statement::Function { scalar_return, .. } => {
            if let Some(r) = scalar_return {
                f(&mut r.value);
            }
        }
        Statement::Transaction { .. } => (),
        _ => syntax::scalar_expressions(s, f),
    }
}
fn expr_refs(e: &mut ScalarExpr, f: &mut impl FnMut(&mut String, &mut Span)) {
    match &mut e.expression {
        ScalarExpression::VectorLiteral {
            name, type_span, ..
        } => f(name, type_span),
        ScalarExpression::Call { arguments, .. } => {
            for a in arguments {
                expr_refs(a, f);
            }
        }
        ScalarExpression::Convert { input, .. } => expr_refs(input, f),
        _ => (),
    }
}
pub(crate) fn local_refs(s: &mut Statement, f: &mut impl FnMut(&mut String, &mut Span)) {
    local_types(s, &mut |t| {
        if let ScalarType::VectorReference { name, span } = t {
            f(name, span);
        }
    });
    local_exprs(s, &mut |e| expr_refs(e, f));
}
pub(crate) fn walk(s: &mut Statement, f: &mut impl FnMut(&mut Statement)) {
    f(s);
    if let Statement::Function { body, .. } | Statement::Transaction { body, .. } = s {
        for child in body {
            walk(child, f);
        }
    }
}
fn descriptor<'a>(
    types: &'a BTreeMap<String, VectorDescriptor>,
    name: &str,
    span: Span,
) -> Result<&'a VectorDescriptor, Diagnostic> {
    types.get(name).ok_or_else(|| {
        error(
            "E_VECTOR_DESCRIPTOR",
            format!("Unknown vector type '{name}'"),
            span,
        )
    })
}
fn resolve_expr(
    e: &mut ScalarExpr,
    types: &BTreeMap<String, VectorDescriptor>,
    budget: &mut Budget,
) -> Result<(), Diagnostic> {
    match &mut e.expression {
        ScalarExpression::VectorLiteral {
            name,
            type_span,
            values,
        } => {
            let d = descriptor(types, name, *type_span)?;
            budget.charge(d.size() + values.len() * 32 + 64, e.span)?;
            let value = CheckedVector::new(d.clone(), std::mem::take(values))
                .map_err(|v| error(v.0, v.to_string(), e.span))?;
            e.expression = ScalarExpression::Literal(ScalarValue::Vector(value));
        }
        ScalarExpression::Call { arguments, .. } => {
            for a in arguments {
                resolve_expr(a, types, budget)?;
            }
        }
        ScalarExpression::Convert { input, .. } => resolve_expr(input, types, budget)?,
        _ => (),
    }
    Ok(())
}
pub(crate) fn resolve(mut program: syntax::Program) -> Result<syntax::Program, Diagnostic> {
    let mut types = BTreeMap::new();
    let mut spaces = BTreeMap::new();
    let mut budget = Budget::default();
    for s in &program.statements {
        if let Statement::VectorType {
            name,
            name_span,
            definition,
        } = s
        {
            if types.len() >= 256 {
                return Err(error(
                    "E_VECTOR_BUDGET",
                    "At most 256 vector declarations",
                    *name_span,
                ));
            }
            if program
                .statements
                .iter()
                .filter(|s| crate::functions::declaration(s).0 == name)
                .count()
                != 1
            {
                return Err(error(
                    "E_DUPLICATE",
                    "Duplicate vector declaration name",
                    *name_span,
                ));
            }
            let key = (&definition.space().id, &definition.space().revision);
            if let Some(prior) = spaces.insert(key, &definition.space().geometry)
                && prior != &definition.space().geometry
            {
                return Err(error(
                    "E_VECTOR_DESCRIPTOR_CONFLICT",
                    "Same space ID/revision has different geometry",
                    *name_span,
                ));
            }
            budget.charge(definition.size() + name.len(), *name_span)?;
            types.insert(name.clone(), definition.clone());
        }
    }
    for s in &mut program.statements {
        let mut failure = None;
        walk(s, &mut |s| {
            local_types(s, &mut |t| {
                if let ScalarType::VectorReference { name, span } = t {
                    match descriptor(&types, name, *span).and_then(|d| {
                        budget.charge(d.size(), *span)?;
                        Ok(d.clone())
                    }) {
                        Ok(d) => *t = ScalarType::Vector(d),
                        Err(e) => failure = Some(e),
                    }
                }
            });
            local_exprs(s, &mut |e| {
                if let Err(e) = resolve_expr(e, &types, &mut budget) {
                    failure = Some(e);
                }
            });
        });
        if let Some(e) = failure {
            return Err(e);
        }
    }
    program
        .statements
        .retain(|s| !matches!(s, Statement::VectorType { .. }));
    Ok(program)
}
