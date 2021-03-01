use tide::Request;

use crate::executor::Executor;
use crate::state::State;

/// Shared data for a single GraphQL request. This context is accessible throughout the schema.
pub struct Context {
    executor: Executor,
}

impl Context {
    // Create a new context for the specified request.
    pub async fn new(request: Request<State>) -> Self {
        // Create a new executor for the request, passing it the global server state.
        Context {
            executor: Executor::new(request.state().clone()),
        }
    }

    /// Get the executor for the current request.
    pub fn executor(&self) -> &Executor {
        &self.executor
    }
}
