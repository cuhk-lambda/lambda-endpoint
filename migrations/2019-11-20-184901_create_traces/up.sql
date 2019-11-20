-- Your SQL goes here

CREATE TABLE traces (
                       id SERIAL PRIMARY KEY,
                       process VARCHAR NOT NULL,
                       function_list TEXT[],
                       environment TEXT[],
                       values TEXT[],
                    options TEXT[],
                    trace_type VARCHAR NOT NULL

)