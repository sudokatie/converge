//! Typed identifiers for game pack content.

use serde::{Deserialize, Serialize};

macro_rules! define_id {
    ($name:ident, $display_prefix:literal) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            Default,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        pub struct $name(u64);

        impl $name {
            #[must_use]
            pub const fn from_raw(value: u64) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn new(seed: u32, sequence: u32) -> Self {
                Self(((seed as u64) << 32) | (sequence as u64))
            }

            #[must_use]
            pub const fn raw(self) -> u64 {
                self.0
            }

            #[must_use]
            pub const fn seed(self) -> u32 {
                (self.0 >> 32) as u32
            }

            #[must_use]
            #[expect(
                clippy::cast_possible_truncation,
                reason = "intentional extraction of lower 32 bits"
            )]
            pub const fn sequence(self) -> u32 {
                self.0 as u32
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{}:{:08x}:{:08x}",
                    $display_prefix,
                    self.seed(),
                    self.sequence()
                )
            }
        }
    };
}

define_id!(PackId, "pack");
define_id!(BlockId, "block");
define_id!(SystemId, "system");
define_id!(HazardId, "hazard");
define_id!(ShaderId, "shader");
define_id!(RuleProfileId, "rule");

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_id {
        ($name:ident, $id_type:ty, $prefix:literal) => {
            mod $name {
                use super::*;

                #[test]
                fn new_and_extract() {
                    let id = <$id_type>::new(0xDEAD, 0xBEEF);
                    assert_eq!(id.seed(), 0xDEAD);
                    assert_eq!(id.sequence(), 0xBEEF);
                }

                #[test]
                fn from_raw_roundtrip() {
                    let raw = 0x1234_5678_9ABC_DEF0;
                    let id = <$id_type>::from_raw(raw);
                    assert_eq!(id.raw(), raw);
                }

                #[test]
                fn display() {
                    let id = <$id_type>::new(0x0000_1234, 0x0000_5678);
                    let s = format!("{id}");
                    assert_eq!(s, concat!($prefix, ":00001234:00005678"));
                }

                #[test]
                fn ordering() {
                    let id1 = <$id_type>::new(1, 0);
                    let id2 = <$id_type>::new(1, 1);
                    let id3 = <$id_type>::new(2, 0);

                    assert!(id1 < id2);
                    assert!(id2 < id3);
                }

                #[test]
                fn serde_roundtrip() {
                    let id = <$id_type>::new(42, 100);
                    let serialized = serde_json::to_string(&id).unwrap();
                    let deserialized: $id_type = serde_json::from_str(&serialized).unwrap();
                    assert_eq!(id, deserialized);
                }

                #[test]
                fn bincode_roundtrip() {
                    let id = <$id_type>::new(42, 100);
                    let serialized = bincode::serialize(&id).unwrap();
                    let deserialized: $id_type = bincode::deserialize(&serialized).unwrap();
                    assert_eq!(id, deserialized);
                }
            }
        };
    }

    test_id!(pack_id, PackId, "pack");
    test_id!(block_id, BlockId, "block");
    test_id!(system_id, SystemId, "system");
    test_id!(hazard_id, HazardId, "hazard");
    test_id!(shader_id, ShaderId, "shader");
    test_id!(rule_profile_id, RuleProfileId, "rule");
}
