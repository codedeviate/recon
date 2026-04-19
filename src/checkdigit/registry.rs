//! Static registry of all check-digit specs. Resolve by canonical name or alias.

use super::brand::Brand;
use super::{
    aba, base58check, bech32_mod, brand, country_id, eip55, luhn, mod10_ean, mod11, mod31, mod97,
    mrz, vat, vin, Spec, Verdict,
};
use anyhow::Result;

static SPEC_LUHN: Spec = Spec {
    canonical: "luhn",
    aliases: &[],
    description: "Bare Luhn mod-10 check on any digit string",
    verify_fn: luhn::verify_bare,
    create_fn: luhn::create_bare,
};

static SPEC_CREDITCARD: Spec = Spec {
    canonical: "creditcard",
    aliases: &[],
    description: "Credit card (auto-detects brand from IIN)",
    verify_fn: brand::verify_creditcard,
    create_fn: brand::create_creditcard,
};

// Trampoline functions for brand-specific specs (closures don't coerce to fn pointers).
fn verify_visa(i: &str) -> Verdict { brand::verify_brand(i, Brand::Visa) }
fn create_visa(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Visa, r) }

fn verify_mastercard(i: &str) -> Verdict { brand::verify_brand(i, Brand::Mastercard) }
fn create_mastercard(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Mastercard, r) }

fn verify_amex(i: &str) -> Verdict { brand::verify_brand(i, Brand::Amex) }
fn create_amex(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Amex, r) }

fn verify_discover(i: &str) -> Verdict { brand::verify_brand(i, Brand::Discover) }
fn create_discover(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Discover, r) }

fn verify_jcb(i: &str) -> Verdict { brand::verify_brand(i, Brand::Jcb) }
fn create_jcb(i: &str, r: bool) -> Result<String> { brand::create_brand(i, Brand::Jcb, r) }

static SPEC_VISA: Spec = Spec {
    canonical: "visa",
    aliases: &[],
    description: "Visa credit card (Luhn, 13/16/19 digits, IIN 4)",
    verify_fn: verify_visa,
    create_fn: create_visa,
};

static SPEC_MASTERCARD: Spec = Spec {
    canonical: "mastercard",
    aliases: &["mc"],
    description: "MasterCard (Luhn, 16 digits, IIN 51-55 or 2221-2720)",
    verify_fn: verify_mastercard,
    create_fn: create_mastercard,
};

static SPEC_AMEX: Spec = Spec {
    canonical: "amex",
    aliases: &[],
    description: "American Express (Luhn, 15 digits, IIN 34 or 37)",
    verify_fn: verify_amex,
    create_fn: create_amex,
};

static SPEC_DISCOVER: Spec = Spec {
    canonical: "discover",
    aliases: &[],
    description: "Discover (Luhn, 16 digits, IIN 6011/65/644-649)",
    verify_fn: verify_discover,
    create_fn: create_discover,
};

static SPEC_JCB: Spec = Spec {
    canonical: "jcb",
    aliases: &[],
    description: "JCB (Luhn, 16 digits, IIN 3528-3589)",
    verify_fn: verify_jcb,
    create_fn: create_jcb,
};

static SPEC_IMEI: Spec = Spec {
    canonical: "imei",
    aliases: &[],
    description: "Mobile IMEI (Luhn, 15 digits)",
    verify_fn: brand::verify_imei,
    create_fn: brand::create_imei,
};

static SPEC_ISIN: Spec = Spec {
    canonical: "isin",
    aliases: &[],
    description: "International Securities ID (12 alnum, Luhn on letter-transliterated form)",
    verify_fn: luhn::verify_isin,
    create_fn: luhn::create_isin,
};

static SPEC_NPI: Spec = Spec {
    canonical: "npi",
    aliases: &[],
    description: "US National Provider Identifier (10 digits, Luhn with 80840 prefix)",
    verify_fn: luhn::verify_npi,
    create_fn: luhn::create_npi,
};

