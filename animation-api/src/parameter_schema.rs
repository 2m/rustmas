use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct EnumOption {
    pub name: String,
    pub description: Option<String>,
    pub value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParameterValue {
    Number { min: f64, max: f64, step: f64 },
    Color,
    Enum { values: Vec<EnumOption> },
    Speed,
    Percentage,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Parameter {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(flatten)]
    pub value: ParameterValue,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct ParametersSchema {
    pub parameters: Vec<Parameter>,
}

pub trait GetParametersSchema {
    fn schema() -> ParametersSchema;
}

pub fn get_schema<T: GetParametersSchema>() -> ParametersSchema {
    T::schema()
}

pub trait GetEnumOptions {
    fn enum_options() -> Vec<EnumOption>;
}

pub fn get_enum_options<T: GetEnumOptions>() -> Vec<EnumOption> {
    T::enum_options()
}
