use regex::Regex;
use std::fs::{File, read_dir};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug)]
struct ProcessRecord {
    pid: u32,
    ppid: u32,
    cmdline: String,
}

#[derive(Debug)]
enum PidReadError {
    ParseError(String),
    IOError(std::io::Error),
}

impl From<std::num::ParseIntError> for PidReadError {
    fn from(err: std::num::ParseIntError) -> PidReadError {
        PidReadError::ParseError(format!("{}", err))
    }
}
impl From<&str> for PidReadError {
    fn from(err: &str) -> PidReadError {
        PidReadError::ParseError(String::from(err))
    }
}

impl From<std::io::Error> for PidReadError {
    fn from(err: std::io::Error) -> PidReadError {
        PidReadError::IOError(err)
    }
}

fn get_pid_info(pid_dir: &Path) -> Result<ProcessRecord, PidReadError>  {
    let params = read_pid_file(&pid_dir)?;

    let pid = get_u32_param(&params, "Pid:")?;
    let ppid = get_u32_param(&params, "PPid:")?;
    let cmdline = parse_cmdline(&pid_dir)?;

    Ok(ProcessRecord { pid, ppid, cmdline, })
}

fn read_pid_file(pid_dir: &Path) -> std::io::Result<HashMap<String, Vec<String>>> {
    let status_file = pid_dir.join("status");
    let handle = File::open(status_file.as_path())?;
    let reader = BufReader::new(handle);
    let mut params: HashMap<String, Vec<String>> = HashMap::new();
    for line in reader.lines() {
        let line = line?;
        let v: Vec<_> = line.split('\t').into_iter().collect();
        let (head, tail) = v.split_at(1);
        let tail: Vec<_> = tail.into_iter().map(|e| e.to_string()).collect();
        let head = head[0].clone();
        params.insert(String::from(head), tail);
    }
    Ok(params)
}

fn parse_cmdline(pid_dir: &Path) -> Result<String, PidReadError> {
    let status_file = pid_dir.join("cmdline");
    let handle = File::open(status_file.as_path())?;
    let mut reader = BufReader::new(handle);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let tokens: Vec<&str> = line.split('\0').collect();
    Ok(tokens.join(" "))
}

fn get_u32_param(params: &HashMap<String, Vec<String>>, param: &str) -> Result<u32, PidReadError> {
    match params.get(param) {
        Some(p) => Ok(p[0].parse::<u32>()?),
        None    => Err(PidReadError::ParseError(format!("missing {} parameter", param))),
    }
}

fn visit_pids(dir: &Path) -> Result<HashMap<u32, ProcessRecord>, PidReadError> {
    let re = Regex::new(r"^/proc/[0-9]+$").unwrap();
    let mut pids = HashMap::new();

    if dir.is_dir() {
        for entry in read_dir(dir)? {
            let file = entry?;
            let pathbuf = file.path();
            let path = pathbuf.as_path();
            let path_str = path.to_string_lossy();

            if path.is_dir() && re.is_match(&path_str) {
                let proc = get_pid_info(path)?;
                pids.insert(proc.pid, proc);
            }
        }
    }
    Ok(pids)
}

fn main() {
    match visit_pids(Path::new("/proc")) {
        Ok(pids) => println!("Pids count {}:\n{:?}", pids.len(), pids),
        Err(err) => println!("Error: {:?}", err),
    }
}
