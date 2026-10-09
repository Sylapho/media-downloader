use std::{future::Future, io, path::PathBuf, process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command, time::timeout};

pub(crate) struct ProcessRequest {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub timeout: Duration,
    pub stdout_limit: u64,
    pub stderr_limit: u64,
}

#[derive(Debug)]
pub(crate) struct ProcessOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ProcessError {
    NotFound,
    PermissionDenied,
    Failed(String),
    Timeout,
    OutputLimit,
}

pub(crate) trait ProcessRunner: Sync {
    fn run(
        &self,
        request: ProcessRequest,
    ) -> impl Future<Output = Result<ProcessOutput, ProcessError>> + Send;
}

pub(crate) struct TokioProcessRunner;

fn io_error(error: io::Error) -> ProcessError {
    match error.kind() {
        io::ErrorKind::NotFound => ProcessError::NotFound,
        io::ErrorKind::PermissionDenied => ProcessError::PermissionDenied,
        _ => ProcessError::Failed(error.to_string()),
    }
}

async fn read_bounded(
    reader: impl tokio::io::AsyncRead + Unpin,
    limit: u64,
) -> Result<Vec<u8>, ProcessError> {
    let mut output = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut output)
        .await
        .map_err(io_error)?;
    if output.len() as u64 > limit {
        return Err(ProcessError::OutputLimit);
    }
    Ok(output)
}

impl ProcessRunner for TokioProcessRunner {
    async fn run(&self, request: ProcessRequest) -> Result<ProcessOutput, ProcessError> {
        let mut command = Command::new(request.program);
        command
            .args(request.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        // Avoid flashing console windows when inspecting native Windows tools.
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let mut child = command.spawn().map_err(io_error)?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProcessError::Failed("Missing stdout pipe".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ProcessError::Failed("Missing stderr pipe".into()))?;
        let result = timeout(request.timeout, async {
            tokio::try_join!(
                async { child.wait().await.map_err(io_error) },
                read_bounded(stdout, request.stdout_limit),
                read_bounded(stderr, request.stderr_limit),
            )
        })
        .await;

        match result {
            Ok(Ok((status, stdout, stderr))) => Ok(ProcessOutput {
                success: status.success(),
                stdout,
                stderr,
            }),
            result => {
                // Reap the child after errors/timeouts; kill_on_drop also handles cancellation.
                let _ = child.kill().await;
                match result {
                    Err(_) => Err(ProcessError::Timeout),
                    Ok(Err(error)) => Err(error),
                    Ok(Ok(_)) => unreachable!(),
                }
            }
        }
    }
}
