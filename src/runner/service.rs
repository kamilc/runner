tonic::include_proto!("service");

macro_rules! impl_from_anyhow {
    ($($for:path), +) => {
        $(
            impl<T> std::convert::From<T> for $for
            where
                T: std::convert::Into<anyhow::Error>,
            {
                fn from(error: T) -> $for {
                    run_response::Error {
                        description: error.into().to_string(),
                        errors: Some(run_response::error::Errors::GeneralError(1)),
                    }
                }
            }
        )+
    };
}

impl_from_anyhow!(run_response::Error);