static SPEC_PERSONNUMMER: Spec = Spec {
    canonical: "personnummer",
    aliases: &["se-id"],
    description: "Swedish personnummer (10 or 12 digits, Luhn on last 10; + separator for ≥100 yrs)",
    verify_fn: country_id::verify_personnummer,
    create_fn: country_id::create_personnummer,
};

static SPEC_SIN: Spec = Spec {
    canonical: "sin",
    aliases: &["ca-sin"],
    description: "Canadian Social Insurance Number (9 digits, Luhn)",
    verify_fn: country_id::verify_sin,
    create_fn: country_id::create_sin,
};

static SPEC_SA_ID: Spec = Spec {
    canonical: "sa-id",
    aliases: &[],
    description: "South African ID (13 digits, Luhn)",
    verify_fn: country_id::verify_sa_id,
    create_fn: country_id::create_sa_id,
};

static SPEC_EAN13: Spec = Spec {
    canonical: "ean13",
    aliases: &["ean"],
    description: "European Article Number EAN-13 (13 digits)",
    verify_fn: mod10_ean::verify_ean13,
    create_fn: mod10_ean::create_ean13,
};

static SPEC_EAN8: Spec = Spec {
    canonical: "ean8",
    aliases: &[],
    description: "Short EAN (8 digits)",
    verify_fn: mod10_ean::verify_ean8,
    create_fn: mod10_ean::create_ean8,
};

static SPEC_UPCA: Spec = Spec {
    canonical: "upca",
    aliases: &["upc"],
    description: "Universal Product Code A (12 digits)",
    verify_fn: mod10_ean::verify_upca,
    create_fn: mod10_ean::create_upca,
};

static SPEC_UPCE: Spec = Spec {
    canonical: "upce",
    aliases: &[],
    description: "Short UPC (8 digits)",
    verify_fn: mod10_ean::verify_upce,
    create_fn: mod10_ean::create_upce,
};

static SPEC_ISBN13: Spec = Spec {
    canonical: "isbn13",
    aliases: &[],
    description: "International Standard Book Number, 13-digit (simple 3-1-2-6-1 hyphenation)",
    verify_fn: mod10_ean::verify_isbn13,
    create_fn: mod10_ean::create_isbn13,
};

static SPEC_GTIN8: Spec = Spec {
    canonical: "gtin8",
    aliases: &[],
    description: "GTIN-8 (Global Trade Item Number, 8 digits — same as EAN-8)",
    verify_fn: mod10_ean::verify_gtin8,
    create_fn: mod10_ean::create_gtin8,
};

static SPEC_GTIN12: Spec = Spec {
    canonical: "gtin12",
    aliases: &[],
    description: "GTIN-12 (12 digits — same as UPC-A)",
    verify_fn: mod10_ean::verify_gtin12,
    create_fn: mod10_ean::create_gtin12,
};

static SPEC_GTIN13: Spec = Spec {
    canonical: "gtin13",
    aliases: &[],
    description: "GTIN-13 (13 digits — same as EAN-13)",
    verify_fn: mod10_ean::verify_gtin13,
    create_fn: mod10_ean::create_gtin13,
};

static SPEC_GTIN14: Spec = Spec {
    canonical: "gtin14",
    aliases: &["gtin"],
    description: "GTIN-14 (14 digits, logistic units)",
    verify_fn: mod10_ean::verify_gtin14,
    create_fn: mod10_ean::create_gtin14,
};

static SPEC_SSCC: Spec = Spec {
    canonical: "sscc",
    aliases: &[],
    description: "Serial Shipping Container Code (18 digits)",
    verify_fn: mod10_ean::verify_sscc,
    create_fn: mod10_ean::create_sscc,
};

static SPEC_ISBN10: Spec = Spec {
    canonical: "isbn10",
    aliases: &[],
    description: "International Standard Book Number, 10-digit (mod 11, may end in 'X')",
    verify_fn: mod11::verify_isbn10,
    create_fn: mod11::create_isbn10,
};

static SPEC_CPR: Spec = Spec {
    canonical: "cpr",
    aliases: &["dk-id"],
    description: "Danish CPR-nummer (10 digits, mod-11; note: post-2007 may not satisfy check)",
    verify_fn: mod11::verify_cpr,
    create_fn: mod11::create_cpr,
};

