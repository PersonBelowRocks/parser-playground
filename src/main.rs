use std::str::FromStr;

use parser_playground::Key;

fn main() -> miette::Result<()> {
    let arg = std::env::args().nth(1).unwrap();

    let key = Key::from_str(&arg)?;
    dbg!(key);

    Ok(())
}
