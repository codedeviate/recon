//! EU VAT check digits (per-country dispatch).

pub mod se;
pub mod dk;
pub mod fi;
pub mod de;
pub mod fr;
pub mod pl;
pub mod pt;
pub mod si;
pub mod sk;
pub mod ee;
pub mod hu;
pub mod at;
pub mod be;

pub use se::{verify_se_vat, create_se_vat};
pub use dk::{verify_dk_vat, create_dk_vat};
pub use fi::{verify_fi_vat, create_fi_vat};
pub use de::{verify_de_vat, create_de_vat};
pub use fr::{verify_fr_vat, create_fr_vat};
pub use pl::{verify_pl_vat, create_pl_vat};
pub use pt::{verify_pt_vat, create_pt_vat};
pub use si::{verify_si_vat, create_si_vat};
pub use sk::{verify_sk_vat, create_sk_vat};
pub use ee::{verify_ee_vat, create_ee_vat};
pub use hu::{verify_hu_vat, create_hu_vat};
pub use at::{verify_at_vat, create_at_vat};
pub use be::{verify_be_vat, create_be_vat};
