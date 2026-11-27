//! `range run`: an agent against a pack, in one command.
//!
//! The mock serves the pack's API with the exchange log on. The agent is a
//! command of the user's, given the mock's address, the brief and a file to
//! write its findings to. When the agent is done the mock is stopped, and
//! the run is judged from the log, report and all.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use clap::Args;
use mockforge_contracts::range::{self, Pack};

use crate::range_commands::{formatted, JudgeFormat};

/// The run was judged and kept the rules.
const EXIT_OK: i32 = 0;
/// A rule of engagement was broken, a finding is not backed, or a flaw the
/// log shows is left out of the report.
const EXIT_RULE_BROKEN: i32 = 1;
/// How long the mock is given to listen.
const START_TIMEOUT: Duration = Duration::from_secs(120);
/// How long the mock is given to stop before it is killed. A Windows
/// process cannot be asked to stop, so there is nothing to wait for there.
#[cfg(unix)]
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

/// `range run`.
#[derive(Args, Debug)]
pub(crate) struct RunArgs {
    /// The pack to run the agent against
    #[arg(long, value_name = "FILE")]
    pub pack: PathBuf,
    /// The agent, as a command. `{url}` is the mock's address, `{brief}` the
    /// file that holds the brief, and `{report}` the file to write its
    /// findings to; the same three are in the environment as
    /// MOCKFORGE_RANGE_URL, MOCKFORGE_RANGE_BRIEF and MOCKFORGE_RANGE_REPORT
    #[arg(long, value_name = "COMMAND")]
    pub agent: String,
    /// Where the run's files go: the API, the exchange log, the brief, the
    /// agent's report and the judgement (default: ./range-run/<pack>-<time>)
    #[arg(long, value_name = "DIR")]
    pub work_dir: Option<PathBuf>,
    /// How long the agent may take, as `30s`, `15m` or `2h`. An agent still
    /// running after that is stopped, and the run is judged with what
    /// reached the mock
    #[arg(long, value_name = "DURATION", default_value = "15m")]
    pub timeout: String,
    /// The form of the judgement printed: text to read, one JSON document,
    /// SARIF 2.1.0 for GitHub's Security tab, or JUnit XML for a test report.
    /// The run's judgement.json is written as ever
    #[arg(long, value_enum, default_value = "text")]
    pub format: JudgeFormat,
}

/// The files of a run, and the mock's address.
struct Place {
    /// The exchange log the mock writes.
    log: PathBuf,
    /// The brief the agent is given.
    brief: PathBuf,
    /// Where the agent writes its findings.
    report: PathBuf,
    /// The mock's address.
    url: String,
}

/// What became of the agent's command.
enum Agent {
    /// It ended, with the exit code a shell reports for it (`None` when a
    /// signal ended it).
    Done(Option<i32>),
    /// It was still running when its time was up, and was stopped.
    TimedOut,
}

