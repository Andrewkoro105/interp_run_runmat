use crate::RunmatError;
use crate::runmat_json::json2runmat_value;
use crate::runmat_json::runmat2json_value;
use runmat_builtins::Value as RunmatValue;
use runmat_hir::LoweringContext;
use runmat_hir::lower;
use runmat_ignition::compile;
use runmat_ignition::{Bytecode, InterpreterOutcome, interpret_with_vars};
use runmat_parser::parse;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use sugar::hashmap;

pub fn execute(
    script: String,
    data: HashMap<String, JsonValue>,
) -> Result<HashMap<String, JsonValue>, RunmatError> {
    let input_data = json2runmat_value(JsonValue::Object(data.into_iter().collect()));

    let ast = parse(&script).map_err(RunmatError::SyntaxError)?;
    let low = lower(
        &ast,
        &LoweringContext {
            variables: &hashmap! {
                "input_data".into() => 0,
            },
            functions: &HashMap::new(),
        },
    )
    .map_err(RunmatError::SemanticError)?;
    let bc = compile(&low.hir, &HashMap::new()).map_err(RunmatError::CompileError)?;
    if bc.var_count != 0 {
        let values = interpret(&bc, input_data)?;

        Ok(low
            .var_names
            .into_iter()
            .map(|(id, name)| (name, runmat2json_value(values[id.0].clone())))
            .collect())
    } else {
        Ok(HashMap::new())
    }
}

fn interpret(bytecode: &Bytecode, value: RunmatValue) -> Result<Vec<RunmatValue>, RunmatError> {
    let mut vars = [
        vec![value],
        vec![RunmatValue::Num(0.0); bytecode.var_count - 1],
    ]
    .concat();

    match tokio::runtime::Runtime::new()
        .map_err(RunmatError::TokioRuntime)?
        .block_on(interpret_with_vars(bytecode, &mut vars, Some("<main>")))
        .map_err(RunmatError::RuntimeError)?
    {
        InterpreterOutcome::Completed(values) => Ok(values),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sugar::hashmap;

    use super::*;

    #[test]
    fn simpl() {
        assert_eq!(
            1764,
            execute(
                "result = input_data.a ^ 2;".to_string(),
                hashmap! {
                    "a".to_string() => json!(42),
                }
            )
            .unwrap()
            .get(&"result".to_string())
            .unwrap()
            .as_u64()
            .unwrap()
        );
    }

    #[test]
    fn empty() {
        execute(
            "".to_string(),
            hashmap! {
                "a".to_string() => json!(42),
            },
        )
        .unwrap();
    }
}
