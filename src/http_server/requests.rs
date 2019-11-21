use serde::*;
#[derive(Debug, Serialize, Deserialize)]
struct RemoveTrace {
    remove_type: String,
    remove_id: i32
}

#[derive(Debug, Serialize, Deserialize)]
struct StartTrace {
    trace_type: String,
    trace_id: i32,
    lasting: i32
}

