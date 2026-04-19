//! EU VAT check digits (per-country dispatch).

pub mod se;
pub mod dk;
pub mod fi;
pub mod de;
pub mod fr;

pub use se::{verify_se_vat, create_se_vat};
pub use dk::{verify_dk_vat, create_dk_vat};
pub use fi::{verify_fi_vat, create_fi_vat};
pub use de::{verify_de_vat, create_de_vat};
pub use fr::{verify_fr_vat, create_fr_vat};
