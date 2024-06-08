#[macro_export]
macro_rules! make_config {
    ($($id:literal : { $($resource:ident : $amt:literal),* }),+) => {
        std::collections::HashMap::from([
            $((
                $id,
                (
                    format!("127.0.0.1:{}", 8000 + $id).parse().unwrap(),
                    std::collections::HashMap::from([
                        $((
                            stringify!($resource).to_owned(),
                            $amt
                        )),*
                    ])
                )
            )),+
        ])
    };
}
