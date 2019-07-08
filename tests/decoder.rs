use insta::assert_debug_snapshot_matches;
use log::{debug, info};

const WEBM_FILE_LIST: &'static [&'static str] = &[
    "./matroska-test-files/test_files/test1.mkv",
    "./matroska-test-files/test_files/test2.mkv",
    "./matroska-test-files/test_files/test3.mkv",
    // "./matroska-test-files/test_files/test4.mkv", // this file is broken so not pass encoder_decoder_test
    "./matroska-test-files/test_files/test5.mkv",
    "./matroska-test-files/test_files/test6.mkv",
    // "./matroska-test-files/test_files/test7.mkv", // this file has unknown tag so cannot write file
    "./matroska-test-files/test_files/test8.mkv",
];

#[test]
fn test_decoder() {
    dotenv::dotenv().ok();
    env_logger::try_init().ok();
    for path in WEBM_FILE_LIST {
        info!("start: {}", path);
        let mut decoder = ebml::Decoder::default();
        let mut mkv = std::fs::File::open(path).unwrap();
        let mut buffer = vec![];
        use std::io::Read;
        mkv.read_to_end(&mut buffer).unwrap();
        let elms = decoder.decode(buffer).unwrap();
        assert_debug_snapshot_matches!(
            format!(
                "{}.snapshot",
                std::path::Path::new(path)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
            ),
            elms
        );
        debug!("{:?}", elms);
        // assert_eq!(elms.len(), 2766);
    }
}
