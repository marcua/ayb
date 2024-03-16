#[macro_export]
macro_rules! try_from_i16 {
    ($struct:ident, { $($left:literal => $right:expr),+ }) => {
        impl TryFrom<i16> for $struct {
            type Error = AybError;

            fn try_from(value: i16) -> Result<Self, Self::Error> {
                match value {
                    $($left => Ok($right),)*
                    _ => Err(Self::Error::Other {
                        message: format!("Unknown value: {}", value),
                    }),
                }
            }
        }
    };
}

#[macro_export]
macro_rules! from_str {
    ($struct:ident, { $($left:literal => $right:expr),+ }) => {
        impl FromStr for $struct {
            type Err = AybError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($left => Ok($right),)*
                    _ => Err(Self::Err::Other {
                        message: format!("Unknown value: {}", value),
                    }),
                }
            }
        }
    };
}
