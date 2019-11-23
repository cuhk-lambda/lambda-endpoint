use serde::*;

use crate::db::schema::trace::traces;

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveTrace {
    pub remove_type: String,
    pub remove_id: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartTrace {
    pub trace_type: String,
    pub trace_id: i32,
    pub lasting: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KillTrace {
    pub file_path: String,
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[table_name = "traces"]
pub struct PutTrace {
    pub process: String,
    pub function_list: Vec<String>,
    pub environment: Vec<String>,
    pub values: Vec<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteTrace {
    pub trace_id: i32,
}
