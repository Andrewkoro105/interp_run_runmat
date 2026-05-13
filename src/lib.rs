use std::collections::HashMap;

use execute::execute;
use interp_run::backend::{
    self, Backend, Values,
    from_sys::script::{InspectorError, MatParser, ScriptInspector},
};
use runmat_async::RuntimeError;
use runmat_hir::SemanticError;
use runmat_ignition::CompileError;
use runmat_parser::SyntaxError;
use serde_json::Value as JsonValue;
use sugar::hashset;
pub mod execute;
pub mod runmat_json;

#[derive(Debug)]
pub enum RunmatError {
    SyntaxError(SyntaxError),
    SemanticError(SemanticError),
    CompileError(CompileError),
    RuntimeError(RuntimeError),
    TokioRuntime(std::io::Error),
    InspectorError(InspectorError),
}

pub struct Runmat {
    pub script_inspector: ScriptInspector,
}

impl Backend for Runmat {
    type Script = backend::from_sys::script::Script;
    type Error = RunmatError;

    fn run_scripts(
        &self,
        script: Vec<Self::Script>,
        data: HashMap<String, JsonValue>,
    ) -> Result<Vec<Values>, Self::Error> {
        todo!()
    }

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

    fn run(
        &mut self,
        script: Self::Script,
        data: HashMap<String, JsonValue>,
    ) -> Result<Values, Self::Error> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        todo!()
    }

    fn get_data(&self) -> Result<Values, Self::Error> {
        todo!()
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

    use crate::backend::Backend;
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
