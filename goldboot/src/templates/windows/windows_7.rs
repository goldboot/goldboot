use serde::{Deserialize, Serialize};
use std::error::Error;
use validator::Validate;

#[derive(Clone, Serialize, Deserialize, Validate, Debug)]
pub struct Windows7Template {
	pub id: TemplateId,
}
