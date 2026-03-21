mod routes;
mod handlers;
mod templates;
mod utils;

pub use routes::web_routes;
pub use utils::{mask_api_key, calculate_context};