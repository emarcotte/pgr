use getopts::{Fail, Options,};
use std::collections::HashMap;
use std::fs::{File, read_dir, DirEntry, };
use std::io::{BufRead, BufReader, };
use std::path::Path;
use users::{get_current_uid};
use unicode_width::UnicodeWidthStr;
use terminal_size::{Width, terminal_size};

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

fn print_child(child: &Process, width: usize, indent: &str, turn: &str, indent_bar: &str, mut writer: &mut std::io::Write) -> std::io::Result<()> {
    let digits = (child.pid as f32).log10().floor() as usize;
    let split_cmd = wrap_cmdline(&child.cmdline, width - digits - 1);
    let has_children = !child.children.is_empty();
    if let Some((head, tail)) = split_cmd.split_first() {
        writeln!(&mut writer, "{}{} {} {}", indent, turn, child.pid, head)?;
        if !tail.is_empty() {
            let wrap_indent = format!("   {}{:2$}", if has_children { "│" } else { " " }, "", digits);
            for tokens in tail {
                writeln!(&mut writer, "{}{}  {}", indent, wrap_indent, tokens)?;
            }
        }
    }

    print_trees(
        &child.children.iter().collect::<Vec<_>>(),
        width - 3,
        &format!("{}{}  ", indent, indent_bar),
        writer,
    )?;
    Ok(())
}

fn print_trees(trees: &[&Process], width: usize, indent: &str, writer: &mut std::io::Write) -> std::io::Result<()> {
    if let Some((last, rest)) = trees.split_last() {
        for proc in rest {
            print_child(&proc, width, indent, "├─", "│" , writer)?;
        }
        print_child(&last, width, indent, "└─", " ", writer)?;
    }
    Ok(())
}

#[derive(Debug)]
struct RunOpts {
    filter: Option<String>,
    uid_search: bool,
}

impl RunOpts {
    fn new(command_args: &[String]) -> Result<RunOpts, Fail> {
        let mut opts = Options::new();
        opts.optflag("a", "", "show all uids");

        let matches = opts.parse(&command_args[1..])?;

        Ok(
            RunOpts {
                filter: match matches.free.get(0) {
                    Some(f) => Some(f.clone()),
                    None    => None,
                },
                uid_search: ! matches.opt_present("a"),
            }
        )
    }
}

fn wrap_cmdline(line: &str, width: usize) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let tokens = line.split_whitespace();
    let mut cur_line_used = 0;

    for token in tokens {
        let token_width = UnicodeWidthStr::width(token);
        if cur_line_used + token_width < width {
            if let Some(curr_line) = result.last_mut() {
                curr_line.push_str(token);
                curr_line.push_str(" ");
                cur_line_used += token_width;
            }
            else {
                result.push(String::new());
                if let Some(curr_line) = result.last_mut() {
                    curr_line.push_str(token);
                    curr_line.push_str(" ");
                    cur_line_used = token_width + 1;
                }
            }
        }
        else {
            result.push(String::new());
            if let Some(curr_line) = result.last_mut() {
                curr_line.push_str(token);
                curr_line.push_str(" ");
                cur_line_used = token_width + 1;
            }
        }
    }

    result
}

#[test]
fn test_wrap_cmdline() {
    assert_eq!(wrap_cmdline("hello", 2), vec!("hello "));
    assert_eq!(wrap_cmdline("hello --world", 20), vec!("hello --world "));
    assert_eq!(wrap_cmdline("hello --world", 7), vec!("hello ", "--world "));
    assert_eq!(wrap_cmdline("hello --world-war", 6), vec!("hello ", "--world-war "));
    assert_eq!(wrap_cmdline("hello --word z", 9), vec!("hello ", "--word z "));
    assert_eq!(
        wrap_cmdline("hello z --word z superdyduperdydo", 9),
        vec!("hello z ", "--word z ", "superdyduperdydo ")
    );
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let opts = RunOpts::new(&args).expect("Couldn't parse command line flags");

    let pids = visit_pids(Path::new("/proc")).expect("Couldn't read /proc");
    let trees = build_trees(&pids);

    let mut matched = vec!();

    let uid = get_current_uid();

    let width = match terminal_size() {
        Some((Width(w), _)) => w as usize,
        None => 80usize,
    };

    for tree in &trees {
        tree.search(&mut matched, &|p| {
            (!opts.uid_search || (p.uid == uid)) && match &opts.filter {
                Some(f) => p.cmdline.contains(f),
                None    => true,
            }
        });
    }

    match print_trees(&matched, width - 3, &String::from(""), &mut std::io::stdout()) {
        Err(_) => {},
        Ok(()) => {},
    };
}