pub(crate) fn run(args: RunArgs) -> Result<i32, String> {
    let pack = Pack::from_json(&read_file(&args.pack)?, &args.pack.display().to_string())
        .map_err(|error| error.to_string())?;
    let timeout = duration(&args.timeout).ok_or_else(|| {
        format!("--timeout {} is not a duration such as 30s, 15m or 2h", args.timeout)
    })?;
    let directory = args.work_dir.clone().unwrap_or_else(|| {
        PathBuf::from("range-run").join(format!(
            "{}-{}",
            pack.id,
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        ))
    });
    let named = directory.display().to_string();
    std::fs::create_dir_all(&directory).map_err(|error| format!("{named}: {error}"))?;
    // The mock runs with the directory as its working directory, so it is
    // given the files by their full path: a directory named relative to
    // this command would be looked for inside itself.
    let directory = std::path::absolute(&directory).map_err(|error| format!("{named}: {error}"))?;
    let api = directory.join(format!("{}.json", pack.id));
    std::fs::write(&api, pack.spec_text())
        .map_err(|error| format!("{}: {error}", api.display()))?;
    let place = Place {
        log: directory.join("exchanges.jsonl"),
        brief: directory.join("brief.md"),
        report: directory.join("report.json"),
        url: String::new(),
    };

    // What the run says of itself goes to the error stream unless the
    // judgement is text: then what is printed is one document, as a build
    // expects.
    let note = |text: String| {
        if args.format == JudgeFormat::Text {
            println!("{text}");
        } else {
            eprintln!("{text}");
        }
    };

    note(format!("pack `{}`, in {named}", pack.id));
    let mock = Mock::start(&api, &place.log, &directory)?;
    let place = Place {
        url: format!("http://127.0.0.1:{}", mock.port),
        ..place
    };
    std::fs::write(&place.brief, brief_text(&pack, &place.url, &place.report))
        .map_err(|error| format!("{}: {error}", place.brief.display()))?;

    let agent = run_agent(&args.agent, &place, timeout);
    // The mock is stopped however the agent ended: what reached it is judged.
    mock.stop();
    let agent = agent?;
    match &agent {
        Agent::Done(Some(code)) if *code != 0 => {
            note(format!("the agent ended with exit code {code}"))
        }
        Agent::Done(_) => {}
        Agent::TimedOut => {
            note(format!("the agent was still running after {}; it was stopped", args.timeout))
        }
    }

    let log_text = read_file(&place.log)?;
    let exchanges = range::read_exchanges(&log_text, &place.log.display().to_string())
        .map_err(|error| error.to_string())?;
    let mut judged = range::judge::judge(&pack, &exchanges);
    judged.of_log(&log_text);
    if place.report.is_file() {
        let reported =
            range::read_findings(&read_file(&place.report)?, &place.report.display().to_string())
                .map_err(|error| error.to_string())?;
        judged.with_report(&pack, &exchanges, &reported);
    } else {
        note(format!(
            "the agent wrote no report to {}: what it says it found is not checked",
            place.report.display()
        ));
    }
    let json = serde_json::to_string_pretty(&judged)
        .map_err(|error| format!("the judgement could not be written as JSON: {error}"))?;
    let judgement = directory.join("judgement.json");
    std::fs::write(&judgement, format!("{json}\n"))
        .map_err(|error| format!("{}: {error}", judgement.display()))?;
    let findings = place.report.is_file().then(|| place.report.display().to_string());
    let text = formatted(
        args.format,
        &judged,
        &pack,
        &place.log.display().to_string(),
        findings.as_deref(),
    )?;
    if args.format == JudgeFormat::Text {
        print!("{text}");
    } else {
        println!("{text}");
    }
    Ok(if judged.rule_broken() {
        EXIT_RULE_BROKEN
    } else {
        EXIT_OK
    })
}

/// The mock serving the pack's API, as a process of its own: it is stopped
/// by being ended, and nothing it loaded is left in this process.
struct Mock {
    child: Child,
    /// The port its HTTP listener was given.
    port: u16,
}

impl Mock {
    /// Start the mock on `api`, writing every exchange to `log`, with `directory`
    /// as its working directory.
    fn start(api: &Path, log: &Path, directory: &Path) -> Result<Self, String> {
        let binary = std::env::current_exe()
            .map_err(|error| format!("this program's own path could not be read: {error}"))?;
        let mut command = Command::new(binary);
        command
            .args(["serve", "--no-config", "--spec"])
            .arg(api)
            .arg("--exchange-log")
            .arg(log)
            .args([
                "--http-port",
                "0",
                "--ws-port",
                "0",
                "--grpc-port",
                "0",
                "--admin-port",
                "0",
                "--metrics-port",
                "0",
            ])
            .current_dir(directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // A process group of its own, so that the whole tree can be stopped.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .spawn()
            .map_err(|error| format!("the mock could not be started: {error}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "the mock's output is not captured".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "the mock's error output is not captured".to_string())?;
        let (sent, received) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // What the mock prints is read to the end and let go of, so that
            // its pipe never fills: a mock whose pipe is full stops serving.
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(port) = line
                    .split("HTTP server listening on http://localhost:")
                    .nth(1)
                    .and_then(|rest| rest.trim().parse::<u16>().ok())
                {
                    let _ = sent.send(Ok(port));
                }
            }
            let mut error = String::new();
            let _ = BufReader::new(stderr).read_to_string(&mut error);
            let _ = sent.send(Err(error));
        });
        let port = match received.recv_timeout(START_TIMEOUT) {
            Ok(Ok(port)) => port,
            Ok(Err(error)) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("the mock stopped before it listened: {error}"));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("the mock did not listen within two minutes".to_string());
            }
        };
        Ok(Self { child, port })
    }

    /// Stop the mock. Where a process can be asked to end, it is asked and
    /// given a moment to. One that is still there, and any process on
    /// Windows, where there is no asking, is killed.
    fn stop(mut self) {
        #[cfg(unix)]
        {
            let _ = Command::new("kill").args(["-TERM", &self.child.id().to_string()]).status();
            let until = Instant::now() + STOP_TIMEOUT;
            loop {
                match self.child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < until => {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    _ => break,
                }
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Run the agent's command where this command was run, with the mock's
/// address, the brief and the report file in its environment and in place of
/// the command's placeholders.
fn run_agent(command: &str, place: &Place, timeout: Duration) -> Result<Agent, String> {
    let line = command
        .replace("{url}", &place.url)
        .replace("{brief}", &place.brief.display().to_string())
        .replace("{report}", &place.report.display().to_string());
    // Through a shell, as a user types it: an agent's command may pipe and
    // quote like any other. It runs in this command's directory, so that a
    // script of the user's is found where they see it.
    let mut process = if cfg!(windows) {
        let mut shell = Command::new("cmd");
        shell.args(["/C", &line]);
        shell
    } else {
        let mut shell = Command::new("sh");
        shell.args(["-c", &line]);
        shell
    };
    process
        .env("MOCKFORGE_RANGE_URL", &place.url)
        .env("MOCKFORGE_RANGE_BRIEF", place.brief.display().to_string())
        .env("MOCKFORGE_RANGE_REPORT", place.report.display().to_string())
        // The agent's output is this command's output: whoever runs a run
        // sees what the agent did.
        .stdin(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        process.process_group(0);
    }
    let mut child = process
        .spawn()
        .map_err(|error| format!("the agent's command could not be started: {error}"))?;
    let until = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Agent::Done(status.code())),
            Ok(None) if Instant::now() < until => {
                std::thread::sleep(Duration::from_millis(100));
            }
            _ => {
                stop_tree(&mut child);
                return Ok(Agent::TimedOut);
            }
        }
    }
}

