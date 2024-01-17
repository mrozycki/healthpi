use std::{fmt, str::FromStr};

use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct MacAddress([u8; 6]);

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl From<[u8; 6]> for MacAddress {
    fn from(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }
}

impl From<MacAddress> for [u8; 6] {
    fn from(mac: MacAddress) -> Self {
        mac.0
    }
}

impl Serialize for MacAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MacAddress {
    fn deserialize<D>(deserializer: D) -> Result<MacAddress, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MacVisitor;
        impl<'de> Visitor<'de> for MacVisitor {
            type Value = MacAddress;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a string containing MAC address")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                MacAddress::from_str(v).map_err(|e| serde::de::Error::custom(e))
            }
        }
        deserializer.deserialize_str(MacVisitor)
    }
}

impl FromStr for MacAddress {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(MacAddress(
            s.split(':')
                .map(|octet| {
                    if octet.len() != 2 {
                        Err(format!("Invalid octet \"{}\" in MAC address {}", octet, s))
                    } else {
                        u8::from_str_radix(octet, 16).map_err(|_| {
                            format!("Invalid octet \"{}\" in MAC address {}", octet, s)
                        })
                    }
                })
                .collect::<Result<Vec<u8>, _>>()?
                .try_into()
                .map_err(|_| format!("Invalid MAC address: {}", s))?,
        ))
    }
}
