//! Source inventory CLI; only a zero exit status qualifies complete output.
use std::{env, io, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = env::args()
        .nth(1)
        .ok_or("usage: matea-api-audit SOURCE_ROOT")?;
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    matea_api_audit::audit(&mut out, Path::new(&root))
}
