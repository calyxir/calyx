use std::fmt::Display;

use evalexpr::{
    eval_int_with_context, ContextWithMutableVariables, HashMapContext, Value,
};

const DEFINE_DIRECTIVE: &str = "$define";
const EVAL: char = '$';
const START_LINE_COMMENT: &str = "// ";

fn is_valid_macro_char(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

#[derive(PartialEq, Eq, Debug)]
pub struct PreprocessingError {
    msg: String,
}

impl PreprocessingError {
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}

impl Display for PreprocessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.msg.fmt(f)
    }
}

pub struct Context {
    env: HashMapContext,
}

pub type ContextResult<T> = Result<T, PreprocessingError>;

impl Context {
    pub fn new() -> Self {
        Self {
            env: HashMapContext::new(),
        }
    }

    pub fn define(&mut self, name: String, value: Value) -> ContextResult<()> {
        self.env
            .set_value(name, value)
            .map_err(|err| PreprocessingError::new(err.to_string()))
    }

    pub fn process(&mut self, line: String) -> ContextResult<String> {
        if line.starts_with(DEFINE_DIRECTIVE) {
            let (_, later) = line.split_at(DEFINE_DIRECTIVE.len());
            let later = later.trim();
            let space_index =
                later.find(' ').ok_or(PreprocessingError::new(format!(
                    "missing space after macro name in {} directive",
                    DEFINE_DIRECTIVE
                )))?;
            let (name, value) = later.split_at(space_index);
            let value = value.trim();
            if !name.chars().all(is_valid_macro_char) {
                Err(PreprocessingError::new("macro names can only contain alphabetic characters and unerscores".into()))?;
            }
            self.env
                .set_value(name.to_string(), Value::Int(self.eval(value)?))
                .map_err(|err| PreprocessingError::new(err.to_string()))?;
            Ok(format!("{}{}", START_LINE_COMMENT, line))
        } else {
            self.eval_line(line)
        }
    }

    fn eval_line(&mut self, line: String) -> ContextResult<String> {
        let mut result_line = String::new();
        let mut expr_acc = String::new();
        let mut paren_count = 0;
        let mut in_expr = false;

        let chars: Vec<_> = line.chars().collect();
        let next_chars = chars.iter().skip(1).chain(std::iter::once(&'\0'));
        for (c, next_c) in chars.iter().zip(next_chars) {
            let (c, next_c) = (*c, *next_c);
            if in_expr {
                expr_acc.push(c);
                if c == '(' {
                    paren_count += 1;
                } else if c == ')' {
                    paren_count -= 1;
                    if paren_count == 0 {
                        let result = self.eval(&expr_acc)?;
                        result_line.push_str(&result.to_string());
                        expr_acc.clear();
                        in_expr = false;
                    }
                } else if paren_count == 0 && (!is_valid_macro_char(next_c)) {
                    let result = self.eval(&expr_acc)?;
                    result_line.push_str(&result.to_string());
                    expr_acc.clear();
                    in_expr = false;
                }
            } else {
                if c == EVAL {
                    in_expr = true;
                } else {
                    result_line.push(c);
                }
            }
        }

        if paren_count != 0 {
            return Err(PreprocessingError::new(
                "unmatched parentheses in expression".into(),
            ))?;
        }

        Ok(result_line)
    }

    fn eval(&self, expr: &str) -> ContextResult<i64> {
        eval_int_with_context(expr, &self.env)
            .map_err(|err| PreprocessingError::new(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_define_directive() {
        let mut context = Context::new();
        let result = context.process("$define test 10".to_string());
        assert_eq!(result, Ok("// $define test 10".to_string()));
    }

    #[test]
    fn test_process_invalid_define_directive() {
        let mut context = Context::new();
        let result = context.process("$define test".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_process_eval_line() {
        let mut context = Context::new();
        context
            .define("test".to_string(), Value::Int(10))
            .expect("failed to define value");
        let result = context.process(
            "this is $(test + 10) a $((1 + 2) * 3) test $(test + test) $test $test f$test"
                .to_string(),
        );
        assert_eq!(result, Ok("this is 20 a 9 test 20 10 10 f10".to_string()));
    }

    #[test]
    fn test_process_eval_line_with_invalid_expression() {
        let mut context = Context::new();
        let result = context.process("$(test + 10)".to_string());
        assert!(result.is_err());
    }
}
