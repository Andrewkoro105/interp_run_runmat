use std::collections::HashMap;

use execute::execute;
use interp_run::{
    from_sys::script::{InspectorError, MatParser, Script, ScriptInspector},
    run_script::RunScript,
    values::Values,
};
use runmat_async::RuntimeError;
use runmat_hir::SemanticError;
use runmat_ignition::CompileError;
use runmat_parser::SyntaxError;
use serde_json::Value as JsonValue;
use sugar::hashset;
pub mod execute;
pub mod runmat_json;

pub struct RunmatBuilder {}

#[derive(Debug)]
pub enum RunmatError {
    SyntaxError(SyntaxError),
    SemanticError(SemanticError),
    CompileError(CompileError),
    RuntimeError(RuntimeError),
    TokioRuntime(std::io::Error),
    InspectorError(InspectorError),
    SerdeJson(serde_json::Error),
}

pub struct Runmat {
    pub script_inspector: ScriptInspector,
}

impl RunScript for Runmat {
    type Script = Script;
    type Error = RunmatError;

    fn run_script(
        &self,
        script: Self::Script,
        data: HashMap<String, JsonValue>,
    ) -> Result<Values, Self::Error> {
        let result_name = format!("{}_result", env!("CARGO_PKG_NAME"));

        let mut base_script = self
            .script_inspector
            .to_string(script)
            .map_err(RunmatError::InspectorError)?
            .trim()
            .to_string();
        if !base_script.is_empty() {
            let pos = base_script.rfind('\n').map_or(0, |p| p + 1);
            base_script.insert_str(pos, &format!("{} = ", result_name));
        }
        execute(base_script, data).map(|values| Values::new(values, result_name))
    }
}

impl Runmat {
    pub fn new() -> Self {
        Self {
            script_inspector: ScriptInspector {
                restricted_functions: hashset! {},
                parser: Box::new(MatParser {}) as _,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::Number;

    use interp_run::run_script::RunScript;
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn simle() {
        let mut data = HashMap::new();
        data.insert(
            "test_value".to_string(),
            JsonValue::Number(Number::from_u128(42).unwrap()),
        );

        let script_result = Runmat::new()
            .run_script("input_data.test_value ^ 2".to_string().into(), data)
            .unwrap()
            .get_result()
            .as_u64()
            .unwrap();

        assert_eq!(script_result, 1764);
    }
}
