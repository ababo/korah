use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

/// Wrapper to write error chain for Display formatting.
pub struct ErrorChainDisplay<'a, E: Error>(pub &'a E);

impl<E: Error> Display for ErrorChainDisplay<'_, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.0)?;
        let mut source = self.0.source();
        while let Some(cause) = source {
            write!(f, ": {}", cause)?;
            source = cause.source();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_chain_display() {
        #[derive(Debug, thiserror::Error)]
        enum E {
            #[error("v")]
            V,
        }

        #[derive(Debug, thiserror::Error)]
        enum E2 {
            V2(#[source] E),
        }

        impl Display for E2 {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result {
                write!(f, "v2")
            }
        }

        #[derive(Debug, thiserror::Error)]
        enum E3 {
            #[error("v3")]
            V3(#[source] E2),
        }

        let err = E3::V3(E2::V2(E::V));
        let alt = ErrorChainDisplay(&err);
        assert_eq!(format!("{err}"), "v3");
        assert_eq!(format!("{alt}"), "v3: v2: v");
    }
}
