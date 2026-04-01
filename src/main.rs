use parser_playground::parse_map;

fn main() -> miette::Result<()> {
    let arg = std::env::args()
        .skip(1)
        .reduce(|mut acc, s| {
            acc.push(' ');
            acc.push_str(&s);
            acc
        })
        .unwrap();

    let map = parse_map(arg)?;
    dbg!(map);

    Ok(())
}
