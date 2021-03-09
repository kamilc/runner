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

/// Represents an internal specific error. This is later converted into
/// a general error as defined in the service.
/// Provided here to keep error types handling away from the
/// logic to keep it clear to think about
#[derive(Debug)]
pub struct InternalError {
    pub description: String,
}

macro_rules! impl_from_internal_error {
    ($err:path, $variant:expr) => {
        impl std::convert::From<InternalError> for $err {
            fn from(error: InternalError) -> $err {
                $err {
                    description: error.description.to_string(),
                    errors: Some($variant(GeneralError::InternalError as i32)),
                }
            }
        }
    };
}

impl std::fmt::Display for stop_response::stop_error::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            stop_response::stop_error::Error::ProcessAlreadyStoppedError => {
                write!(f, "Process already stopped")
            }
            stop_response::stop_error::Error::CouldntStopError => {
                write!(f, "Couldn't kill a process")
            }
            stop_response::stop_error::Error::InvalidId => {
                write!(f, "Invalid process id")
            }
            stop_response::stop_error::Error::ProcessNotFoundError => {
                write!(f, "Process not found")
            }
        }
    }
}

impl std::fmt::Display for run_response::run_error::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            run_response::run_error::Error::NameEmptyError => {
                write!(f, "Command name empty")
            }
        }
    }
}

impl std::fmt::Display for status_response::status_error::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            status_response::status_error::Error::InvalidId => {
                write!(f, "Invalid process id")
            }
            status_response::status_error::Error::ProcessNotFoundError => {
                write!(f, "Process not found")
            }
        }
    }
}

impl std::fmt::Display for log_response::log_error::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            log_response::log_error::Error::InvalidId => {
                write!(f, "Invalid process id")
            }
            log_response::log_error::Error::ProcessNotFoundError => {
                write!(f, "Process not found")
            }
        }
    }
}

impl std::convert::From<run_response::run_error::Error> for run_response::RunError {
    fn from(error: run_response::run_error::Error) -> run_response::RunError {
        run_response::RunError {
            description: format!("{}", error),
            errors: Some(run_response::run_error::Errors::RunError(error as i32)),
        }
    }
}

impl std::convert::From<stop_response::stop_error::Error> for stop_response::StopError {
    fn from(error: stop_response::stop_error::Error) -> stop_response::StopError {
        stop_response::StopError {
            description: format!("{}", error),
            errors: Some(stop_response::stop_error::Errors::StopError(error as i32)),
        }
    }
}

impl std::convert::From<status_response::status_error::Error> for status_response::StatusError {
    fn from(error: status_response::status_error::Error) -> status_response::StatusError {
        status_response::StatusError {
            description: format!("{}", error),
            errors: Some(status_response::status_error::Errors::StatusError(
                error as i32,
            )),
        }
    }
}

impl std::convert::From<log_response::log_error::Error> for log_response::LogError {
    fn from(error: log_response::log_error::Error) -> log_response::LogError {
        log_response::LogError {
            description: format!("{}", error),
            errors: Some(log_response::log_error::Errors::LogError(error as i32)),
        }
    }
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

impl_from_internal_error!(
    run_response::RunError,
    run_response::run_error::Errors::RunError
);

impl_from_internal_error!(
    stop_response::StopError,
    stop_response::stop_error::Errors::StopError
);

impl_from_internal_error!(
    status_response::StatusError,
    status_response::status_error::Errors::StatusError
);

impl_from_internal_error!(
    log_response::LogError,
    log_response::log_error::Errors::LogError
);
