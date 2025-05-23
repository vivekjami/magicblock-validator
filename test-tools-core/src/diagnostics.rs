use std::{env, path::Path};

use log::{error, info};
use solana_svm::transaction_commit_result::CommittedTransaction;

// -----------------
// init_logger
// -----------------
pub fn init_logger_for_test_path(full_path_to_test_file: &str) {
    // In order to include logs from the test themselves we need to add the
    // name of the test file (minus the extension) to the RUST_LOG filter
    let mut rust_log = env::var(env_logger::DEFAULT_FILTER_ENV)
        .ok()
        .unwrap_or("".into());
    if rust_log.ends_with(',') || rust_log.is_empty() {
        let p = Path::new(full_path_to_test_file);
        let file = p.file_stem().unwrap();
        let test_level =
            env::var("RUST_TEST_LOG").unwrap_or("info".to_string());
        rust_log.push_str(&format!(
            "{}={}",
            file.to_str().unwrap(),
            test_level
        ));
        env::set_var(env_logger::DEFAULT_FILTER_ENV, rust_log);
    }

    let _ = env_logger::builder()
        .format_timestamp_micros()
        .is_test(true)
        .try_init();
}

#[macro_export]
macro_rules! init_logger {
    () => {
        $crate::diagnostics::init_logger_for_test_path(::std::file!());
    };
}

// -----------------
// Solana Logs
// -----------------
pub fn log_exec_details(transaction_results: &CommittedTransaction) {
    info!("");
    info!("=============== Logs ===============");
    let logs = match &transaction_results.status {
        Ok(_) => &transaction_results.log_messages,
        Err(error) => {
            error!("error executing transaction: {error}");
            return;
        }
    };

    if let Some(logs) = logs {
        for log in logs {
            info!("> {log}");
        }
    }
    info!("");
}
