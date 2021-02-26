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

pub struct TaskError {
    pub description: String,
    pub variant: i32,
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

macro_rules! task_error {
    ($desc:literal, $variant:path) => {
        Err(TaskError {
            description: $desc.to_string(),
            variant: $variant as i32,
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
