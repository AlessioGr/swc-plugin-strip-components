use std::path::PathBuf;

use swc_core::ecma::{transforms::testing::test_fixture, visit::as_folder};
use swc_plugin_strip_components::loadable_transform;
use swc_ecma_parser::{Syntax, TsConfig};


fn ts_syntax() -> Syntax {
    Syntax::Typescript(TsConfig {
        tsx: true,
        ..Default::default()
    })
}


#[testing::fixture("tests/fixture/**/input.ts")]
fn fixture(input: PathBuf) {
    let output = input.parent().unwrap().join("output.ts");


    test_fixture(
        ts_syntax(),
        &|t| as_folder(loadable_transform(t.comments.clone())),
        &input,
        &output,
        Default::default(),
    );
}
