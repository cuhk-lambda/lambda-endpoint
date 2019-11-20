table! {
    traces (id) {
        id -> Integer,
        process -> Text,
        function_list -> Array<Text>,
        environment -> Array<Text>,
        values -> Array<Text>,
        options -> Array<Text>,
        trace_type -> Text,
    }
}