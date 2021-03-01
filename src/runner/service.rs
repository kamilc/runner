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

/// Represents a task specific error. This is later converted into
/// a type that's linked with the error for specific request.
/// Provided here to keep error types handling away from the
/// logic to keep it clear to think about
#[derive(Clone, Debug)]
pub struct TaskError {
    pub description: String,
    pub variant: i32,
}

/// Represents an internal specific error. This is later converted into
/// a general error as defined in teh service.
/// Provided here to keep error types handling away from the
/// logic to keep it clear to think about
#[derive(Clone, Debug)]
pub struct InternalError {
    pub description: String,
}

macro_rules! impl_from_task_error {
    ($err:path, $variant:expr) => {
        impl std::convert::From<TaskError> for $err {
            fn from(error: TaskError) -> $err {
                $err {
                    description: error.description,
                    errors: Some($variant(error.variant)),
                }
            }
        }
    };
}

macro_rules! impl_from_internal_error {
    ($err:path, $variant:expr) => {
        impl std::convert::From<InternalError> for $err {
            fn from(error: InternalError) -> $err {
                $err {
                    description: error.description,
                    errors: Some($variant(GeneralError::InternalError as i32)),
                }
            }
        }
    };
}

macro_rules! task_error {
    ($desc:literal, $variant:path) => {
        Err(TaskError {
            description: $desc.to_string(),
            variant: $variant as i32,
        }
        .into())
    };
}

macro_rules! internal_error {
    ($desc:literal) => {
        Err(InternalError {
            description: $desc.to_string(),
        }
        .into())
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

impl_from_task_error!(
    run_response::RunError,
    run_response::run_error::Errors::RunError
);

impl_from_task_error!(
    stop_response::StopError,
    stop_response::stop_error::Errors::StopError
);

impl_from_task_error!(
    status_response::StatusError,
    status_response::status_error::Errors::StatusError
);

impl_from_task_error!(
    log_response::LogError,
    log_response::log_error::Errors::LogError
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