static SPEC_BSN: Spec = Spec {
    canonical: "bsn",
    aliases: &["nl-id"],
    description: "Dutch Burgerservicenummer (8 or 9 digits, elfproef mod-11)",
    verify_fn: mod11::verify_bsn,
    create_fn: mod11::create_bsn,
};

static SPEC_FODSELSNUMMER: Spec = Spec {
    canonical: "fodselsnummer",
    aliases: &["no-id"],
    description: "Norwegian fødselsnummer (11 digits, two mod-11 check digits)",
    verify_fn: mod11::verify_fodselsnummer,
    create_fn: mod11::create_fodselsnummer,
};

static SPEC_HENKILOTUNNUS: Spec = Spec {
    canonical: "henkilotunnus",
    aliases: &["fi-id"],
    description: "Finnish henkilötunnus (11 chars, mod-31 with lookup; century markers +, -, Y-U, A-F)",
    verify_fn: mod31::verify_henkilotunnus,
    create_fn: mod31::create_henkilotunnus,
};

static SPEC_IBAN: Spec = Spec {
    canonical: "iban",
    aliases: &[],
    description: "International Bank Account Number (15-34 chars, mod 97, 80+ country formats)",
    verify_fn: mod97::verify_iban,
    create_fn: mod97::create_iban,
};

static SPEC_VIN: Spec = Spec {
    canonical: "vin",
    aliases: &[],
    description: "Vehicle Identification Number (17 alnum, transliterate + weighted mod 11; I/O/Q disallowed)",
    verify_fn: vin::verify_vin,
    create_fn: vin::create_vin,
};

static SPEC_MRZ: Spec = Spec {
    canonical: "mrz",
    aliases: &[],
    description: "Passport / ID card MRZ (ICAO Doc 9303, TD1/TD2/TD3)",
    verify_fn: mrz::verify_mrz,
    create_fn: mrz::create_mrz,
};

static SPEC_ABA: Spec = Spec {
    canonical: "aba",
    aliases: &["us-routing"],
    description: "US ABA routing number (9 digits, weighted mod-10 with [3,7,1])",
    verify_fn: aba::verify_aba,
    create_fn: aba::create_aba,
};

static SPEC_BTC: Spec = Spec {
    canonical: "btc",
    aliases: &["bitcoin"],
    description: "Bitcoin address (base58check; P2PKH and P2SH)",
    verify_fn: base58check::verify_btc,
    create_fn: base58check::create_unsupported,
};

static SPEC_LTC: Spec = Spec {
    canonical: "ltc",
    aliases: &["litecoin"],
    description: "Litecoin address (base58check)",
    verify_fn: base58check::verify_ltc,
    create_fn: base58check::create_unsupported,
};

static SPEC_DOGE: Spec = Spec {
    canonical: "doge",
    aliases: &["dogecoin"],
    description: "Dogecoin address (base58check)",
    verify_fn: base58check::verify_doge,
    create_fn: base58check::create_unsupported,
};

static SPEC_ETH: Spec = Spec {
    canonical: "eth",
    aliases: &["ethereum", "eip55"],
    description: "Ethereum address (EIP-55 mixed-case checksum)",
    verify_fn: eip55::verify_eip55,
    create_fn: eip55::create_eip55,
};

static SPEC_BECH32: Spec = Spec {
    canonical: "bech32",
    aliases: &["segwit"],
    description: "Bech32 / SegWit address (BIP-173)",
    verify_fn: bech32_mod::verify_bech32,
    create_fn: bech32_mod::create_unsupported,
};

static SPEC_BG_VAT: Spec = Spec {
    canonical: "bg-vat",
    aliases: &["bgvat"],
    description: "Bulgarian VAT (auto-detect EGN/BULSTAT)",
    verify_fn: vat::verify_bg_vat,
    create_fn: vat::create_bg_vat,
};

static SPEC_BG_EGN: Spec = Spec {
    canonical: "bg-egn",
    aliases: &[],
    description: "Bulgarian EGN (personal, 10 digits with birth-date + century-encoded month)",
    verify_fn: vat::verify_bg_egn,
    create_fn: vat::create_bg_egn,
};

