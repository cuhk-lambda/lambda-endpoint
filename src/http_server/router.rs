use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single::single_pipeline;
use gotham::pipeline::single_middleware;
use gotham::router::builder::*;
use gotham::router::Router;

use super::global_state::*;
use super::handler::*;

pub fn router() -> Router {
    // create the counter to share across handlers
    let state = GlobalState::new();

    // create our state middleware to share the counter
    let middleware = StateMiddleware::new(state);


    // create a middleware pipeline from our middleware
    let pipeline = single_middleware(middleware);

    // construct a basic chain from our pipeline
    let (chain, pipelines) = single_pipeline(pipeline);

    // build a router with the chain & pipeline
    build_router(chain, pipelines, |route| {
        route.get("/heartbeat").to(heartbeat);
        route.get("/state").to(endpoint_state);
        route.post("/start_trace").to(start_trace);
        route.get("/list").to(trace_list);
        route.get("/running_list").to(running_traces);
    })
}