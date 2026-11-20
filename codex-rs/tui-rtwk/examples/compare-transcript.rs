//! Compare two checkpoints captured by tools/rtwk-port/mock-model/check.py.

use std::path::Path;

use anyhow::{Context, Result, bail};
use rtwk_test::{Screen, row_text};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [size, oracle, rtwk] = args.as_slice() else {
        bail!("usage: compare-transcript WIDTHxHEIGHT ORACLE.ansi RTWK.ansi");
    };
    let (width, height) = size.split_once('x').context("expected WIDTHxHEIGHT")?;
    let (width, height) = (width.parse()?, height.parse()?);
    let oracle = replay(width, height, Path::new(oracle))?;
    let rtwk = replay(width, height, Path::new(rtwk))?;
    let mut different = 0;
    for row in 0..height {
        let expected = row_text(&oracle, row, true);
        let actual = row_text(&rtwk, row, true);
        if expected != actual {
            println!("{:>3} oracle  {}", row + 1, expected.trim_end());
            println!("{:>3} rtwk    {}", row + 1, actual.trim_end());
            different += 1;
        }
    }
    println!("{different}/{height} rows differ in content or style");
    Ok(())
}

fn replay(width: usize, height: usize, path: &Path) -> Result<Screen> {
    let mut screen = Screen::new(width, height);
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    screen.feed(&bytes);
    Ok(screen)
}