static SPEC_BG_BULSTAT: Spec = Spec {
    canonical: "bg-bulstat",
    aliases: &[],
    description: "Bulgarian BULSTAT (legal, 9 digits, weighted mod-11 with secondary fallback)",
    verify_fn: vat::verify_bg_bulstat,
    create_fn: vat::create_bg_bulstat,
};

static SPEC_SE_VAT: Spec = Spec {
    canonical: "se-vat",
    aliases: &["sevat"],
    description: "Swedish VAT (12 digits, org.nr + '01'; Luhn on org.nr)",
    verify_fn: vat::verify_se_vat,
    create_fn: vat::create_se_vat,
};

static SPEC_DK_VAT: Spec = Spec {
    canonical: "dk-vat",
    aliases: &["dkvat"],
    description: "Danish VAT / CVR (8 digits, weighted mod-11 on full number)",
    verify_fn: vat::verify_dk_vat,
    create_fn: vat::create_dk_vat,
};

static SPEC_FI_VAT: Spec = Spec {
    canonical: "fi-vat",
    aliases: &["fivat"],
    description: "Finnish VAT / Y-tunnus (8 digits, weighted mod-11)",
    verify_fn: vat::verify_fi_vat,
    create_fn: vat::create_fi_vat,
};

static SPEC_DE_VAT: Spec = Spec {
    canonical: "de-vat",
    aliases: &["devat"],
    description: "German VAT / USt-IdNr (9 digits, running-product mod-11)",
    verify_fn: vat::verify_de_vat,
    create_fn: vat::create_de_vat,
};

static SPEC_FR_VAT: Spec = Spec {
    canonical: "fr-vat",
    aliases: &["frvat"],
    description: "French VAT (2-key + 9-SIREN, key = mod-97 of SIREN)",
    verify_fn: vat::verify_fr_vat,
    create_fn: vat::create_fr_vat,
};

static SPEC_PL_VAT: Spec = Spec {
    canonical: "pl-vat",
    aliases: &["plvat"],
    description: "Polish VAT / NIP (10 digits, weighted mod-11)",
    verify_fn: vat::verify_pl_vat,
    create_fn: vat::create_pl_vat,
};

static SPEC_PT_VAT: Spec = Spec {
    canonical: "pt-vat",
    aliases: &["ptvat"],
    description: "Portuguese VAT / NIF (9 digits, weighted mod-11)",
    verify_fn: vat::verify_pt_vat,
    create_fn: vat::create_pt_vat,
};

static SPEC_SI_VAT: Spec = Spec {
    canonical: "si-vat",
    aliases: &["sivat"],
    description: "Slovenian VAT (8 digits, weighted mod-11)",
    verify_fn: vat::verify_si_vat,
    create_fn: vat::create_si_vat,
};

static SPEC_SK_VAT: Spec = Spec {
    canonical: "sk-vat",
    aliases: &["skvat"],
    description: "Slovak VAT (10 digits, full number divisible by 11)",
    verify_fn: vat::verify_sk_vat,
    create_fn: vat::create_sk_vat,
};

static SPEC_EE_VAT: Spec = Spec {
    canonical: "ee-vat",
    aliases: &["eevat"],
    description: "Estonian VAT (9 digits, weighted mod-10)",
    verify_fn: vat::verify_ee_vat,
    create_fn: vat::create_ee_vat,
};

static SPEC_HU_VAT: Spec = Spec {
    canonical: "hu-vat",
    aliases: &["huvat"],
    description: "Hungarian VAT (8 digits, weighted mod-10)",
    verify_fn: vat::verify_hu_vat,
    create_fn: vat::create_hu_vat,
};

static SPEC_AT_VAT: Spec = Spec {
    canonical: "at-vat",
    aliases: &["atvat"],
    description: "Austrian VAT / UID (8 digits after ATU prefix, Luhn-like mod-10)",
    verify_fn: vat::verify_at_vat,
    create_fn: vat::create_at_vat,
};

