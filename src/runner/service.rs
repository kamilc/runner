tonic::include_proto!("service");

impl std::fmt::Display for run_response::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}

macro_rules! impl_from_error {
    ($to:path, $($from:path), +) => {
        $(
            impl std::convert::From<$from> for $to {
                fn from(error: $from) -> $to {
                    run_response::Error {
                        description: error.to_string(),
                        errors: Some(run_response::error::Errors::GeneralError(1)),
                    }
                }
            }
        )+
    };
}

impl_from_error!(run_response::Error, anyhow::Error, cgroups_rs::error::Error);
