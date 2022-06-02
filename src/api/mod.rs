mod error;
pub mod links;
mod oauth2;
mod user;

use crate::app::User;

pub use oauth2::{authenticate, login, logout};

pub use user::UserId;
