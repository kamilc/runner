tonic::include_proto!("service");

macro_rules! impl_from_anyhow {
    ($for:path, $error_type:expr) => {
        impl<T> std::convert::From<T> for $for
        where
            T: std::convert::Into<anyhow::Error>,
        {
            fn from(error: T) -> $for {
                $for {
                    description: error.into().to_string(),
                    errors: Some($error_type(1)),
                }
            }
        }
    };
}

impl_from_anyhow!(
    run_response::RunError,
    run_response::run_error::Errors::GeneralError
);

impl_from_anyhow!(
    stop_response::StopError,
    stop_response::stop_error::Errors::GeneralError
);

impl_from_anyhow!(
    status_response::StatusError,
    status_response::status_error::Errors::GeneralError
);

impl_from_anyhow!(
    log_response::LogError,
    log_response::log_error::Errors::GeneralError
);
