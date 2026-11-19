use std::sync::LazyLock;

use anyhow::{Result, anyhow};
use ort::environment::Environment;

static ONNX_RUNTIME_ENVIRONMENT: LazyLock<Result<Environment, String>> = LazyLock::new(|| {
    ort::init()
        .with_name("crowndictator")
        .build()
        .map_err(|error| error.to_string())
});

pub fn onnx_runtime_environment() -> Result<&'static Environment> {
    ONNX_RUNTIME_ENVIRONMENT
        .as_ref()
        .map_err(|error| anyhow!("{error}"))
}
