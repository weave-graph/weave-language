//! Bounded compile-time ordinary values. No graph reads or host effects.
use crate::interval::Interval as TimeInterval;
use crate::syntax::{Diagnostic, Span};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use weave_contract::{
    decimal::Decimal,
    quantity::{Quantity, UnitDescriptor},
};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalarType {
    Boolean,
    Integer,
    String,
    Time,
    Interval,
    Decimal,
    Quantity(UnitDescriptor),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ScalarValue {
    Boolean(bool),
    Integer(i64),
    String(String),
    Time(i64),
    Interval(TimeInterval),
    Decimal(Decimal),
    Quantity(Quantity),
}
impl ScalarValue {
    pub fn value_type(&self) -> ScalarType {
        match self {
            Self::Boolean(_) => ScalarType::Boolean,
            Self::Integer(_) => ScalarType::Integer,
            Self::String(_) => ScalarType::String,
            Self::Time(_) => ScalarType::Time,
            Self::Interval(_) => ScalarType::Interval,
            Self::Decimal(_) => ScalarType::Decimal,
            Self::Quantity(q) => ScalarType::Quantity(q.unit().clone()),
        }
    }
    pub(crate) fn size(&self) -> usize {
        match self {
            Self::String(s) => s.len() + 32,
            Self::Interval(_) => 128,
            Self::Quantity(q) => {
                q.unit().dimension_id().len()
                    + q.unit().unit_id().len()
                    + q.unit().revision().len()
                    + 128
            }
            _ => 64,
        }
    }
    pub fn json(&self) -> serde_json::Value {
        match self {
            Self::Boolean(v) => serde_json::json!(v),
            Self::Integer(v) | Self::Time(v) => serde_json::json!(v),
            Self::String(v) => serde_json::json!(v),
            Self::Interval(v) => serde_json::json!(v),
            Self::Decimal(v) => serde_json::json!(v),
            Self::Quantity(v) => serde_json::json!(v),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScalarExpr {
    pub span: Span,
    pub expression: ScalarExpression,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ScalarExpression {
    Literal(ScalarValue),
    Parameter(String),
    Value(String),
    Convert {
        input: Box<ScalarExpr>,
        conversion: Box<weave_contract::quantity::RationalConversion>,
    },
    Call {
        operator: String,
        arguments: Vec<ScalarExpr>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScalarReturn {
    pub value_type: ScalarType,
    pub value: ScalarExpr,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "literal", content = "value", rename_all = "snake_case")]
pub enum LiteralExpr {
    Json(serde_json::Value),
    Scalar(ScalarExpr),
    Array(Vec<LiteralExpr>),
    Object(BTreeMap<String, LiteralExpr>),
}
impl LiteralExpr {
    pub(crate) fn into_json(self) -> serde_json::Value {
        match self {
            Self::Json(v) => v,
            _ => unreachable!("literal references resolved before lowering"),
        }
    }
    pub(crate) fn resolve(
        &self,
        values: &BTreeMap<String, ScalarValue>,
        budget: &mut Budget,
    ) -> Result<serde_json::Value, Diagnostic> {
        Ok(match self {
            Self::Json(v) => v.clone(),
            Self::Scalar(e) => e.evaluate(&BTreeMap::new(), values, budget)?.json(),
            Self::Array(v) => serde_json::Value::Array(
                v.iter()
                    .map(|v| v.resolve(values, budget))
                    .collect::<Result<_, _>>()?,
            ),
            Self::Object(v) => serde_json::Value::Object(
                v.iter()
                    .map(|(k, v)| Ok((k.clone(), v.resolve(values, budget)?)))
                    .collect::<Result<_, Diagnostic>>()?,
            ),
        })
    }
}
pub(crate) fn error(code: &str, message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic::new(code, message, span.0, span.1)
}
#[derive(Default)]
pub(crate) struct Budget {
    steps: usize,
    bytes: usize,
    bindings: usize,
}
impl Budget {
    pub(crate) fn step(&mut self, span: Span) -> Result<(), Diagnostic> {
        self.steps += 1;
        if self.steps > 100_000 {
            return Err(error(
                "E_SCALAR_BUDGET",
                "Scalar evaluation exceeds 100000 steps",
                span,
            ));
        }
        Ok(())
    }
    pub(crate) fn charge(&mut self, bytes: usize, span: Span) -> Result<(), Diagnostic> {
        if bytes > 1_048_576 || self.bytes.saturating_add(bytes) > 4_194_304 {
            return Err(error(
                "E_SCALAR_BUDGET",
                "Scalar materialization exceeds bounded byte quota",
                span,
            ));
        }
        self.bytes += bytes;
        Ok(())
    }
    pub(crate) fn binding(&mut self, v: &ScalarValue, span: Span) -> Result<(), Diagnostic> {
        self.bindings += 1;
        if self.bindings > 10000 {
            return Err(error("E_SCALAR_BUDGET", "Too many scalar bindings", span));
        }
        self.charge(v.size(), span)
    }
}
fn output(operator: &str, args: &[ScalarType], span: Span) -> Result<ScalarType, Diagnostic> {
    use ScalarType::*;
    let expected = match operator {
        "time_equal" | "time_lt" if args == [Time, Time] => Some(Boolean),
        "interval" if args == [Time, Time] => Some(Interval),
        "interval_open" if args == [Time] => Some(Interval),
        "interval_start" | "interval_end" if args == [Interval] => Some(Time),
        "interval_contains" if args == [Interval, Time] => Some(Boolean),
        "interval_equal" | "interval_overlaps" | "interval_before" | "interval_meets"
        | "interval_within"
            if args == [Interval, Interval] =>
        {
            Some(Boolean)
        }
        "interval_intersection" if args == [Interval, Interval] => Some(Interval),
        "boolean_not" if args == [Boolean] => Some(Boolean),
        "boolean_and" | "boolean_or" if args == [Boolean, Boolean] => Some(Boolean),
        "integer_add" | "integer_sub" | "integer_mul" | "integer_div"
            if args == [Integer, Integer] =>
        {
            Some(Integer)
        }
        "integer_equal" | "integer_lt" if args == [Integer, Integer] => Some(Boolean),
        "string_concat" if args == [String, String] => Some(String),
        "string_equal" if args == [String, String] => Some(Boolean),
        "decimal_add" | "decimal_sub" | "decimal_mul" | "decimal_div"
            if args == [Decimal, Decimal] =>
        {
            Some(Decimal)
        }
        "decimal_equal" | "decimal_lt" if args == [Decimal, Decimal] => Some(Boolean),
        "quantity_add" | "quantity_sub" | "quantity_equal" | "quantity_lt"
            if args.len() == 2 && args[0] == args[1] && matches!(args[0], Quantity(_)) =>
        {
            Some(if operator.ends_with("equal") || operator.ends_with("lt") {
                Boolean
            } else {
                args[0].clone()
            })
        }
        "quantity_scale" | "quantity_div"
            if args.len() == 2 && matches!(args[0], Quantity(_)) && args[1] == Decimal =>
        {
            Some(args[0].clone())
        }
        _ => None,
    };
    expected.ok_or_else(|| {
        error(
            "E_SCALAR_TYPE",
            format!("Invalid typed operands for '{operator}'"),
            span,
        )
    })
}
impl ScalarExpr {
    pub(crate) fn retained_bytes(&self) -> usize {
        64 + match &self.expression {
            ScalarExpression::Literal(v) => v.size(),
            ScalarExpression::Parameter(n) | ScalarExpression::Value(n) => n.len(),
            ScalarExpression::Call {
                operator,
                arguments,
            } => operator.len() + arguments.iter().map(Self::retained_bytes).sum::<usize>(),
            ScalarExpression::Convert { input, conversion } => {
                input.retained_bytes()
                    + conversion.source().dimension_id().len()
                    + conversion.source().unit_id().len()
                    + conversion.source().revision().len()
                    + conversion.target().dimension_id().len()
                    + conversion.target().unit_id().len()
                    + conversion.target().revision().len()
                    + 128
            }
        }
    }
    pub(crate) fn check_constants(
        &self,
        values: &BTreeMap<String, ScalarValue>,
        budget: &mut Budget,
    ) -> Result<Option<ScalarValue>, Diagnostic> {
        budget.step(self.span)?;
        let closed = match &self.expression {
            ScalarExpression::Literal(_) => true,
            ScalarExpression::Parameter(_) => false,
            ScalarExpression::Value(n) => values.contains_key(n),
            ScalarExpression::Call { arguments, .. } => {
                let mut closed = true;
                for a in arguments {
                    closed &= a.check_constants(values, budget)?.is_some();
                }
                closed
            }
            ScalarExpression::Convert { input, .. } => {
                input.check_constants(values, budget)?.is_some()
            }
        };
        if closed {
            Ok(Some(self.evaluate(&BTreeMap::new(), values, budget)?))
        } else {
            Ok(None)
        }
    }
    pub(crate) fn infer(
        &self,
        params: &BTreeMap<String, ScalarType>,
        values: &BTreeMap<String, ScalarType>,
    ) -> Result<ScalarType, Diagnostic> {
        match &self.expression {
            ScalarExpression::Literal(v) => Ok(v.value_type()),
            ScalarExpression::Parameter(n) => params.get(n).cloned().ok_or_else(|| {
                error(
                    "E_FUNCTION_SCOPE",
                    format!("Unknown scalar parameter '{n}'"),
                    self.span,
                )
            }),
            ScalarExpression::Value(n) => values.get(n).cloned().ok_or_else(|| {
                error(
                    "E_SCALAR_UNKNOWN",
                    format!("Unknown scalar value '{n}'"),
                    self.span,
                )
            }),
            ScalarExpression::Convert { input, conversion } => {
                if input.infer(params, values)? != ScalarType::Quantity(conversion.source().clone())
                {
                    return Err(error(
                        "E_SCALAR_TYPE",
                        "Conversion requires its exact source unit",
                        self.span,
                    ));
                }
                Ok(ScalarType::Quantity(conversion.target().clone()))
            }
            ScalarExpression::Call {
                operator,
                arguments,
            } => output(
                operator,
                &arguments
                    .iter()
                    .map(|a| a.infer(params, values))
                    .collect::<Result<Vec<_>, _>>()?,
                self.span,
            ),
        }
    }
    pub(crate) fn evaluate(
        &self,
        params: &BTreeMap<String, ScalarValue>,
        values: &BTreeMap<String, ScalarValue>,
        budget: &mut Budget,
    ) -> Result<ScalarValue, Diagnostic> {
        budget.step(self.span)?;
        let value = match &self.expression {
            ScalarExpression::Literal(v) => {
                budget.charge(v.size(), self.span)?;
                v.clone()
            }
            ScalarExpression::Parameter(n) | ScalarExpression::Value(n) => {
                let map = if matches!(self.expression, ScalarExpression::Parameter(_)) {
                    params
                } else {
                    values
                };
                let v = map.get(n).ok_or_else(|| {
                    error(
                        "E_SCALAR_UNKNOWN",
                        format!("Unknown scalar value '{n}'"),
                        self.span,
                    )
                })?;
                budget.charge(v.size(), self.span)?;
                v.clone()
            }
            ScalarExpression::Convert { input, conversion } => {
                let v = input.evaluate(params, values, budget)?;
                let ScalarValue::Quantity(v) = v else {
                    return Err(error(
                        "E_SCALAR_TYPE",
                        "Conversion expects Quantity",
                        self.span,
                    ));
                };
                ScalarValue::Quantity(
                    v.convert(conversion)
                        .map_err(|e| error("E_NUMERIC", e.to_string(), self.span))?,
                )
            }
            ScalarExpression::Call {
                operator,
                arguments,
            } => {
                let args = arguments
                    .iter()
                    .map(|a| a.evaluate(params, values, budget))
                    .collect::<Result<Vec<_>, _>>()?;
                output(
                    operator,
                    &args.iter().map(ScalarValue::value_type).collect::<Vec<_>>(),
                    self.span,
                )?;
                use ScalarValue::*;
                let numeric = |message: &str| error("E_NUMERIC", message, self.span);
                match args.as_slice() {
                    [Time(a), Time(b)] => match operator.as_str() {
                        "time_equal" => Boolean(a == b),
                        "time_lt" => Boolean(a < b),
                        "interval" => Interval(
                            TimeInterval::new(*a, Some(*b))
                                .map_err(|e| error(e.code(), e.to_string(), self.span))?,
                        ),
                        _ => unreachable!("checked temporal signature"),
                    },
                    [Time(a)] => {
                        Interval(TimeInterval::new(*a, None).expect("unbounded interval is valid"))
                    }
                    [Interval(a)] => Time(match operator.as_str() {
                        "interval_start" => a.start(),
                        "interval_end" => a
                            .finite_end()
                            .map_err(|e| error(e.code(), e.to_string(), self.span))?,
                        _ => unreachable!("checked interval accessor"),
                    }),
                    [Interval(a), Time(b)] => Boolean(a.contains(*b)),
                    [Interval(a), Interval(b)] => match operator.as_str() {
                        "interval_equal" => Boolean(a == b),
                        "interval_overlaps" => Boolean(a.overlaps(*b)),
                        "interval_before" => Boolean(a.before(*b)),
                        "interval_meets" => Boolean(a.meets(*b)),
                        "interval_within" => Boolean(a.within(*b)),
                        "interval_intersection" => Interval(
                            a.intersection(*b)
                                .map_err(|e| error(e.code(), e.to_string(), self.span))?,
                        ),
                        _ => unreachable!("checked interval signature"),
                    },
                    [Boolean(a)] => Boolean(!a),
                    [Boolean(a), Boolean(b)] => Boolean(if operator == "boolean_and" {
                        *a && *b
                    } else {
                        *a || *b
                    }),
                    [Integer(a), Integer(b)] => match operator.as_str() {
                        "integer_equal" => Boolean(a == b),
                        "integer_lt" => Boolean(a < b),
                        _ => Integer(
                            match operator.as_str() {
                                "integer_add" => a.checked_add(*b),
                                "integer_sub" => a.checked_sub(*b),
                                "integer_mul" => a.checked_mul(*b),
                                _ => a
                                    .checked_rem(*b)
                                    .filter(|r| *r == 0)
                                    .and_then(|_| a.checked_div(*b)),
                            }
                            .ok_or_else(|| {
                                numeric("Integer overflow, zero divisor or inexact division")
                            })?,
                        ),
                    },
                    [String(a), String(b)] => {
                        if operator == "string_equal" {
                            Boolean(a == b)
                        } else {
                            budget.charge(a.len().saturating_add(b.len()) + 32, self.span)?;
                            String(format!("{a}{b}"))
                        }
                    }
                    [Decimal(a), Decimal(b)] => match operator.as_str() {
                        "decimal_equal" => Boolean(a == b),
                        "decimal_lt" => Boolean(decimal_compare(*a, *b).is_lt()),
                        _ => Decimal(
                            match operator.as_str() {
                                "decimal_add" => a.checked_add(*b),
                                "decimal_sub" => a.checked_sub(*b),
                                "decimal_mul" => a.checked_mul(*b),
                                _ => a.checked_div(*b),
                            }
                            .map_err(|e| numeric(&e.to_string()))?,
                        ),
                    },
                    [Quantity(a), Quantity(b)] => match operator.as_str() {
                        "quantity_equal" => Boolean(a == b),
                        "quantity_lt" => Boolean(decimal_compare(a.amount(), b.amount()).is_lt()),
                        _ => Quantity(
                            if operator == "quantity_add" {
                                a.checked_add(b)
                            } else {
                                a.checked_sub(b)
                            }
                            .map_err(|e| numeric(&e.to_string()))?,
                        ),
                    },
                    [Quantity(a), Decimal(b)] => Quantity(
                        if operator == "quantity_scale" {
                            a.checked_mul(*b)
                        } else {
                            a.checked_div(*b)
                        }
                        .map_err(|e| numeric(&e.to_string()))?,
                    ),
                    _ => unreachable!("checked scalar signature"),
                }
            }
        };
        Ok(value)
    }
    pub(crate) fn visit_references(
        &mut self,
        f: &mut impl FnMut(&mut String, Span, bool) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        match &mut self.expression {
            ScalarExpression::Parameter(n) => f(n, self.span, true),
            ScalarExpression::Value(n) => f(n, self.span, false),
            ScalarExpression::Convert { input, .. } => input.visit_references(f),
            ScalarExpression::Call { arguments, .. } => {
                for a in arguments {
                    a.visit_references(f)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    pub(crate) fn spans(&mut self, f: &mut impl FnMut(&mut Span)) {
        f(&mut self.span);
        if let ScalarExpression::Convert { input, .. } = &mut self.expression {
            input.spans(f);
        }
        if let ScalarExpression::Call { arguments, .. } = &mut self.expression {
            for a in arguments {
                a.spans(f);
            }
        }
    }
}
// Decimal's bounded canonical representation has <=18 coefficient digits and scale<=18.
// Padding canonical strings to a common scale fits signed i128 (<=36 digits).
fn decimal_compare(a: Decimal, b: Decimal) -> std::cmp::Ordering {
    fn parts(v: Decimal) -> (i128, u32) {
        let s = v.to_string();
        let scale = s.split_once('.').map_or(0, |(_, r)| r.len() as u32);
        (
            s.replace('.', "")
                .parse()
                .expect("bounded canonical decimal"),
            scale,
        )
    }
    let (a, sa) = parts(a);
    let (b, sb) = parts(b);
    let scale = sa.max(sb);
    (a * 10_i128.pow(scale - sa)).cmp(&(b * 10_i128.pow(scale - sb)))
}
