#[test]
fn test_encoder() {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "trace");
    env_logger::try_init().ok();

    let schema = ebml::schema::DefaultSchema::default();
    let mut encoder = ebml::Encoder::new(&schema);
    let mut decoder = ebml::Decoder::new(&schema);
    let elms: Vec<ebml::ebml::Element> = vec![ebml::ebml::Utf8Element {
        ebml_id: 0.into(),
        value: "a".to_string(),
    }
    .into()];
    let buf = encoder.encode(elms.clone()).unwrap();
    let elms2 = decoder.decode(buf).unwrap();
    assert_eq!(
        elms,
        elms2
            .into_iter()
            .map(Into::into)
            .collect::<Vec<ebml::ebml::Element>>()
    );
}