static SPEC_BE_VAT: Spec = Spec {
    canonical: "be-vat",
    aliases: &["bevat"],
    description: "Belgian VAT (10 digits, last 2 = 97 - body mod 97)",
    verify_fn: vat::verify_be_vat,
    create_fn: vat::create_be_vat,
};

static SPEC_LU_VAT: Spec = Spec {
    canonical: "lu-vat",
    aliases: &["luvat"],
    description: "Luxembourg VAT (8 digits, last 2 = first 6 mod 89)",
    verify_fn: vat::verify_lu_vat,
    create_fn: vat::create_lu_vat,
};

static SPEC_NL_VAT: Spec = Spec {
    canonical: "nl-vat",
    aliases: &["nlvat"],
    description: "Dutch VAT (9 digits + 'B' + 2-digit suffix, mod-11 elfproef on first 9; distinct from bsn)",
    verify_fn: vat::verify_nl_vat,
    create_fn: vat::create_nl_vat,
};

static SPEC_IT_VAT: Spec = Spec {
    canonical: "it-vat",
    aliases: &["itvat"],
    description: "Italian VAT / partita IVA (11 digits, Luhn)",
    verify_fn: vat::verify_it_vat,
    create_fn: vat::create_it_vat,
};

static SPEC_RO_VAT: Spec = Spec {
    canonical: "ro-vat",
    aliases: &["rovat"],
    description: "Romanian VAT / CIF (2-10 digits, weighted mod-11 with left-padding)",
    verify_fn: vat::verify_ro_vat,
    create_fn: vat::create_ro_vat,
};

static SPEC_EL_VAT: Spec = Spec {
    canonical: "el-vat",
    aliases: &["elvat", "gr-vat"],
    description: "Greek VAT (9 digits, weights = powers of 2, mod 11 mod 10)",
    verify_fn: vat::verify_el_vat,
    create_fn: vat::create_el_vat,
};

static SPEC_CY_VAT: Spec = Spec {
    canonical: "cy-vat",
    aliases: &["cyvat"],
    description: "Cypriot VAT (8 digits + letter, odd-position remap, mod 26)",
    verify_fn: vat::verify_cy_vat,
    create_fn: vat::create_cy_vat,
};

static SPEC_MT_VAT: Spec = Spec {
    canonical: "mt-vat",
    aliases: &["mtvat"],
    description: "Maltese VAT (8 digits, weights [3,4,6,7,8,9], last 2 = 37 - sum mod 37)",
    verify_fn: vat::verify_mt_vat,
    create_fn: vat::create_mt_vat,
};

static SPEC_HR_VAT: Spec = Spec {
    canonical: "hr-vat",
    aliases: &["hrvat"],
    description: "Croatian VAT / OIB (11 digits, ISO 7064 MOD 11,10)",
    verify_fn: vat::verify_hr_vat,
    create_fn: vat::create_hr_vat,
};

static SPEC_IE_VAT: Spec = Spec {
    canonical: "ie-vat",
    aliases: &["ievat"],
    description: "Irish VAT (7 digits + letter, 3 historical formats)",
    verify_fn: vat::verify_ie_vat,
    create_fn: vat::create_ie_vat,
};

static SPEC_LT_VAT: Spec = Spec {
    canonical: "lt-vat",
    aliases: &["ltvat"],
    description: "Lithuanian VAT (9 or 12 digits auto-detect, weighted mod-11 with fallback)",
    verify_fn: vat::verify_lt_vat,
    create_fn: vat::create_lt_vat,
};

static SPEC_ES_VAT: Spec = Spec {
    canonical: "es-vat",
    aliases: &["esvat"],
    description: "Spanish VAT (auto-detect NIF/NIE/CIF)",
    verify_fn: vat::verify_es_vat,
    create_fn: vat::create_es_vat,
};

static SPEC_ES_NIF: Spec = Spec {
    canonical: "es-nif",
    aliases: &[],
    description: "Spanish NIF (citizen): 8 digits + letter, mod-23 lookup",
    verify_fn: vat::verify_es_nif,
    create_fn: vat::create_es_nif,
};

