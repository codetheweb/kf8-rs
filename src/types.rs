#[cfg(test)]
use proptest::{arbitrary::any, prop_compose};
#[cfg(test)]
use proptest_derive::Arbitrary;

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct SectionHeader {
    pub offset: u32,
    pub flags: u8,
    #[cfg_attr(test, proptest(strategy = "0..(u32::from(ux::u24::MAX))"))]
    pub val: u32,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum MobiHeaderIdent {
    BookMobi,
    TextRead,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum CompressionType {
    None,
    PalmDoc,
    HuffCdic,
}

#[derive(Debug, PartialEq)]
pub struct MobiHeader {
    pub name: String,
    pub num_sections: u16,
    pub ident: MobiHeaderIdent,
    pub section_headers: Vec<SectionHeader>,
}

#[cfg(test)]
prop_compose! {
    // name is limited to valid ASCII characters rather than UTF-8 because otherwise the codepoint splitting gets weird
    pub fn mobi_header()(name in "[\x01-\x7F]{0,31}", ident in any::<MobiHeaderIdent>(), section_headers in any::<Vec<SectionHeader>>()) -> MobiHeader {
        MobiHeader {
            name,
            num_sections: section_headers.len() as u16,
            ident,
            section_headers
        }
    }
}
