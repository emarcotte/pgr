use getopts::{Matches, Options,};
use std::collections::HashMap;
use std::fs::{File, read_dir, DirEntry, };
use std::io::{BufRead, BufReader, };
use std::path::Path;
use users::{get_current_uid};

type ProcessMap = HashMap<u32, ProcessRecord>;
type ProcessParams = HashMap<String, Vec<String>>;

#[derive(Debug)]
struct ProcessRecord {
    pid: u32,
    uid: u32,
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

#[derive(Debug)]
struct Process {
    pid: u32,
    uid: u32,
    ppid: u32,
    cmdline: String,
    children: Vec<Process>,
}

impl Process {
    fn new(rec: &ProcessRecord, tree: &HashMap<u32, Vec<&ProcessRecord>>) -> Process {
        let mut proc = Process {
            children: match tree.get(&rec.pid) {
                Some(children) => children
                    .iter()
                    .map(|c| Process::new(&c, &tree))
                    .collect(),
                None           => vec!(),
            },
            cmdline:  rec.cmdline.clone(),
            pid:      rec.pid,
            ppid:     rec.ppid,
            uid:      rec.uid,
        };
        proc.children.sort_by_key(|k| k.pid);
        proc
    }

    fn search<'a>(self: &'a Process, result: &mut Vec<&'a Process>, matcher: &Fn(&Process) -> bool) {
        if matcher(self) {
            result.push(self);
        }
        else {
            for child in &self.children {
                child.search(result, matcher);
            }
        }
    }
}

fn get_string_param(params: &ProcessParams, param: &str) -> Result<String, PidReadError> {
    match params.get(param) {
        Some(p) => Ok(p[0].clone()),
        None    => Err(PidReadError::ParseError(format!("missing {} parameter", param))),
    }
}

fn get_u32_param(params: &ProcessParams, param: &str) -> Result<u32, PidReadError> {
    match params.get(param) {
        Some(p) => Ok(p[0].parse::<u32>()?),
        None    => Err(PidReadError::ParseError(format!("missing {} parameter", param))),
    }
}

fn get_pid_info(pid_dir: &Path) -> Result<ProcessRecord, PidReadError>  {
    let params = read_pid_file(&pid_dir)?;

    let pid = get_u32_param(&params, "Pid:")?;
    let ppid = get_u32_param(&params, "PPid:")?;
    let uid = get_u32_param(&params, "Uid:")?;
    let status = get_string_param(&params, "State:")?;
    let mut cmdline = parse_cmdline(&pid_dir)?;

    if cmdline.is_empty() {
        cmdline = get_string_param(&params, "Name:")?;
        cmdline = format!("[{}]", cmdline);
    }

    if status.starts_with('Z') {
        cmdline = format!("[{}] zombie!", cmdline);
    }

    Ok(ProcessRecord { pid, ppid, uid, cmdline, })
}

fn read_pid_file(pid_dir: &Path) -> std::io::Result<ProcessParams> {
    let status_file = pid_dir.join("status");
    let handle = File::open(status_file.as_path())?;
    let reader = BufReader::new(handle);
    let mut params = ProcessParams::new();
    for line in reader.lines() {
        let line = line?;
        let v: Vec<_> = line.split('\t').collect();
        let (head, tail) = v.split_at(1);
        let tail: Vec<_> = tail.iter().map(|e| e.to_string()).collect();
        let head = head[0];
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
    Ok(
        line
            .split('\0')
            .map(|s| {
                if s.contains(' ') {
                    format!("\"{}\"", s)
                }
                else {
                    s.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    )
}

fn visit_pids(dir: &Path) -> Result<ProcessMap, PidReadError> {
    let mut pids = HashMap::new();

    for entry in read_dir(dir)? {
        let file: DirEntry = entry?;
        let pathbuf = file.path();
        if let Some(file_name) = pathbuf.file_name() {
            let name = file_name.to_string_lossy();
            if pathbuf.is_dir() && name.chars().all(char::is_numeric) {
                match get_pid_info(pathbuf.as_path()) {
                    Ok(proc) => { pids.insert(proc.pid, proc); }
                    Err(e)   => { println!("Warning couldn't read {} pid file: {:?}", name, e); }
                };
            }
        }
    }

    Ok(pids)
}

fn build_trees(records: &ProcessMap) -> Vec<Process> {
    let mut tree = HashMap::<u32, Vec<&ProcessRecord>>::new();

    for record in records.values() {
        tree.entry(record.ppid)
            .or_insert_with(|| vec!())
            .push(record);
    }

    records.values()
        .filter_map(|rec| {
            if rec.ppid == 0 {
                Some(Process::new(rec, &tree))
            }
            else {
                None
            }
        })
        .collect()
}

fn print_trees(trees: &[&Process], indent: &str, mut writer: &mut std::io::Write) -> std::io::Result<()> {
    if let Some((last, rest)) = trees.split_last() {
        for proc in rest {
            writeln!(&mut writer, "{}├─ {} {}", indent, proc.pid, proc.cmdline)?;
            print_trees(
                &proc.children.iter().collect::<Vec<_>>(),
                &format!("{}│  ", indent),
                writer,
            )?;
        }
        writeln!(&mut writer, "{}└─ {} {}", indent, last.pid, last.cmdline)?;
        print_trees(
            &last.children.iter().collect::<Vec<_>>(),
            &format!("{}   ", indent),
            writer,
        )?;
    }
    Ok(())
}

#[derive(Debug)]
struct RunOpts {
    filter: Option<String>,
    uid_search: bool,
}

impl RunOpts {
    fn new(command_args: &[String]) -> RunOpts {
        let mut opts = Options::new();
        opts.optflag("a", "", "show all uids");

        let matches: Matches = match opts.parse(&command_args[1..]) {
            Ok(m)  => m,
            Err(f) => panic!(f.to_string()),
        };

        RunOpts {
            filter: match matches.free.get(0) {
                Some(f) => Some(f.clone()),
                None    => None,
            },
            uid_search: ! matches.opt_present("a"),
        }
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let opts = RunOpts::new(&args);
    let uid = get_current_uid();

    let pids = visit_pids(Path::new("/proc")).expect("Couldn't read /proc");
    let trees = build_trees(&pids);
    let mut matched = vec!();

    for tree in &trees {
        tree.search(&mut matched, &|p| {
            (!opts.uid_search || (p.uid == uid)) && match &opts.filter {
                Some(f) => p.cmdline.contains(f),
                None    => true,
            }
        });
    }

    match print_trees(&matched, &String::from(""), &mut std::io::stdout()) {
        Err(_) => {},
        Ok(()) => {},
    };
}
