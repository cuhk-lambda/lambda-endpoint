use serde::*;

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

