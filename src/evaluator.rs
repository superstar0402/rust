use std::{collections::BTreeMap, sync::Arc, ops::Add};

use ordered_float::OrderedFloat;
use time::Date;

use crate::{ast, models::DataValue};



#[derive(Debug)]
pub enum EvaluationError {
    DivideByZero,
    InvalidType,
    UnknownIdentifier(String),
    UnknownFunction(String),
    InvalidArgumentCount(String),
}


pub type QueryVariables = BTreeMap<Arc<str>, DataValue>;

#[derive(Debug, Clone)]
pub struct ExpressionEvaluationContext {
  variables: QueryVariables,
  
}

impl ExpressionEvaluationContext {

  pub fn new(variables: QueryVariables) -> ExpressionEvaluationContext {
    ExpressionEvaluationContext {
      variables,
    }
  }

  pub fn replace_variables(&mut self, new_data: QueryVariables) {
    self.variables = new_data;
  }

  pub fn get_variable(&self, name: &str) -> Option<&DataValue> {
    self.variables.get(name)
  }

  pub fn clone_variables(&self) -> QueryVariables {
    self.variables.clone()
  }
  
}

pub struct ExpressionEvaluator {
}

impl ExpressionEvaluator {

    pub fn new() -> ExpressionEvaluator {
        ExpressionEvaluator {  }
    }

    pub fn evaluate_expression(
        &self,
        context: &ExpressionEvaluationContext,
        expression: &ast::Expression,
    ) -> Result<DataValue, EvaluationError> {
        match expression {
            ast::Expression::UnaryExpression(expression) => {
                self.evaluate_unary_expression(context, expression)
            }
            ast::Expression::BinaryExpression(expression) => {
                self.evaluate_binary_expression(context, expression)
            }
            _ => todo!(),
        }
    }

    pub fn evaluate_predicate(
        &self,
        context: &ExpressionEvaluationContext,
        expression: &ast::Expression,
    ) -> Result<bool, EvaluationError> {
        let value = self.evaluate_expression(context, expression)?;
        match value {
            DataValue::Bool(b) => Ok(b),
            _ => Ok(false),
        }
    }

    pub fn evaluate_projection_field(
        &self,
        context: &ExpressionEvaluationContext,
        expression: &ast::Expression,
    ) -> Result<(String, DataValue), EvaluationError> {
        let value = self.evaluate_expression(context, expression)?;
        let alias = match expression {
            ast::Expression::UnaryExpression(expression) => match expression {
                ast::UnaryExpression::Property { name: _, key } => key,
                ast::UnaryExpression::Parameter(p) => p,
                ast::UnaryExpression::Alias { source: _, alias } => alias,
                ast::UnaryExpression::Identifier(id) => id,
                _ => "expression",
            },
            ast::Expression::BinaryExpression(_) => "expression",
            _ => todo!()
            // ast::Expression::FunctionExpression(f) => f.name,
            // ast::Expression::CaseExpression(_) => "case",
            // ast::Expression::ListExpression(_) => "list",
        };

        Ok((alias.to_string(), value))
    }

    fn evaluate_unary_expression(
        &self,
        context: &ExpressionEvaluationContext,
        expression: &ast::UnaryExpression,
    ) -> Result<DataValue, EvaluationError> {
        let result = match expression {
            ast::UnaryExpression::Not(expression) => {
                DataValue::Bool(!self.evaluate_predicate(context, expression)?)
            }
            ast::UnaryExpression::Exists(_) => todo!(),
            ast::UnaryExpression::IsNull(e) => DataValue::Bool(self.evaluate_expression(context, e)?.is_null()),
            ast::UnaryExpression::IsNotNull(e) => DataValue::Bool(!self.evaluate_expression(context, e)?.is_null()),
            ast::UnaryExpression::Literal(l) => match l {
                ast::Literal::Boolean(b) => DataValue::Bool(*b),
                ast::Literal::Text(t) => DataValue::String(t.to_string()),
                ast::Literal::Null => DataValue::Null,
                ast::Literal::Integer(i) => DataValue::Int(*i),
                ast::Literal::Real(r) => DataValue::Float(OrderedFloat::from(*r)),
                ast::Literal::Date(d) => DataValue::Date(*d),
                
            },
            ast::UnaryExpression::Property { name, key } => match context.get_variable(name) {
                Some(v) => match v {
                    DataValue::Map(o) => match o.get(key) {
                        Some(v) => v.clone(),
                        None => DataValue::Null,
                    },
                    _ => DataValue::Null,
                },
                None => DataValue::Null,
            },
            ast::UnaryExpression::Parameter(p) => match context.get_variable(p) {
                Some(v) => v.clone(),
                None => DataValue::Null,
            },
            ast::UnaryExpression::Alias { source, alias: _ } => {
                self.evaluate_expression(context, source)?
            }
            ast::UnaryExpression::Identifier(ident) => match context.get_variable(ident) {
                Some(value) => value.clone(),
                None => return Err(EvaluationError::UnknownIdentifier(ident.to_string())),
            },
        };
        Ok(result)
    }

