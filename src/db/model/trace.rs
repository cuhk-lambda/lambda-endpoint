use diesel::*;

#[derive(Queryable)]
pub struct Trace {
    pub id: i32,
    pub process: String,
    pub function_list: Vec<String>,
    pub environment: Vec<String>,
    pub values: Vec<String>,
    pub options: Vec<String>,
    pub trace_type: String
}