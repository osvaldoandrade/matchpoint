use super::App;
use crate::{error::Error, matching::Service};
impl App {
    /// Open isolated local matching state without changing historical remote jobs.
    pub fn matching(&self) -> Result<Service, Error> {
        Service::open(
            self.paths.root.clone(),
            std::sync::Arc::clone(&self.secrets),
        )
    }
}