    fn evaluate_binary_expression(
        &self,
        context: &ExpressionEvaluationContext,
        expression: &ast::BinaryExpression,
    ) -> Result<DataValue, EvaluationError> {
        let result = match expression {
            ast::BinaryExpression::And(c1, c2) => DataValue::Bool(
                self.evaluate_predicate(context, c1)? && self.evaluate_predicate(context, c2)?,
            ),
            ast::BinaryExpression::Or(c1, c2) => DataValue::Bool(
                self.evaluate_predicate(context, c1)? || self.evaluate_predicate(context, c2)?,
            ),
            ast::BinaryExpression::Eq(e1, e2) => match (
                self.evaluate_expression(context, e1)?,
                self.evaluate_expression(context, e2)?,
            ) {
                (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Bool(n1 == n2),
                (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Bool(n1 == n2),
                (DataValue::Date(n1), DataValue::Date(n2)) => DataValue::Bool(n1 == n2),
                (DataValue::String(s1), DataValue::String(s2)) => DataValue::Bool(s1 == s2),
                (DataValue::Bool(b1), DataValue::Bool(b2)) => DataValue::Bool(b1 == b2),
                (DataValue::Null, DataValue::Null) => DataValue::Bool(true),
                //(QueryValue::List(a1), QueryValue::List(a2)) => QueryValue::Bool(a1 == a2),
                _ => DataValue::Bool(false),
            },
            ast::BinaryExpression::Ne(e1, e2) => match (
                self.evaluate_expression(context, e1)?,
                self.evaluate_expression(context, e2)?,
            ) {
                (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Bool(n1 != n2),
                (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Bool(n1 != n2),
                (DataValue::Date(n1), DataValue::Date(n2)) => DataValue::Bool(n1 != n2),
                (DataValue::String(s1), DataValue::String(s2)) => DataValue::Bool(s1 != s2),
                (DataValue::Bool(b1), DataValue::Bool(b2)) => DataValue::Bool(b1 != b2),
                (DataValue::Null, DataValue::Null) => DataValue::Bool(false),
                //(QueryValue::List(a1), QueryValue::List(a2)) => QueryValue::Bool(a1 != a2),
                _ => DataValue::Bool(false),
            },
            ast::BinaryExpression::Lt(e1, e2) => match (
                self.evaluate_expression(context, e1)?,
                self.evaluate_expression(context, e2)?,
            ) {
                (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Bool(n1 < n2),
                (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Bool(n1 < n2),
                (DataValue::Date(n1), DataValue::Date(n2)) => DataValue::Bool(n1 < n2),
                _ => DataValue::Bool(false),
            },
            ast::BinaryExpression::Le(e1, e2) => match (
                self.evaluate_expression(context, e1)?,
                self.evaluate_expression(context, e2)?,
            ) {
                (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Bool(n1 <= n2),
                (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Bool(n1 <= n2),
                (DataValue::Date(n1), DataValue::Date(n2)) => DataValue::Bool(n1 <= n2),
                _ => DataValue::Bool(false),
            },
            ast::BinaryExpression::Gt(e1, e2) => match (
                self.evaluate_expression(context, e1)?,
                self.evaluate_expression(context, e2)?,
            ) {
                (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Bool(n1 > n2),
                (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Bool(n1 > n2),
                (DataValue::Date(n1), DataValue::Date(n2)) => DataValue::Bool(n1 > n2),
                _ => DataValue::Bool(false),
            },
            ast::BinaryExpression::Ge(e1, e2) => match (
                self.evaluate_expression(context, e1)?,
                self.evaluate_expression(context, e2)?,
            ) {
                (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Bool(n1 >= n2),
                (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Bool(n1 >= n2),
                (DataValue::Date(n1), DataValue::Date(n2)) => DataValue::Bool(n1 >= n2),
                _ => DataValue::Bool(false),
            },
            ast::BinaryExpression::Add(e1, e2) => {
                let n1 = self.evaluate_expression(context, e1)?;
                let n2 = self.evaluate_expression(context, e2)?;
                match (n1, n2) {
                    (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Int(n1 + n2),
                    (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Float(n1 + n2),
                    (DataValue::Int(n1), DataValue::Float(n2)) => DataValue::Float(OrderedFloat::from(n1 as f64) + n2),
                    (DataValue::Float(n1), DataValue::Int(n2)) => DataValue::Float(n1 + n2 as f64),
                    //(QueryValue::Date(d1), QueryValue::Date(d2)) => QueryValue::Date(d1.add(d2)),

                    (DataValue::Int(n1), DataValue::String(s2)) => DataValue::String(n1.to_string() + &s2),
                    (DataValue::String(s1), DataValue::Bool(b2)) => DataValue::String(s1 + &b2.to_string()),
                    (DataValue::String(s1), DataValue::Int(n2)) => DataValue::String(s1 + &n2.to_string()),
                    (DataValue::String(s1), DataValue::String(s2)) => DataValue::String(s1 + &s2),
                    _ => DataValue::Null,
                }
            }
            ast::BinaryExpression::Subtract(e1, e2) => {
                let n1 = self.evaluate_expression(context, e1)?;
                let n2 = self.evaluate_expression(context, e2)?;
                match (n1, n2) {
                    (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Int(n1 - n2),
                    (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Float(n1 - n2),
                    (DataValue::Int(n1), DataValue::Float(n2)) => DataValue::Float(OrderedFloat::from(n1 as f64) - n2),
                    (DataValue::Float(n1), DataValue::Int(n2)) => DataValue::Float(n1 - n2 as f64),
                    _ => DataValue::Null,
                }
            }
            ast::BinaryExpression::Multiply(e1, e2) => {
                let n1 = self.evaluate_expression(context, e1)?;
                let n2 = self.evaluate_expression(context, e2)?;
                match (n1, n2) {
                    (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Int(n1 * n2),
                    (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Float(n1 * n2),
                    (DataValue::Int(n1), DataValue::Float(n2)) => DataValue::Float(OrderedFloat::from(n1 as f64) * n2),
                    (DataValue::Float(n1), DataValue::Int(n2)) => DataValue::Float(n1 * n2 as f64),
                    _ => DataValue::Null,
                }
            }
            ast::BinaryExpression::Divide(e1, e2) => {
                let n1 = self.evaluate_expression(context, e1)?;
                let n2 = self.evaluate_expression(context, e2)?;
                match (n1, n2) {
                    (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Int(n1 / n2),
                    (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Float(n1 / n2),
                    (DataValue::Int(n1), DataValue::Float(n2)) => DataValue::Float(OrderedFloat::from(n1 as f64) / n2),
                    (DataValue::Float(n1), DataValue::Int(n2)) => DataValue::Float(n1 / n2 as f64),
                    _ => DataValue::Null,
                }
            }
            ast::BinaryExpression::In(e1, e2) => {
                let e1 = self.evaluate_expression(context, e1)?;
                match self.evaluate_expression(context, e2)? {
                    DataValue::List(a) => DataValue::Bool(a.contains(&e1)),
                    _ => return Err(EvaluationError::InvalidType),
                }                
            },
            ast::BinaryExpression::Modulo(e1, e2) => {
                let n1 = self.evaluate_expression(context, e1)?;
                let n2 = self.evaluate_expression(context, e2)?;
                match (n1, n2) {
                    (DataValue::Int(n1), DataValue::Int(n2)) => DataValue::Int(n1 % n2),
                    (DataValue::Float(n1), DataValue::Float(n2)) => DataValue::Float(n1 % n2),
                    (DataValue::Int(n1), DataValue::Float(n2)) => DataValue::Float(OrderedFloat::from(n1 as f64) % n2),
                    (DataValue::Float(n1), DataValue::Int(n2)) => DataValue::Float(n1 % n2 as f64),
                    _ => DataValue::Null,
                }
            },
            ast::BinaryExpression::Exponent(e1, e2) => {
                let n1 = self.evaluate_expression(context, e1)?;
                let n2 = self.evaluate_expression(context, e2)?;
                todo!()
            },
        };
        Ok(result)
    }

    fn evaluate_function_expression(
        &self,
        context: &ExpressionEvaluationContext,
        expression: &ast::FunctionExpression,
    ) -> Result<DataValue, EvaluationError> {
        let mut values = Vec::new();
        for arg in &expression.args {
            values.push(self.evaluate_expression(context, arg)?);
        }

        todo!()
    }

    fn evaluate_case_expression(
        &self,
        context: &ExpressionEvaluationContext,
        expression: &ast::CaseExpression,
    ) -> Result<DataValue, EvaluationError> {
        let match_ = match expression.match_ {
            Some(ref match_) => Some(self.evaluate_expression(context, match_)?),
            None => None,
        };

        for when in &expression.when {
            match match_ {
                Some(ref match_) => {
                    let condition = self.evaluate_expression(context, &when.0)?;
                    if condition == *match_ {
                        return Ok(self.evaluate_expression(context, &when.1)?);
                    }
                }
                None => {
                    let condition = self.evaluate_predicate(context, &when.0)?;
                    if condition {
                        return Ok(self.evaluate_expression(context, &when.1)?);
                    }
                }
            }
        }

        match expression.else_ {
            Some(ref else_) => Ok(self.evaluate_expression(context, else_)?),
            None => Ok(DataValue::Null),
        }
    }

    fn evaluate_list_expression(&self, context: &ExpressionEvaluationContext, expression: &ast::ListExpression) -> Result<DataValue, EvaluationError> {
        let mut result = Vec::new();
        for e in &expression.elements {
            result.push(self.evaluate_expression(context, e)?);
        }
        
        Ok(DataValue::List(result))
    }
}