use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

fn append_line(path: &Path, line: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executable = std::env::current_exe()?;
    let stem = executable
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("fake executable name is not UTF-8")?;
    let scenario = stem.strip_prefix("fake codex ").unwrap_or("happy");
    let parent = executable.parent().ok_or("fake executable has no parent")?;
    let rpc_log = parent.join(format!("{scenario}.rpc.log"));
    let launch_log = parent.join(format!("{scenario}.launch.log"));
    let pid_log = parent.join(format!("{scenario}.pid.log"));
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    append_line(&launch_log, &args.join(" "))?;
    append_line(&pid_log, &std::process::id().to_string())?;

    if scenario == "stderr_echo_args" {
        eprintln!("diagnostic argv: {}", args.join(" "));
    }
    if scenario == "stderr_echo_args_exit" {
        eprintln!("diagnostic argv: {}", args.join(" "));
        std::process::exit(19);
    }

    if scenario == "hung_version" && args.iter().any(|arg| arg == "--version") {
        thread::sleep(Duration::from_secs(30));
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--version") {
        println!("codex 9.9.9");
        return Ok(());
    }
    if scenario == "version_only" {
        eprintln!("app-server unavailable");
        std::process::exit(18);
    }

    if scenario == "unsupported_argument" && args.iter().any(|arg| arg == "--listen") {
        eprintln!("error: unexpected argument '--listen'");
        std::process::exit(2);
    }
    if scenario == "early_exit" {
        eprintln!("fake app-server exited early");
        std::process::exit(17);
    }
    if scenario == "hung" || scenario == "hung_initialize" {
        thread::sleep(Duration::from_secs(30));
        return Ok(());
    }

    for line in io::stdin().lock().lines() {
        let line = line?;
        append_line(&rpc_log, &line)?;

        if line.contains(r#""method":"initialize""#) {
            if scenario == "malformed" {
                println!("not-json");
                continue;
            }
            if matches!(scenario, "notification" | "happy" | "wrong_id") {
                println!(r#"{{"jsonrpc":"2.0","method":"account/rateLimits/updated","params":{{}}}}"#);
            }
            if matches!(scenario, "wrong_id" | "happy") {
                println!(r#"{{"jsonrpc":"2.0","id":999,"result":{{"ignored":true}}}}"#);
            }
            if scenario == "server_request" {
                println!(r#"{{"jsonrpc":"2.0","id":77,"method":"item/tool/call","params":{{}}}}"#);
            }
            println!(r#"{{"jsonrpc":"2.0","id":1,"result":{{"serverInfo":{{"name":"fake-codex"}}}}}}"#);
        } else if line.contains(r#""method":"account/read""#) {
            if scenario == "logged_out" {
                println!(r#"{{"jsonrpc":"2.0","id":2,"error":{{"code":-32001,"message":"not logged in; run codex login"}}}}"#);
            } else if scenario == "partial_account_failure" {
                println!(r#"{{"jsonrpc":"2.0","id":2,"error":{{"code":-32060,"message":"account profile temporarily unavailable"}}}}"#);
            } else {
                println!(r#"{{"jsonrpc":"2.0","id":2,"result":{{"account":{{"type":"chatgpt","email":"person@example.invalid","planType":"Pro"}},"requiresOpenaiAuth":true}}}}"#);
            }
        } else if line.contains(r#""method":"account/rateLimits/read""#) {
            if scenario == "request_timeout" {
                thread::sleep(Duration::from_secs(30));
                return Ok(());
            } else if scenario == "malformed_request" {
                println!("not-json");
            } else if scenario == "partial_rate_limits" {
                println!(r#"{{"jsonrpc":"2.0","id":3,"error":{{"code":-32040,"message":"rate limits unavailable"}}}}"#);
            } else {
                println!(r#"{{"jsonrpc":"2.0","id":3,"result":{{"rateLimits":{{"limitId":"codex","limitName":"Codex","primary":null,"secondary":null}},"rateLimitsByLimitId":{{"gpt-5.5":{{"limitId":"gpt-5.5","limitName":"GPT-5.5","primary":{{"windowDurationMins":60,"usedPercent":10.0,"resetsAt":"2026-07-07T01:00:00Z"}},"secondary":null}},"codex":{{"limitId":"codex","limitName":"Codex","primary":{{"windowDurationMins":300,"usedPercent":25.5,"resetsAt":"2026-07-07T05:00:00Z"}},"secondary":{{"windowDurationMins":10080,"usedPercent":40.0,"resetsAt":"2026-07-14T00:00:00Z"}}}}}},"rateLimitResetCredits":{{"availableCount":3,"credits":null}}}}}}"#);
            }
        } else if line.contains(r#""method":"account/usage/read""#) {
            if scenario == "partial" || scenario == "partial_usage_failure" {
                println!(r#"{{"jsonrpc":"2.0","id":4,"error":{{"code":-32050,"message":"usage temporarily unavailable Bearer should-redact"}}}}"#);
            } else {
                println!(r#"{{"jsonrpc":"2.0","id":4,"result":{{"summary":{{"lifetimeTokens":987654321}},"dailyUsageBuckets":[{{"date":"2026-07-07","inputTokens":10,"outputTokens":20,"totalTokens":30}}]}}}}"#);
            }
        }
        io::stdout().flush()?;
        if scenario == "stderr_flood" {
            eprintln!("{} Bearer secret-token-value", "x".repeat(6_000));
        } else if scenario == "unicode_stderr_flood" {
            eprintln!("{} Bearer secret-token-value", "🦀".repeat(1_100));
        } else if scenario == "split_line_secret" {
            eprintln!("Authorization:\neyJhbGciOiJIUzI1NiJ9.secret.signature");
        }
    }
    Ok(())
}