static SPEC_ES_NIE: Spec = Spec {
    canonical: "es-nie",
    aliases: &[],
    description: "Spanish NIE (foreigner): X/Y/Z + 7 digits + letter",
    verify_fn: vat::verify_es_nie,
    create_fn: vat::create_es_nie,
};

static SPEC_ES_CIF: Spec = Spec {
    canonical: "es-cif",
    aliases: &[],
    description: "Spanish CIF (legal entity): letter + 7 digits + check (letter or digit)",
    verify_fn: vat::verify_es_cif,
    create_fn: vat::create_es_cif,
};

static SPEC_CZ_VAT: Spec = Spec {
    canonical: "cz-vat",
    aliases: &["czvat"],
    description: "Czech VAT (auto-detect IČO 8 digits or rodné číslo 9-10 digits)",
    verify_fn: vat::verify_cz_vat,
    create_fn: vat::create_cz_vat,
};

static SPEC_CZ_LEGAL: Spec = Spec {
    canonical: "cz-legal",
    aliases: &[],
    description: "Czech IČO (legal entity, 8 digits, weighted mod-11)",
    verify_fn: vat::verify_cz_legal,
    create_fn: vat::create_cz_legal,
};

static SPEC_CZ_PERSON: Spec = Spec {
    canonical: "cz-person",
    aliases: &[],
    description: "Czech rodné číslo (person, 9 or 10 digits; 10-digit divisible by 11)",
    verify_fn: vat::verify_cz_person,
    create_fn: vat::create_cz_person,
};

static SPEC_LV_VAT: Spec = Spec {
    canonical: "lv-vat",
    aliases: &["lvvat"],
    description: "Latvian VAT (auto-detect personal/business by first digit)",
    verify_fn: vat::verify_lv_vat,
    create_fn: vat::create_lv_vat,
};

static SPEC_LV_PERSONAL: Spec = Spec {
    canonical: "lv-personal",
    aliases: &[],
    description: "Latvian personal code (11 digits, first digit 0-3, weighted mod-11)",
    verify_fn: vat::verify_lv_personal,
    create_fn: vat::create_lv_personal,
};

static SPEC_LV_BUSINESS: Spec = Spec {
    canonical: "lv-business",
    aliases: &[],
    description: "Latvian business registration number (11 digits, first digit 4-9, weighted mod-11)",
    verify_fn: vat::verify_lv_business,
    create_fn: vat::create_lv_business,
};

pub static SPECS: &[&Spec] = &[
    &SPEC_LUHN,
    &SPEC_CREDITCARD,
    &SPEC_VISA,
    &SPEC_MASTERCARD,
    &SPEC_AMEX,
    &SPEC_DISCOVER,
    &SPEC_JCB,
    &SPEC_IMEI,
    &SPEC_ISIN,
    &SPEC_NPI,
    &SPEC_PERSONNUMMER,
    &SPEC_SIN,
    &SPEC_SA_ID,
    &SPEC_EAN13,
    &SPEC_EAN8,
    &SPEC_UPCA,
    &SPEC_UPCE,
    &SPEC_ISBN13,
    &SPEC_GTIN8,
    &SPEC_GTIN12,
    &SPEC_GTIN13,
    &SPEC_GTIN14,
    &SPEC_SSCC,
    &SPEC_ISBN10,
    &SPEC_CPR,
    &SPEC_BSN,
    &SPEC_FODSELSNUMMER,
    &SPEC_HENKILOTUNNUS,
    &SPEC_IBAN,
    &SPEC_VIN,
    &SPEC_MRZ,
    &SPEC_ABA,
    &SPEC_BTC,
    &SPEC_LTC,
    &SPEC_DOGE,
    &SPEC_ETH,
    &SPEC_BECH32,
    &SPEC_BG_VAT,
    &SPEC_BG_EGN,
    &SPEC_BG_BULSTAT,
    &SPEC_SE_VAT,
    &SPEC_DK_VAT,
    &SPEC_FI_VAT,
    &SPEC_DE_VAT,
    &SPEC_FR_VAT,
    &SPEC_PL_VAT,
    &SPEC_PT_VAT,
    &SPEC_SI_VAT,
    &SPEC_SK_VAT,
    &SPEC_EE_VAT,
    &SPEC_HU_VAT,
    &SPEC_AT_VAT,
    &SPEC_BE_VAT,
    &SPEC_LU_VAT,
    &SPEC_NL_VAT,
    &SPEC_IT_VAT,
    &SPEC_RO_VAT,
    &SPEC_EL_VAT,
    &SPEC_CY_VAT,
    &SPEC_MT_VAT,
    &SPEC_HR_VAT,
    &SPEC_IE_VAT,
    &SPEC_LT_VAT,
    &SPEC_ES_VAT,
    &SPEC_ES_NIF,
    &SPEC_ES_NIE,
    &SPEC_ES_CIF,
    &SPEC_CZ_VAT,
    &SPEC_CZ_LEGAL,
    &SPEC_CZ_PERSON,
    &SPEC_LV_VAT,
    &SPEC_LV_PERSONAL,
    &SPEC_LV_BUSINESS,
];

