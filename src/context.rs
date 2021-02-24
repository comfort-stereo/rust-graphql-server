use tide::Request;

use crate::executor::Executor;
use crate::state::State;

pub struct Context {
    executor: Executor,
}

impl Context {
    pub async fn new(request: Request<State>) -> Self {
        let state = request.state();
        let executor = Executor::new(state.clone());
        Context { executor }
    }

    pub fn executor(&self) -> &Executor {
        &self.executor
    }
}
