use gotham::router::Router;
use gotham::router::builder::*;
use super::handler::*;
use super::global_state::*;
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single_middleware;
use gotham::pipeline::single::single_pipeline;

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
    })
}