/// Resolve a CLI keyword (canonical or alias, case-insensitive).
pub fn resolve(name: &str) -> Option<&'static Spec> {
    for spec in SPECS {
        if spec.canonical.eq_ignore_ascii_case(name) {
            return Some(*spec);
        }
        for alias in spec.aliases {
            if alias.eq_ignore_ascii_case(name) {
                return Some(*spec);
            }
        }
    }
    None
}

/// Old-alias → new-alias hints, surfaced when a user tries a deprecated keyword.
const ALIAS_MIGRATIONS: &[(&str, &str)] = &[
    ("svat", "sevat"),
    ("dvat", "dkvat"),
];

/// Like `resolve(name)` but when no spec matches, checks the migration table
/// and returns a structured hint. Callers use this to produce a friendly
/// "did you mean 'newname'?" error.
pub fn resolve_with_suggestion(name: &str) -> Result<&'static Spec, Option<&'static str>> {
    if let Some(spec) = resolve(name) {
        return Ok(spec);
    }
    for (old, new) in ALIAS_MIGRATIONS {
        if old.eq_ignore_ascii_case(name) {
            return Err(Some(new));
        }
    }
    Err(None)
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn old_svat_returns_hint() {
        match resolve_with_suggestion("svat") {
            Err(Some(new)) => assert_eq!(new, "sevat"),
            _ => panic!("expected Err(Some(\"sevat\"))"),
        }
    }

    #[test]
    fn old_dvat_returns_hint() {
        match resolve_with_suggestion("dvat") {
            Err(Some(new)) => assert_eq!(new, "dkvat"),
            _ => panic!("expected Err(Some(\"dkvat\"))"),
        }
    }

    #[test]
    fn new_sevat_resolves() {
        assert!(resolve_with_suggestion("sevat").is_ok());
    }

    #[test]
    fn new_dkvat_resolves() {
        assert!(resolve_with_suggestion("dkvat").is_ok());
    }

    #[test]
    fn totally_unknown_returns_no_hint() {
        match resolve_with_suggestion("zzvat") {
            Err(None) => {}
            _ => panic!("expected Err(None)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_unknown_returns_none() {
        assert!(resolve("nonexistent").is_none());
    }

    #[test]
    fn resolve_luhn_returns_spec() {
        let spec = resolve("luhn").expect("luhn should resolve");
        assert_eq!(spec.canonical, "luhn");
    }

    #[test]
    fn resolve_is_case_insensitive() {
        assert!(resolve("LUHN").is_some());
        assert!(resolve("Luhn").is_some());
    }

    #[test]
    fn resolve_creditcard_returns_spec() {
        let spec = resolve("creditcard").expect("creditcard should resolve");
        assert_eq!(spec.canonical, "creditcard");
    }

    #[test]
    fn resolve_mc_alias_returns_mastercard() {
        let spec = resolve("mc").expect("mc alias should resolve");
        assert_eq!(spec.canonical, "mastercard");
    }

    #[test]
    fn resolve_imei_returns_spec() {
        let spec = resolve("imei").expect("imei should resolve");
        assert_eq!(spec.canonical, "imei");
    }
}
