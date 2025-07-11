use serde_repr::{Deserialize_repr, Serialize_repr};

#[repr(u8)]
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize_repr, Deserialize_repr,
)]
pub enum ImapSecurity {
    NoEncryption = 1,
    UseSSL = 2,
    UseSTARTTLS = 3,
}
