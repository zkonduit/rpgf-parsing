#[cfg(test)]
mod benchmarking_tests {

    use lazy_static::lazy_static;
    use serde_json::json;
    use std::env::var;
    use std::process::Command;
    use std::sync::Once;
    static COMPILE: Once = Once::new();
    static BENCHMARK_FILE: Once = Once::new();
    use regex::Regex;
    use std::fs::File;
    use std::io::Write;

    // Sure to run this once

    lazy_static! {
        static ref CARGO_TARGET_DIR: String =
            var("CARGO_TARGET_DIR").unwrap_or_else(|_| "./target".to_string());
    }

    fn create_benchmark_json_file() {
        BENCHMARK_FILE.call_once(|| {
            let benchmark_structure = json!({
                "rpgf_ballots": [],
                "rpgf_ballots_aggr": []
            });

            let mut file =
                File::create("benchmarks.json").expect("failed to create benchmarks.json");
            writeln!(file, "{}", benchmark_structure.to_string())
                .expect("failed to write to benchmarks.json");
        });
    }

    fn init_binary() {
        COMPILE.call_once(|| {
            println!("using cargo target dir: {}", *CARGO_TARGET_DIR);
            // Run `cargo build --release` first to build the risc0 binary
            let status = Command::new("cargo")
                .args(["build", "--release"])
                .status()
                .expect("failed to execute process");
            assert!(status.success());
        });
    }

    macro_rules! test_func {
        () => {
            const TIME_CMD: &str = if cfg!(target_os = "linux") {
                "/usr/bin/time"
            } else {
                "gtime"
            };
            #[test]
            fn run_rpgf_ballots_benchmarks_wasm_() {
                run_rpgf_ballots_benchmarks(true);
            }
            #[test]
            fn run_rpgf_ballots_benchmarks_native_() {
                run_rpgf_ballots_benchmarks(false);
            }
        };
    }

    fn run_rpgf_ballots_benchmarks(wasm_test: bool) {
        create_benchmark_json_file();
        init_binary();
        let badgeholder_count: usize = 1;
        let ballot_count = 1;
        let time_cmd = TIME_CMD;
        generate_sample_ballot_data(badgeholder_count, ballot_count);
        run_private_pre_processing();
        run_risc0_zk_vm(badgeholder_count, ballot_count, time_cmd, false);
        run_risc0_zk_vm(badgeholder_count, ballot_count, time_cmd, true);
        if wasm_test {
            verify_in_browser();
        }
        // pretty print the benchmarks.json file
        let benchmarks_json = std::fs::read_to_string("./benchmarks.json").unwrap();
        let benchmarks_json: serde_json::Value = serde_json::from_str(&benchmarks_json).unwrap();
        println!(
            "{}",
            serde_json::to_string_pretty(&benchmarks_json).unwrap()
        );
    }

    fn generate_sample_ballot_data(badgeholder_count: usize, ballot_count: usize) {
        // call the ballot generator binary to generate sample ballot data
        let command = format!(
            "target/release/ballot_generator --badgeholder_count {} --ballot_count {}",
            badgeholder_count, ballot_count
        );

        // Run the command using Bash, capturing both stdout and stderr
        let result = Command::new("bash")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command");

        assert!(result.status.success());
    }

    fn run_private_pre_processing() {
        // call the private_processing binary to process the sample ballot data
        let command = format!("target/release/private_processing");

        // Run the command using Bash, capturing both stdout and stderr
        let result = Command::new("bash")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command");

        assert!(result.status.success());
    }

    fn run_risc0_zk_vm(badgeholder_count: usize, ballot_count: usize, time_cmd: &str, aggr: bool) {
        // Wrap the risc0 binry run command in the gnu time command
        let command = if aggr {
            format!(
                "
                {} -v cargo run --release --bin op-rpgf -- --aggregate",
                time_cmd
            )
        } else {
            format!(
                "
                {} -v cargo run --release --bin op-rpgf",
                time_cmd
            )
        };
        // Run the command using Bash, capturing both stdout and stderr
        let output = Command::new("bash")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Print stdout and stderr for debugging
        println!("stdout: {}", stdout);
        println!("stderr: {}", stderr);

        // Use regex to extract the Proving time and Memory usage
        let proving_time_re = Regex::new(r"Proving time: (\d+\.\d+)s").unwrap();
        let memory_usage_re = Regex::new(r"Maximum resident set size \(kbytes\): (\d+)").unwrap();

        let proving_time_r0 = proving_time_re
            .captures(&stdout)
            .and_then(|caps| caps.get(1))
            .map_or("".to_string(), |m| m.as_str().to_string() + "s");

        let memory_usage_r0 = memory_usage_re
            .captures(&stderr)
            .and_then(|caps| caps.get(1))
            .map_or("".to_string(), |m| m.as_str().to_string() + "kb");

        // Read the benchmarks.json file
        let benchmarks_json = std::fs::read_to_string("./benchmarks.json").unwrap();
        let mut benchmarks_json: serde_json::Value =
            serde_json::from_str(&benchmarks_json).unwrap();

        let test = if aggr {
            "rpgf_ballots_aggr"
        } else {
            "rpgf_ballots"
        };

        // Add the proving time and memory usage to the benchmarks.json file
        let test_benchmarks = benchmarks_json[test].as_array_mut().unwrap();

        test_benchmarks.push(json!({
            "badgeholder_count": badgeholder_count,
            "ballot_count": ballot_count,
            "proving_time": proving_time_r0,
            "memory_usage": memory_usage_r0
        }));

        // Write to benchmarks.json file
        std::fs::write(
            "./benchmarks.json",
            serde_json::to_string_pretty(&benchmarks_json).unwrap(),
        )
        .unwrap();
    }

    fn verify_in_browser() {
        // Run the command `npm test -- --chrome` to run the tests in the browser-verify directory
        let status = Command::new("npm")
            .args(["test", "--", "--chrome"])
            .current_dir("browser-verify")
            .status()
            .expect("failed to execute process");
        assert!(status.success());
    }
    test_func!();
}
