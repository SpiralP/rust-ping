use std::time::Duration;

#[test]
fn basic() {
    let addr = "1.1.1.1".parse().unwrap();
    let timeout = Duration::from_secs(1);
    println!(
        "{:#?}",
        ping::ping(
            addr,
            Some(timeout),
            Some(166),
            Some(3),
            Some(5),
            Some(&[7; ping::TOKEN_SIZE]),
        )
        .unwrap()
    );
}
