use runmat_builtins::IntValue as RunmatIntValue;
use runmat_builtins::StructValue;
use runmat_builtins::Value as RunmatValue;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use serde_json::{Number, Value as JsonValue};
use std::collections::HashMap;

pub fn runmat2json_value(value: RunmatValue) -> JsonValue {
    match value {
        RunmatValue::Int(value) => match value {
            RunmatIntValue::I8(value) => json!(value),
            RunmatIntValue::I16(value) => json!(value),
            RunmatIntValue::I32(value) => json!(value),
            RunmatIntValue::I64(value) => json!(value),
            RunmatIntValue::U8(value) => json!(value),
            RunmatIntValue::U16(value) => json!(value),
            RunmatIntValue::U32(value) => json!(value),
            RunmatIntValue::U64(value) => json!(value),
        },
        RunmatValue::Num(value) => JsonValue::Number(Number::from_i128(value as _).unwrap()),
        RunmatValue::Complex(re, im) => json!({ "re": re, "im": im }),
        RunmatValue::Bool(value) => json!(value),
        RunmatValue::LogicalArray(value) => to_value(
            &value
                .data
                .into_iter()
                .map(|value| if value != 0 { true } else { false })
                .collect::<Vec<_>>(),
            &value.shape,
        ),
        RunmatValue::String(value) => json!(value),
        RunmatValue::StringArray(value) => to_value(&value.data, &value.shape),
        RunmatValue::CharArray(value) => to_value(&value.data, &[value.rows, value.cols]),
        RunmatValue::Tensor(value) => to_value(&value.data, &value.shape),
        RunmatValue::ComplexTensor(value) => to_value(&value.data, &value.shape),
        RunmatValue::Cell(value) => to_value(
            &value
                .data
                .into_iter()
                .map(|value| runmat2json_value((*value).clone()))
                .collect::<Vec<_>>(),
            &value.shape,
        ),
        RunmatValue::Struct(value) => JsonValue::Object(
            value
                .fields
                .into_iter()
                .map(|(name, value)| (name, runmat2json_value(value)))
                .collect(),
        ),
        RunmatValue::GpuTensor(value) => json!(value),
        RunmatValue::Object(value) => {
            let properties = value
                .properties
                .into_iter()
                .map(|(name, value)| (name, runmat2json_value(value)))
                .collect::<HashMap<_, _>>();
            json!({"class_name": value.class_name, "properties": properties})
        }
        RunmatValue::HandleObject(value) => {
            json!({
                "class_name": value.class_name,
                "target": runmat2json_value((*value.target).clone()),
                "valid": value.valid,
            })
        }
        RunmatValue::Listener(value) => {
            json!({
                "id": value.id,
                "target": runmat2json_value((*value.target).clone()),
                "event_name": value.event_name,
                "callback": runmat2json_value((*value.callback).clone()),
                "enabled": value.enabled,
                "valid": value.valid,
            })
        }
        RunmatValue::OutputList(value) => {
            JsonValue::Array(value.into_iter().map(runmat2json_value).collect::<Vec<_>>())
        }
        RunmatValue::FunctionHandle(value) => json!(value),
        RunmatValue::Closure(value) => json!({
            "function_name": value.function_name,
            "captures": value
                .captures
                .into_iter()
                .map(runmat2json_value)
                .collect::<Vec<_>>(),
        }),
        RunmatValue::ClassRef(value) => json!(value),
        RunmatValue::MException(value) => json!({
            "identifier": value.identifier,
            "message": value.message,
            "stack": value.stack,
        }),
    }
}

pub fn json2runmat_value(value: JsonValue) -> RunmatValue {
    match value {
        JsonValue::Null => RunmatValue::OutputList(vec![]),
        JsonValue::Bool(value) => RunmatValue::Bool(value),
        JsonValue::Number(value) => {
            if value.is_f64() {
                RunmatValue::Num(value.as_f64().unwrap())
            } else if value.is_i64() {
                RunmatValue::Int(RunmatIntValue::I64(value.as_i64().unwrap()))
            } else {
                RunmatValue::Int(RunmatIntValue::U64(value.as_u64().unwrap()))
            }
        }
        JsonValue::String(value) => RunmatValue::String(value),
        JsonValue::Array(value) => {
            RunmatValue::OutputList(value.into_iter().map(json2runmat_value).collect())
        }
        JsonValue::Object(value) => RunmatValue::Struct(StructValue {
            fields: value
                .into_iter()
                .map(|(name, value)| (name, json2runmat_value(value)))
                .collect(),
        }),
    }
}

pub fn to_value<T: Serialize>(data: &[T], shape: &[usize]) -> JsonValue {
    let total: usize = shape.iter().product();
    assert_eq!(total, data.len(), "Data length does not match shape");
    build_json_value(data, shape)
}

pub fn from_value<T: DeserializeOwned>(value: &JsonValue) -> Result<(Vec<T>, Vec<usize>), String> {
    let shape = extract_shape(value)?;
    let data = flatten_value::<T>(value)?;
    let total: usize = shape.iter().product();
    if total != data.len() {
        return Err("Data length does not match shape".to_string());
    }
    Ok((data, shape))
}

fn build_json_value<T: Serialize>(data: &[T], shape: &[usize]) -> JsonValue {
    if shape.is_empty() {
        return serde_json::to_value(&data[0]).expect("Serialization failed");
    }
    let dim = shape[0];
    let rest = &shape[1..];
    let chunk_size: usize = rest.iter().product();
    let mut arr = Vec::with_capacity(dim);
    for i in 0..dim {
        let start = i * chunk_size;
        let end = start + chunk_size;
        arr.push(build_json_value(&data[start..end], rest));
    }
    JsonValue::Array(arr)
}

fn extract_shape(value: &JsonValue) -> Result<Vec<usize>, String> {
    match value {
        JsonValue::Array(arr) => {
            if arr.is_empty() {
                return Err("Empty array is not allowed".to_string());
            }
            let first_shape = extract_shape(&arr[0])?;
            let mut shape = vec![arr.len()];
            shape.extend(first_shape.clone());
            for item in arr.iter().skip(1) {
                if extract_shape(item)? != first_shape {
                    return Err("Inconsistent nested array shapes".to_string());
                }
            }
            Ok(shape)
        }
        _ => Ok(vec![]),
    }
}

fn flatten_value<T: DeserializeOwned>(value: &JsonValue) -> Result<Vec<T>, String> {
    match value {
        JsonValue::Array(arr) => {
            let mut result = Vec::new();
            for item in arr {
                result.extend(flatten_value::<T>(item)?);
            }
            Ok(result)
        }
        _ => {
            let t = serde_json::from_value(value.clone())
                .map_err(|e| format!("Failed to deserialize to T: {}", e))?;
            Ok(vec![t])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simpl() {
        let start_ass = json!({
            "list": [3, 2, 4],
            "bub": false,
            "sbib": "sbib diss",
        });

        println!("{:?}", json2runmat_value(start_ass.clone()));

        assert_eq!(start_ass.clone(), runmat2json_value(json2runmat_value(start_ass)));
    }
}
