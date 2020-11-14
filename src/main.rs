use std::io::{self, BufRead, Write};

fn main() -> io::Result<()> {
    let mut output = io::stdout().lock();
    for line in io::stdin().lock().split(b'\n') {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        output.write_all(&lucent::request(&line))?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}