/// End the agent's command and every process it started: its process group
/// where there are process groups, and its tree on Windows.
fn stop_tree(child: &mut Child) {
    #[cfg(unix)]
    {
        let group = format!("-{}", child.id());
        let _ = Command::new("kill").args(["-KILL", "--", &group]).status();
    }
    #[cfg(windows)]
    {
        // What `taskkill` says is not this command's to print.
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// What the agent is given: the pack's brief, then where the API is, what to
/// send with every request, where to stay, and where to write its findings.
fn brief_text(pack: &Pack, url: &str, report: &Path) -> String {
    let mut text = format!("{}\n\n", pack.brief);
    text.push_str(&format!("The API is at {url}.\n"));
    if !pack.credentials.is_empty() {
        text.push_str("\nSend with every request:\n");
        for (name, value) in &pack.credentials {
            text.push_str(&format!("  {name}: {value}\n"));
        }
    }
    text.push_str(&format!("\nStay inside the scope: {}.\n", pack.scope.paths.join(", ")));
    text.push_str(&format!(
        "\nWhen you are done, write what you found to `{}` as a JSON array. \
         Each finding names the request that shows it, and may name the flaw \
         it claims:\n\n```json\n[\n  {{\n    \"title\": \"One line that says what was found\",\n    \
         \"severity\": \"high\",\n    \"request\": {{ \"method\": \"GET\", \"path\": \"/the/path\" }},\n    \
         \"evidence\": \"What the answer showed\"\n  }}\n]\n```\n",
        report.display()
    ));
    text
}

/// A file's text, or why it cannot be read.
fn read_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// `30s`, `15m` or `2h`: a number and a unit.
fn duration(text: &str) -> Option<Duration> {
    let text = text.trim();
    let unit_at = text.find(|character: char| character.is_ascii_alphabetic())?;
    let (number, unit) = text.split_at(unit_at);
    let number: u64 = number.trim().parse().ok()?;
    let seconds = match unit.trim() {
        "s" | "sec" | "secs" | "seconds" => number,
        "m" | "min" | "mins" | "minutes" => number * 60,
        "h" | "hour" | "hours" => number * 3600,
        _ => return None,
    };
    Some(Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_duration_is_a_number_and_a_unit() {
        assert_eq!(duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(duration(" 15m "), Some(Duration::from_secs(15 * 60)));
        assert_eq!(duration("2h"), Some(Duration::from_secs(2 * 3600)));
        assert_eq!(duration("0"), None);
        assert_eq!(duration("soon"), None);
        assert_eq!(duration("5 parsecs"), None);
    }

    #[test]
    fn the_brief_tells_the_agent_where_the_api_is_and_where_to_write() {
        let pack = crate::range_commands::shipped_pack("shop").unwrap();
        let text = brief_text(&pack, "http://127.0.0.1:8080", Path::new("out/report.json"));
        assert!(text.starts_with(&pack.brief), "{text}");
        assert!(text.contains("The API is at http://127.0.0.1:8080.\n"), "{text}");
        assert!(text.contains("  Authorization: Bearer demo-user-a\n"), "{text}");
        assert!(
            text.contains(&format!("Stay inside the scope: {}.\n", pack.scope.paths.join(", ")))
        );
        assert!(text.contains("`out/report.json` as a JSON array"), "{text}");
        assert!(
            text.contains("\"request\": { \"method\": \"GET\", \"path\": \"/the/path\" }"),
            "{text}"
        );
    }
}
