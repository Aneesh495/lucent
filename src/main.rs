use std::{
    fs::File,
    io::{self, BufRead, Read, Write},
    process::ExitCode,
};

const REQUEST_LIMIT: usize = 65_536;
const CLAIM_LIMIT: usize = 16 * 1024 * 1024;

fn main() -> ExitCode {
    let result = match std::env::args_os().skip(1).collect::<Vec<_>>().as_slice() {
        [] => stream().map(|_| true),
        [flag, path] if flag == "--verify" => verify(File::open(path)),
        _ => {
            eprintln!("Usage: lucent [--verify PACKET.json]");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn verify(input: io::Result<File>) -> io::Result<bool> {
    let mut bytes = Vec::new();
    input?
        .take(CLAIM_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)?;
    let verdict = lucent::verify_packet(&bytes);
    println!("{verdict}");
    Ok(verdict["status"] == "verified")
}

fn stream() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut line = Vec::new();
    let mut too_long = false;
    loop {
        let buffer = input.fill_buf()?;
        let delimiter = buffer.iter().position(|byte| *byte == b'\n');
        let take = delimiter.unwrap_or(buffer.len());
        let done = buffer.is_empty();
        if !too_long {
            if take > REQUEST_LIMIT - line.len() {
                too_long = true;
            } else {
                line.extend_from_slice(&buffer[..take]);
            }
        }
        input.consume(take + usize::from(delimiter.is_some()));
        if done || delimiter.is_some() {
            if too_long {
                output.write_all(
                    b"{\"status\":\"error\",\"message\":\"Request exceeds 64 KiB\"}\n",
                )?;
            } else if !line.is_empty() {
                output.write_all(&lucent::request(&line))?;
                output.write_all(b"\n")?;
            }
            output.flush()?;
            line.clear();
            too_long = false;
        }
        if done {
            break;
        }
    }
    Ok(())
}
